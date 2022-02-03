#![no_std]
#![no_main]

mod demux_matrix;
mod encoder;
mod layout;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[rtic::app(device = rp2040_hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0])]
mod app {
    use cortex_m::prelude::{
        _embedded_hal_watchdog_Watchdog, _embedded_hal_watchdog_WatchdogEnable,
    };
    use defmt_rtt as _;
    use embedded_time::duration::Extensions;
    use panic_probe as _;
    use rp2040_hal;
    use rp2040_hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{bank0::*, dynpin::DynPin},
        pac::PIO0,
        pio::{PIOExt, SM0, SM1},
        sio::Sio,
        timer::{Alarm0, Timer},
        usb::UsbBus,
        watchdog::Watchdog,
    };

    use core::iter::once;

    use crate::demux_matrix::DemuxMatrix;
    use crate::encoder::Encoder;
    use crate::layout as kb_layout;
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::{CustomEvent, Event, Layout};
    use keyberon::matrix::PressedKeys;

    use smart_leds::{brightness, SmartLedsWrite, RGB8};
    use usb_device::class::UsbClass;
    use usb_device::class_prelude::UsbBusAllocator;
    use ws2812_pio::Ws2812Direct as Ws2812Pio;

    const SCAN_TIME_US: u32 = 1000;
    const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;
    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

    pub struct Leds {
        caps_lock: Ws2812Pio<PIO0, SM0, Gpio17>,
    }

    impl keyberon::keyboard::Leds for Leds {
        fn caps_lock(&mut self, status: bool) {
            if status {
                let mut onboard_data: [RGB8; 1] = [RGB8::default(); 1];
                onboard_data[0] = RGB8 {
                    r: 0xFF,
                    g: 0x00,
                    b: 0x00,
                };
                self.caps_lock
                    .write(brightness(once(onboard_data[0]), 32))
                    .unwrap();
            } else {
                let onboard_data: [RGB8; 1] = [RGB8::default(); 1];
                self.caps_lock
                    .write(brightness(once(onboard_data[0]), 32))
                    .unwrap();
            }
        }
    }

    #[shared]
    struct Shared {
        #[lock_free]
        underglow: Ws2812Pio<PIO0, SM1, Gpio7>,
        underglow_state: bool,
        encoder: Encoder,
        usb_dev: usb_device::device::UsbDevice<'static, UsbBus>,
        usb_class: keyberon::Class<'static, UsbBus, Leds>,
        timer: Timer,
        alarm: Alarm0,
        #[lock_free]
        matrix: DemuxMatrix<DynPin, DynPin, 16, 5>,
        layout: Layout<kb_layout::CustomActions>,
        #[lock_free]
        debouncer: Debouncer<PressedKeys<16, 5>>,
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

        let underglow_state: bool = false;

        let encoder_a = pins.gpio8.into_pull_up_input();
        let encoder_b = pins.gpio9.into_pull_up_input();
        let encoder = Encoder::new(encoder_a, encoder_b, (3, 14), (4, 14));

        let mut timer = Timer::new(c.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US.microseconds());
        alarm.enable_interrupt(&mut timer);

        let (mut pio, sm0, sm1, _, _) = c.device.PIO0.split(&mut resets);

        let onboard = Ws2812Pio::new(
            pins.gpio17.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );

        let underglow = Ws2812Pio::new(
            pins.gpio7.into_mode(),
            &mut pio,
            sm1,
            clocks.peripheral_clock.freq(),
        );

        let leds = Leds { caps_lock: onboard };

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
                underglow: underglow,
                underglow_state: underglow_state,
                encoder: encoder,
                usb_dev: usb_dev,
                usb_class: usb_class,
                timer: timer,
                alarm: alarm,
                matrix: matrix.unwrap(),
                debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 10),
                layout: Layout::new(kb_layout::LAYERS),
                watchdog: watchdog,
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[task(binds = USBCTRL_IRQ, priority = 2, shared = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        (usb, kb).lock(|usb, kb| {
            if usb.poll(&mut [kb]) {
                kb.poll();
            }
        });
    }

    #[task(binds = TIMER_IRQ_0, priority = 1, shared = [encoder, underglow, underglow_state, matrix, debouncer, timer, alarm, layout, watchdog, usb_dev, usb_class])]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let timer = c.shared.timer;
        let alarm = c.shared.alarm;
        let mut layout = c.shared.layout;
        let mut usb_class = c.shared.usb_class;
        let underglow = c.shared.underglow;

        (timer, alarm).lock(|t, a| {
            a.clear_interrupt(t);
            let _ = a.schedule(SCAN_TIME_US.microseconds());
        });

        c.shared.watchdog.feed();

        for event in c.shared.debouncer.events(c.shared.matrix.get().unwrap()) {
            layout.lock(|l| l.event(event));
        }

        match layout.lock(|l| l.tick()) {
            CustomEvent::Press(e) => match e {
                kb_layout::CustomActions::Underglow => {
                    c.shared.underglow_state.lock(|us| {
                        if *us {
                            let off: [RGB8; 10] = [RGB8::default(); 10];
                            underglow.write(off.iter().cloned()).unwrap();
                            *us = false;
                        } else {
                            let mut under_data: [RGB8; 10] = [RGB8::default(); 10];
                            for i in 0..10 {
                                under_data[i] = RGB8 {
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
                kb_layout::CustomActions::Bootloader => {
                    rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
                }
            },
            //_ => (),
            _ => {
                c.shared.encoder.lock(|e| {
                    for event in e.read_events().unwrap() {
                        layout.lock(|l| l.event(event));
                    }
                    //let val = e.read();
                    //if val == -1 {
                    //    layout.lock(|l| l.event(Event::Press(3, 14)));
                    //    layout.lock(|l| l.event(Event::Release(3, 14)));
                    //} else if val == 1 {
                    //    layout.lock(|l| l.event(Event::Press(4, 14)));
                    //    layout.lock(|l| l.event(Event::Release(4, 14)));
                    //}
                });
            }
        }

        let report: key_code::KbHidReport = layout.lock(|l| l.keycodes().collect());
        if usb_class.lock(|k| k.device_mut().set_keyboard_report(report.clone())) {
            while let Ok(0) = usb_class.lock(|k| k.write(report.as_bytes())) {}
        }
    }
}
