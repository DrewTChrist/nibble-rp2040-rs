#![no_std]
#![no_main]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_variables)]
mod layout;

#[rtic::app(device = adafruit_kb2040::hal::pac, peripherals = true)]
mod app {
    use adafruit_kb2040 as bsp;
    use cortex_m_rt::entry;
    use defmt::*;
    use defmt_rtt as _;
    use embedded_hal::digital::v2::OutputPin;
    use embedded_time::duration::Extensions;
    use embedded_time::fixed_point::FixedPoint;
    use keyberon::action::Action::{self, *};
    use keyberon::action::{k, l, m, HoldTapConfig};
    use keyberon::debounce::Debouncer;
    use keyberon::key_code::KeyCode::*;
    use keyberon::key_code::{KbHidReport, KeyCode};
    use keyberon::layout::Layout;
    use keyberon::matrix::{Matrix, PressedKeys};
    use panic_probe as _;
    use rtic::{app, Mutex};
    use usb_device::class::UsbClass;
    use usb_device::class_prelude::UsbBusAllocator;

    use super::layout;

    use bsp::hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{bank0::*, dynpin::DynPin, Pin, PullUp, PushPullOutput},
        pac,
        sio::Sio,
        timer::{Alarm0, Timer},
        usb::UsbBus,
        watchdog::Watchdog,
    };

    const SCAN_TIME_US: u32 = 1000000;

    pub struct Leds {
        caps_lock: Pin<Gpio17, PushPullOutput>,
    }

    impl keyberon::keyboard::Leds for Leds {
        fn caps_lock(&mut self, status: bool) {}
    }

    #[shared]
    struct Shared {
        usb_dev: usb_device::device::UsbDevice<'static, UsbBus>,
        usb_class: keyberon::Class<'static, UsbBus, Leds>,
        matrix: Matrix<DynPin, DynPin, 4, 5>,
        debouncer: Debouncer<PressedKeys<4, 5>>,
        layout: Layout,
        timer: Timer,
        alarm: Alarm0,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(mut c: init::Context) -> (Shared, Local, init::Monotonics) {
        unsafe {
            static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;
            let mut resets = c.device.RESETS;
            let mut pac = pac::Peripherals::take().unwrap();
            let core = pac::CorePeripherals::take().unwrap();
            let mut watchdog = Watchdog::new(pac.WATCHDOG);
            let sio = Sio::new(pac.SIO);

            let external_xtal_freq_hz = 12_000_000u32;

            let clocks = init_clocks_and_plls(
                external_xtal_freq_hz,
                pac.XOSC,
                pac.CLOCKS,
                pac.PLL_SYS,
                pac.PLL_USB,
                &mut pac.RESETS,
                &mut watchdog,
            )
            .ok()
            .unwrap();

            /*let usb_bus = UsbBus::new(
                pac.USBCTRL_REGS,
                pac.USBCTRL_DPRAM,
                clocks.usb_clock,
                true,
                &mut pac.RESETS,
            );

            let usb_bus_allocator: UsbBusAllocator<UsbBus> = UsbBusAllocator::new(usb_bus);*/

            USB_BUS = Some(UsbBusAllocator::new(UsbBus::new(
                pac.USBCTRL_REGS,
                pac.USBCTRL_DPRAM,
                clocks.usb_clock,
                true,
                &mut pac.RESETS,
            )));

            //let usb_bus = USB_BUS.as_ref().unwrap();
            let usb_bus = USB_BUS.as_ref().unwrap();

            let mut _delay =
                cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

            let pins = bsp::Pins::new(
                pac.IO_BANK0,
                pac.PADS_BANK0,
                sio.gpio_bank0,
                &mut pac.RESETS,
            );

            let mut led = pins.neopixel.into_push_pull_output();
            let leds = Leds { caps_lock: led };
            let usb_class = keyberon::new_class(&usb_bus, leds);
            let usb_dev = keyberon::new_device(&usb_bus);

            let mut timer = Timer::new(c.device.TIMER, &mut resets);

            let mut alarm = timer.alarm_0().unwrap();
            let _ = alarm.schedule(SCAN_TIME_US.microseconds());
            alarm.enable_interrupt(&mut timer);

            let matrix = Matrix::new(
                [
                    pins.a0.into_push_pull_output().into(),
                    pins.a1.into_push_pull_output().into(),
                    pins.a2.into_push_pull_output().into(),
                    pins.a3.into_push_pull_output().into(),
                ],
                [
                    pins.sclk.into_push_pull_output().into(),
                    pins.miso.into_push_pull_output().into(),
                    pins.mosi.into_push_pull_output().into(),
                    pins.d10.into_push_pull_output().into(),
                    pins.d4.into_push_pull_output().into(),
                ],
            );

            (
                Shared {
                    usb_dev: usb_dev,
                    usb_class: usb_class,
                    matrix: matrix.unwrap(),
                    debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
                    layout: Layout::new(layout::LAYERS),
                    timer: timer,
                    alarm: alarm,
                },
                Local {},
                init::Monotonics(),
            )
        }
    }
    //#[task(binds = USB_LP_CAN_TX0, priority = 2, shared = [usb_dev, usb_class])]
    //#[task(binds = TIMER_IRQ_0, priority = 2, shared = [usb_dev, usb_class])]
    #[task(binds = USBCTRL_IRQ, priority = 2, shared = [usb_dev, usb_class])]
    fn usb_tx(mut c: usb_tx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        //usb_poll(&mut c.shared.usb_dev, &mut c.shared.usb_class);
        (usb, kb).lock(|usb, kb| {
            usb_poll(usb, kb);
        });
    }

    // The keyberon ortho60 example has usb_rx and usb_tx functions
    // I'm thinking maybe this only needs one functions since ther is only
    // one usb enum in rp2040_pac
    /*#[task(binds = USBCTRL_IRQ, priority = 2, shared = [usb_dev, usb_class])]
    fn usb_rx(mut c: usb_rx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        //usb_poll(&mut c.shared.usb_dev, &mut c.shared.usb_class);
        (usb, kb).lock(|usb, kb| {
            usb_poll(usb, kb);
        });
    }*/

    #[task(binds = TIMER_IRQ_0, priority = 1, shared = [usb_class, matrix, debouncer, layout, timer, alarm])]
    fn tick(mut c: tick::Context) {
        let mut usb_class = c.shared.usb_class;
        let debouncer = c.shared.debouncer;
        let matrix = c.shared.matrix;
        let layout = c.shared.layout;
        let timer = c.shared.timer;
        let alarm = c.shared.alarm;

        (debouncer, matrix, layout, timer, alarm).lock(
            |debouncer, matrix, layout, timer, alarm| {
                //timer.alarm_0().unwrap().clear_interrupt();
                alarm.clear_interrupt(timer);
                for event in debouncer.events(matrix.get().unwrap()) {
                    layout.event(event);
                }
                layout.tick();
                send_report(layout.keycodes(), &mut usb_class);
            },
        );
        /*c.shared.timer.clear_update_interrupt_flag();

        for event in c
            .shared
            .debouncer
            .events(c.shared.matrix.get().unwrap())
        {
            c.shared.layout.event(event);
        }
        c.shared.layout.tick();
        send_report(c.shared.layout.keycodes(), &mut c.shared.usb_class);*/
    }

    //fn send_report(iter: impl Iterator<Item = KeyCode>, usb_class: &mut usb_class<'_>) {
    fn send_report(iter: impl Iterator<Item = KeyCode>, usb_class: &mut impl Mutex<T = keyberon::Class<'static, UsbBus, Leds>>) {
        let report: KbHidReport = iter.collect();
        if usb_class.lock(|k| k.device_mut().set_keyboard_report(report.clone())) {
            while let Ok(0) = usb_class.lock(|k| k.write(report.as_bytes())) {}
        }

    }

    fn usb_poll(
        usb_dev: &mut usb_device::device::UsbDevice<'static, UsbBus>,
        keyboard: &mut keyberon::Class<'static, UsbBus, Leds>,
    ) {
        if usb_dev.poll(&mut [keyboard]) {
            keyboard.poll();
        }
    }
}
