use core::fmt::Write;
use rp2040_hal::pac::UART0;

// cargo build/run --features "debug"
//#[cfg(feature = "debug")]
#[cfg(debug_assertions)]
pub fn dbg_msg(
    uart: &mut rp2040_hal::uart::UartPeripheral<rp2040_hal::uart::Enabled, UART0>,
    message: &str,
) {
    //match cfg!(feature = "debug") {
    match cfg!(debug_assertions) {
        true => {
            writeln!(uart, "dbg_msg: \"{:?}\"\r\n", message).unwrap();
        }
        false => {
            writeln!(uart, "You must enable debug feature\r\n").unwrap();
        }
    }
}

// cargo build/run
//#[cfg(not(feature = "debug"))]
#[cfg(not(debug_assertions))]
pub fn dbg_msg(_message: &str) {}
