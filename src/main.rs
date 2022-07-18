#![no_std]
#![no_main]

mod demux_matrix;
mod encoder;
mod layout;
mod types;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[rtic::app(device = rp2040_hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0, PIO0_IRQ_1, PIO1_IRQ_0])]
mod app {
    use cortex_m::prelude::{
        _embedded_hal_watchdog_Watchdog, _embedded_hal_watchdog_WatchdogEnable,
    };
    use defmt_rtt as _;
    use embedded_time::duration::Extensions;
    use embedded_time::rate::Extensions as RateExtensions;
    use panic_probe as _;
    use rp2040_hal;
    use rp2040_hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::dynpin::DynPin,
        pac::I2C0,
        pio::PIOExt,
        sio::Sio,
        timer::{Alarm, Alarm0, Timer},
        usb::UsbBus,
        watchdog::Watchdog,
    };

    use core::iter::once;

    use crate::demux_matrix::DemuxMatrix;
    use crate::encoder::Encoder;
    use crate::layout as kb_layout;
    use crate::types::active;
    use display_interface_i2c::I2CInterface;
    use embedded_graphics::{
        image::{Image, ImageRaw},
        mono_font::{ascii::FONT_7X14_BOLD, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        prelude::*,
        text::{Baseline, Text},
    };
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::{CustomEvent, Event, Layout};
    use ssd1306::mode::BufferedGraphicsMode;
    use ssd1306::{prelude::*, Ssd1306};

    use smart_leds::{brightness, SmartLedsWrite, RGB8};
    use usb_device::class::UsbClass;
    use usb_device::class_prelude::UsbBusAllocator;
    use usb_device::device::UsbDeviceState;
    use ws2812_pio::Ws2812Direct;

    const SCAN_TIME_US: u32 = 1000;
    const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;
    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

    pub struct Leds {
        caps_lock: active::OnBoardLED,
    }

    impl keyberon::keyboard::Leds for Leds {
        fn caps_lock(&mut self, status: bool) {
            if status {
                let mut onboard_data: [RGB8; 1] = [RGB8::default(); 1];
                onboard_data[0] = RGB8 {
                    r: 0x40,
                    g: 0x00,
                    b: 0x00,
                };
                self.caps_lock
                    .write(brightness(once(onboard_data[0]), 32))
                    .unwrap();
            } else {
                let mut onboard_data: [RGB8; 1] = [RGB8::default(); 1];
                onboard_data[0] = RGB8 {
                    r: 0x00,
                    g: 0x40,
                    b: 0x00,
                };
                self.caps_lock
                    .write(brightness(once(onboard_data[0]), 32))
                    .unwrap();
            }
        }
    }

    type DisplayI2C = I2CInterface<rp2040_hal::I2C<I2C0, (active::Sda, active::Scl)>>;

    #[shared]
    struct Shared {
        #[lock_free]
        underglow: active::Underglow,
        underglow_state: bool,
        encoder: Encoder<active::EncoderPadA, active::EncoderPadB>,
        #[lock_free]
        display: Ssd1306<DisplayI2C, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>,
        display_state: bool,
        usb_dev: usb_device::device::UsbDevice<'static, UsbBus>,
        usb_class: keyberon::Class<'static, UsbBus, Leds>,
        timer: Timer,
        alarm: Alarm0,
        #[lock_free]
        matrix: DemuxMatrix<DynPin, DynPin, 16, 5>,
        layout: Layout<16, 5, 1, kb_layout::CustomActions>,
        #[lock_free]
        debouncer: Debouncer<[[bool; 16]; 5]>,
        #[lock_free]
        watchdog: Watchdog,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut resets = c.device.RESETS;
        let mut watchdog = Watchdog::new(c.device.WATCHDOG);
        watchdog.pause_on_debug(false);

        let clocks = init_clocks_and_plls(
            EXTERNAL_XTAL_FREQ_HZ,
            c.device.XOSC,
            c.device.CLOCKS,
            c.device.PLL_SYS,
            c.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let sio = Sio::new(c.device.SIO);
        let pins = rp2040_hal::gpio::Pins::new(
            c.device.IO_BANK0,
            c.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        let mut timer = Timer::new(c.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US.microseconds());
        alarm.enable_interrupt();

        let (mut pio, sm0, sm1, _, _) = c.device.PIO0.split(&mut resets);

        // onboard led is gpio 25 for pro micro
        let onboard = Ws2812Direct::new(
            #[cfg(feature = "kb2040")]
            pins.gpio17.into_mode(),
            #[cfg(feature = "rp2040-pro-micro")]
            pins.gpio25.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );
        let leds = Leds { caps_lock: onboard };

        // underglow is gpio 7 for pro micro
        let underglow = Ws2812Direct::new(
            pins.gpio7.into_mode(),
            &mut pio,
            sm1,
            clocks.peripheral_clock.freq(),
        );
        let underglow_state: bool = false;

        // also 8 and 9 for pro micro
        let encoder_a = pins.gpio8.into_pull_up_input();
        let encoder_b = pins.gpio9.into_pull_up_input();

        let encoder = Encoder::new(
            encoder_a,
            encoder_b,
            kb_layout::ENCODER_LEFT,
            kb_layout::ENCODER_RIGHT,
        );

        // pro micro scl = 17 sda = 16
        #[cfg(feature = "kb2040")]
        let sda_pin = pins.gpio12.into_mode::<rp2040_hal::gpio::FunctionI2C>();
        #[cfg(feature = "kb2040")]
        let scl_pin = pins.gpio13.into_mode::<rp2040_hal::gpio::FunctionI2C>();

        #[cfg(feature = "rp2040-pro-micro")]
        let sda_pin = pins.gpio16.into_mode::<rp2040_hal::gpio::FunctionI2C>();
        #[cfg(feature = "rp2040-pro-micro")]
        let scl_pin = pins.gpio17.into_mode::<rp2040_hal::gpio::FunctionI2C>();

        let i2c = rp2040_hal::I2C::i2c0(
            c.device.I2C0,
            sda_pin,
            scl_pin,
            400_u32.kHz(),
            &mut resets,
            clocks.peripheral_clock,
        );

        let interface = ssd1306::I2CDisplayInterface::new(i2c);

        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();
        display.clear();
        display.flush().unwrap();

        let display_state = false;

        let usb_bus = UsbBusAllocator::new(UsbBus::new(
            c.device.USBCTRL_REGS,
            c.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        ));

        unsafe {
            USB_BUS = Some(usb_bus);
        }

        let usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, leds);
        let usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

        watchdog.start(10_000.microseconds());

        #[cfg(feature = "rp2040-pro-micro")]
        let matrix = DemuxMatrix::new(
            [
                pins.gpio29.into_push_pull_output().into(),
                pins.gpio28.into_push_pull_output().into(),
                pins.gpio27.into_push_pull_output().into(),
                pins.gpio26.into_push_pull_output().into(),
            ],
            [
                pins.gpio22.into_pull_up_input().into(),
                pins.gpio20.into_pull_up_input().into(),
                pins.gpio23.into_pull_up_input().into(),
                pins.gpio21.into_pull_up_input().into(),
                pins.gpio4.into_pull_up_input().into(),
            ],
            16,
        );

        #[cfg(feature = "bit-c-rp2040")]
        let matrix = DemuxMatrix::new(
            [
                pins.gpio29.into_push_pull_output().into(),
                pins.gpio28.into_push_pull_output().into(),
                pins.gpio27.into_push_pull_output().into(),
                pins.gpio26.into_push_pull_output().into(),
            ],
            [
                pins.gpio18.into_pull_up_input().into(),
                pins.gpio20.into_pull_up_input().into(),
                pins.gpio19.into_pull_up_input().into(),
                pins.gpio10.into_pull_up_input().into(),
                pins.gpio4.into_pull_up_input().into(),
            ],
            16,
        );

        #[cfg(feature = "kb2040")]
        let matrix = DemuxMatrix::new(
            [
                pins.gpio29.into_push_pull_output().into(),
                pins.gpio28.into_push_pull_output().into(),
                pins.gpio27.into_push_pull_output().into(),
                pins.gpio26.into_push_pull_output().into(),
            ],
            [
                pins.gpio18.into_pull_up_input().into(),
                pins.gpio20.into_pull_up_input().into(),
                pins.gpio19.into_pull_up_input().into(),
                pins.gpio10.into_pull_up_input().into(),
                pins.gpio4.into_pull_up_input().into(),
            ],
            16,
        );

        (
            Shared {
                underglow,
                underglow_state,
                encoder: encoder.unwrap(),
                display,
                display_state,
                usb_dev,
                usb_class,
                timer,
                alarm,
                matrix: matrix.unwrap(),
                debouncer: Debouncer::new([[false; 16]; 5], [[false; 16]; 5], 10),
                layout: Layout::new(&kb_layout::LAYERS),
                watchdog,
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[task(binds = USBCTRL_IRQ, priority = 4, shared = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        (usb, kb).lock(|usb, kb| {
            if usb.poll(&mut [kb]) {
                kb.poll();
            }
        });
    }

    #[task(priority = 3, shared = [underglow, underglow_state])]
    fn handle_underglow(mut c: handle_underglow::Context) {
        let underglow = c.shared.underglow;
        c.shared.underglow_state.lock(|us| {
            if *us {
                let off: [RGB8; 10] = [RGB8::default(); 10];
                underglow.write(off.iter().cloned()).unwrap();
                *us = false;
            } else {
                let mut under_data: [RGB8; 10] = [RGB8::default(); 10];
                for data in &mut under_data {
                    *data = RGB8 {
                        r: 0xFF,
                        g: 0x00,
                        b: 0xFF,
                    };
                }
                underglow.write(under_data.iter().cloned()).unwrap();
                *us = true;
            }
        });
    }

    #[task(priority = 3, shared = [display, display_state])]
    fn handle_display(c: handle_display::Context) {
        let display = c.shared.display;
        let mut display_state = c.shared.display_state;

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_7X14_BOLD)
            .text_color(BinaryColor::On)
            .build();

        display_state.lock(|ds| {
            if *ds {
                display.clear();
                display.flush().unwrap();
                *ds = false;
            } else {
                display.clear();

                let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("./rust.raw"), 32);

                let im = Image::new(&raw, Point::new(96, 0));

                im.draw(display).unwrap();

                Text::with_baseline("Powered by", Point::new(16, 8), text_style, Baseline::Top)
                    .draw(display)
                    .unwrap();

                display.flush().unwrap();
                *ds = true;
            }
        });
    }

    #[task(priority = 2, capacity = 8, shared = [usb_dev, usb_class, layout])]
    fn handle_event(mut c: handle_event::Context, event: Option<Event>) {
        let mut layout = c.shared.layout;
        match event {
            None => {
                if let CustomEvent::Press(event) = layout.lock(|l| l.tick()) {
                    match event {
                        kb_layout::CustomActions::Underglow => {
                            handle_underglow::spawn().unwrap();
                        }
                        kb_layout::CustomActions::Bootloader => {
                            rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
                        }
                        kb_layout::CustomActions::Display => {
                            handle_display::spawn().unwrap();
                        }
                    };
                }
            }
            Some(e) => {
                layout.lock(|l| l.event(e));
                return;
            }
        }

        let report: key_code::KbHidReport = layout.lock(|l| l.keycodes().collect());
        if !c
            .shared
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        if c.shared.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }
        while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(binds = TIMER_IRQ_0, priority = 1, shared = [encoder, matrix, debouncer, timer, alarm, watchdog, usb_dev, usb_class])]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let mut alarm = c.shared.alarm;

        alarm.lock(|a| {
            a.clear_interrupt();
            let _ = a.schedule(SCAN_TIME_US.microseconds());
        });

        c.shared.watchdog.feed();

        for event in c.shared.debouncer.events(c.shared.matrix.get().unwrap()) {
            handle_event::spawn(Some(event)).unwrap();
        }

        c.shared.encoder.lock(|e| {
            if let Ok(Some(events)) = e.read_events() {
                for event in events {
                    handle_event::spawn(Some(event)).unwrap();
                }
            }
        });

        handle_event::spawn(None).unwrap();
    }
}
