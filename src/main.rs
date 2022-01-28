#![no_std]
#![no_main]
#![allow(unused_imports)]
#![allow(unused_mut)]

mod encoder;
mod demux_matrix;
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
    use embedded_hal::digital::v2::{InputPin, OutputPin};
    use embedded_hal::prelude::_embedded_hal_serial_Write;
    use embedded_hal::spi::MODE_0;
    use embedded_time::duration::Extensions;
    use embedded_time::rate::Extensions as rate_extensions;
    use panic_probe as _;
    use rp2040_hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{bank0::*, dynpin::DynPin, FunctionSpi, Pin, PushPullOutput, PullUpInput},
        pac::{UART0, UART1, SPI0, PIO0},
        pio::{PIOExt, SM0},
        sio::Sio,
        spi::{Enabled, Spi},
        timer::{Alarm0, CountDown, Timer},
        usb::UsbBus,
        watchdog::Watchdog,
    };

    use core::fmt::Write;
    use core::iter::once;

    use crate::encoder::Encoder;
    use crate::demux_matrix::DemuxMatrix;
    use crate::layout as kb_layout;
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::{CustomEvent, Event, Layout};
    use keyberon::matrix::PressedKeys;

    use smart_leds::{brightness, SmartLedsWrite, RGB8};
    use usb_device::class::UsbClass;
    use usb_device::class_prelude::UsbBusAllocator;
    use ws2812_pio::Ws2812Direct as Ws2812Pio;
    use ws2812_spi::Ws2812 as Ws2812Spi;

    const SCAN_TIME_US: u32 = 1000;
    const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;
    const SYS_HZ: u32 = 125_000_000_u32;
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
                self.caps_lock.write(brightness(once(onboard_data[0]), 32)).unwrap();
            } else {
                let mut onboard_data: [RGB8; 1] = [RGB8::default(); 1];
                self.caps_lock.write(brightness(once(onboard_data[0]), 32)).unwrap();
            }
        }
        fn compose(&mut self, status: bool) {
            if status {
                let mut onboard_data: [RGB8; 1] = [RGB8::default(); 1];
                onboard_data[0] = RGB8 {
                    r: 0xFF,
                    g: 0x00,
                    b: 0xFF,
                };
                self.caps_lock.write(brightness(once(onboard_data[0]), 32)).unwrap();
            } else {
                let mut onboard_data: [RGB8; 1] = [RGB8::default(); 1];
                self.caps_lock.write(brightness(once(onboard_data[0]), 32)).unwrap();
            }
        }
    }

    #[shared]
    struct Shared {
        #[lock_free]
        underglow: Ws2812Spi<Spi<Enabled, SPI0, 8>>, 
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
    fn init(mut c: init::Context) -> (Shared, Local, init::Monotonics) {
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

        //for _ in 0..1000 {
        //    cortex_m::asm::nop();
        //}

        //let uart_pins = (
        //    pins.gpio0.into_mode::<rp2040_hal::gpio::FunctionUart>(),
        //    pins.gpio1.into_mode::<rp2040_hal::gpio::FunctionUart>(),
        //);

        //let mut uart0 = rp2040_hal::uart::UartPeripheral::<_, _>::new(c.device.UART0, &mut resets)
        //    .enable(
        //        rp2040_hal::uart::common_configs::_9600_8_N_1,
        //        clocks.peripheral_clock.freq().into(),
        //    )
        //    .unwrap();

        let _spi_sclk = pins.gpio3.into_mode::<FunctionSpi>();
        let _spi_mosi = pins.gpio7.into_mode::<FunctionSpi>();
        let spi = Spi::<_, _, 8>::new(c.device.SPI0).init(
            &mut resets,
            SYS_HZ.Hz(),
            3_000_000u32.Hz(),
            &MODE_0,
        );

        let mut underglow = Ws2812Spi::new(spi);
        let mut underglow_state: bool = false;

        let mut encoder_a = pins.gpio8.into_pull_up_input();
        let mut encoder_b = pins.gpio9.into_pull_up_input();
        let mut encoder = Encoder::new(encoder_a, encoder_b);

        let mut timer = Timer::new(c.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US.microseconds());
        alarm.enable_interrupt(&mut timer);

        let (mut pio, sm0, _, _, _) = c.device.PIO0.split(&mut resets);

        let mut onboard = Ws2812Pio::new(
            pins.gpio17.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );

        let mut leds = Leds { caps_lock: onboard };

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
    fn usb_rx(mut c: usb_rx::Context) {
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
        let mut underglow = c.shared.underglow;

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
                    // /*c.shared.underglow,*/ c.shared.underglow_state.lock(|/*u, */us| {
                    c.shared.underglow_state.lock(|us| {
                        if *us {
                            let mut off: [RGB8; 10] = [RGB8::default(); 10];
                            //u.write(off.iter().cloned()).unwrap();
                            underglow.write(off.iter().cloned()).unwrap();
                            *us = false;
                        } else {
                            let mut under_data: [RGB8; 10] = [RGB8::default(); 10];
                            for i in 0..10 {
                                under_data[i] = RGB8 { r: 0xFF, g: 0x00, b: 0xFF};
                            }
                            underglow.write(under_data.iter().cloned()).unwrap();
                            *us = true;
                        }
                    });
                },
            },
            //_ => (),
            _ => {
                c.shared.encoder.lock(|e| {
                    let val = e.read();
                    if val == -1 {
                        layout.lock(|l| l.event(Event::Press(3, 14)));
                        layout.lock(|l| l.event(Event::Release(3, 14)));
                    } else if val == 1 {
                        layout.lock(|l| l.event(Event::Press(4, 14)));
                        layout.lock(|l| l.event(Event::Release(4, 14)));
                    }
                });
            },
        }

        let report: key_code::KbHidReport = layout.lock(|l| l.keycodes().collect());
        if usb_class.lock(|k| k.device_mut().set_keyboard_report(report.clone())) {
            while let Ok(0) = usb_class.lock(|k| k.write(report.as_bytes())) {}
        }
    }
}
