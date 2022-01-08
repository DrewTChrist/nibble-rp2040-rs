#![no_std]
#![no_main]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_variables)]

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
    //use cortex_m_rt::entry;
    use defmt_rtt as _;
    use embedded_hal::digital::v2::OutputPin;
    use embedded_hal::spi::MODE_0;
    use embedded_time::duration::Extensions;
    use embedded_time::rate::Extensions as rate_extensions;
    use panic_probe as _;
    use rp2040_hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{bank0::*, dynpin::DynPin, FunctionSpi, Pin, PushPullOutput},
        pac::UART0,
        pio::PIOExt,
        sio::Sio,
        spi::Spi,
        timer::{Alarm0, Timer},
        usb::UsbBus,
        watchdog::Watchdog,
    };

    use core::iter::once;

    use keyberon::debounce::Debouncer;
    use keyberon::layout::{Event, Layout};
    use keyberon::matrix::PressedKeys;
    use keyberon::key_code;
    use crate::demux_matrix::DemuxMatrix;
    use crate::layout as kb_layout;

    use usb_device::class::UsbClass;
    use usb_device::class_prelude::UsbBusAllocator;
    use smart_leds::{brightness, SmartLedsWrite, RGB8};
    use ws2812_pio::Ws2812 as Ws2812Pio;
    use ws2812_spi::Ws2812 as Ws2812Spi;

    const MATRIX_ROWS: usize = 5;
    const MATRIX_COLS: usize = 16;
    const MATRIX_MUX_COLS: usize = 4;
    const SCAN_TIME_US: u32 = 1000000;
    const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;
    const SYS_HZ: u32 = 125_000_000_u32;
    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

    pub struct Leds {
        caps_lock: Pin<Gpio17, PushPullOutput>,
    }

    impl keyberon::keyboard::Leds for Leds {
        fn caps_lock(&mut self, status: bool) {}
    }

    #[shared]
    struct Shared {
        usb_dev: usb_device::device::UsbDevice<'static, UsbBus>,
        //usb_class: keyberon::Class<'static, UsbBus, Leds>,
        usb_class: keyberon::Class<'static, UsbBus, ()>,
        timer: Timer,
        alarm: Alarm0,
        uart: UART0,
        //matrix: Matrix<DynPin, DynPin, 4, 5>,
        #[lock_free]
        matrix: DemuxMatrix<DynPin, DynPin, 4, 5>,
        layout: Layout,
        #[lock_free]
        debouncer: Debouncer<PressedKeys<4, 5>>,
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

        for _ in 0..1000 {
            cortex_m::asm::nop();
        }

        resets.reset.modify(|_, w| w.uart0().clear_bit());
        while resets.reset_done.read().uart0().bit_is_clear() {}
        let uart = c.device.UART0;
        uart.uartibrd.write(|w| unsafe { w.bits(0b0100_0011) });
        uart.uartfbrd.write(|w| unsafe { w.bits(0b0011_0100) });
        uart.uartlcr_h.write(|w| unsafe { w.bits(0b0110_0000) });
        uart.uartcr.write(|w| unsafe { w.bits(0b11_0000_0001) });
        uart.uartimsc.write(|w| w.rxim().set_bit());

        let _spi_sclk = pins.gpio3.into_mode::<FunctionSpi>();
        let _spi_mosi = pins.gpio7.into_mode::<FunctionSpi>();
        let spi = Spi::<_, _, 8>::new(c.device.SPI0).init(
            &mut resets,
            SYS_HZ.Hz(),
            3_000_000u32.Hz(),
            &MODE_0,
        );

        let mut under = Ws2812Spi::new(spi);
        //let mut leds = Leds { caps_lock: onboard };

        let mut under_data: [RGB8; 10] = [RGB8::default(); 10];
        //data[0] = RGB8 { r: 0xFF, g: 0x00, b: 0x00};
        //under.write(under_data.iter().cloned()).unwrap();

        let mut timer = Timer::new(c.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US.microseconds());

        let (mut pio, sm0, _, _, _) = c.device.PIO0.split(&mut resets);

        let mut onboard = Ws2812Pio::new(
            pins.gpio17.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
            timer.count_down(),
        );

        let mut onboard_data: [RGB8; 1] = [RGB8::default(); 1];
        onboard_data[0] = RGB8 { r: 0xFF, g: 0x00, b: 0x00 };
        onboard.write(brightness(once(onboard_data[0]), 32)).unwrap();


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

        //let usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, leds);
        let usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, ());
        let usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

        alarm.enable_interrupt(&mut timer);
        watchdog.start(10_000.microseconds());

        let matrix = DemuxMatrix::new(
            [
                /*pins.gpio18.into_pull_up_input().into(),
                pins.gpio20.into_pull_up_input().into(),
                pins.gpio19.into_pull_up_input().into(),
                pins.gpio10.into_pull_up_input().into(),
                pins.gpio4.into_pull_up_input().into(),*/
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
                /*pins.gpio29.into_push_pull_output().into(),
                pins.gpio28.into_push_pull_output().into(),
                pins.gpio27.into_push_pull_output().into(),
                pins.gpio26.into_push_pull_output().into(),*/
            ],
            16,
            //onboard,
        );

        (
            Shared {
                usb_dev: usb_dev,
                usb_class: usb_class,
                uart: uart,
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

    #[task(binds = USBCTRL_IRQ, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_rx(mut c: usb_rx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        (usb, kb).lock(|usb, kb| {
            if usb.poll(&mut [kb]) {
                kb.poll();
            }
        });
    }

    #[task(priority = 2, capacity = 8, shared = [usb_dev, usb_class, layout])]
    fn handle_event(mut c: handle_event::Context, event: Option<Event>) {
        match event {
            Some(e) => {
                c.shared.layout.lock(|l| l.event(e));
                return;
            }
            None => match c.shared.layout.lock(|l| l.tick()) {
                _ => (),
            },
        };
        let report: key_code::KbHidReport = c.shared.layout.lock(|l| l.keycodes().collect());
        if !c
            .shared
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        if c.shared.usb_dev.lock(|d| d.state()) != usb_device::device::UsbDeviceState::Configured {
            return;
        }
        while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(binds = TIMER_IRQ_0, priority = 1, shared = [uart, matrix, debouncer, timer, alarm, layout, watchdog])]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let mut uart = c.shared.uart;
        let timer = c.shared.timer;
        let alarm = c.shared.alarm;
        (timer, alarm).lock(|t, a| {
            a.clear_interrupt(t);
            let _ = a.schedule(SCAN_TIME_US.microseconds());
        });

        c.shared.watchdog.feed();

        for event in c.shared.debouncer.events(c.shared.matrix.get().unwrap()) {
            let mut byte: u8;
            byte = event.coord().1;
            byte |= (event.coord().0 & 0b0000_0111) << 4;
            byte |= (event.is_press() as u8) << 7;
            // Watchdog will catch any possibility for an infinite loop
            while uart.lock(|u| u.uartfr.read().txff().bit_is_set()) {}
            uart.lock(|u| u.uartdr.write(|w| unsafe { w.data().bits(byte) }));
            handle_event::spawn(Some(event)).unwrap();
        }
        handle_event::spawn(None).unwrap();
    }

    #[task(binds = UART0_IRQ, priority = 4, shared = [uart])]
    fn rx(mut c: rx::Context) {
        // RX FIFO is disabled so we just check that the byte received is valid
        // and then we read it. If a bad byte is received, it is possible that the
        // receiving side will never read. TODO: fix this
        if c.shared.uart.lock(|u| {
            u.uartmis.read().rxmis().bit_is_set()
                && u.uartfr.read().rxfe().bit_is_clear()
                && u.uartdr.read().oe().bit_is_clear()
                && u.uartdr.read().be().bit_is_clear()
                && u.uartdr.read().pe().bit_is_clear()
                && u.uartdr.read().fe().bit_is_clear()
        }) {
            let d: u8 = c.shared.uart.lock(|u| u.uartdr.read().data().bits());
            if (d & 0b10000000) > 0 {
                //handle_event::spawn(Some(Event::Press((d >> 4) & 0b0000_0111, d & 0b0000_1111))).unwrap();
                handle_event::spawn(Some(Event::Press(0, d & 0b0011_1111))).unwrap();
            } else {
                //handle_event::spawn(Some(Event::Release((d >> 4) & 0b0000_0111, d & 0b0000_1111,))).unwrap();
                handle_event::spawn(Some(Event::Release(0, d & 0b0011_1111))).unwrap();
            }
        }
    }
}
