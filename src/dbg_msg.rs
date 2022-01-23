use core::fmt::Write;
use rp2040_hal::pac::UART0;

pub fn _dbg_msg(
    uart: &mut rp2040_hal::uart::UartPeripheral<rp2040_hal::uart::Enabled, UART0>,
    message: &str,
) {
    writeln!(uart, "dbg_msg: \"{:?}\"\r\n", message).unwrap();
}
