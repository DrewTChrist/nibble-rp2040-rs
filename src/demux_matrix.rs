use crate::dbg_msg::dbg_msg;
use core::convert::TryInto;
use core::str;
use cortex_m;
use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
use embedded_hal::timer::CountDown;
use keyberon::matrix::PressedKeys;
use rp2040_hal::pac::UART0;

pub struct DemuxMatrix<C, R, const CS: usize, const RS: usize>
where
    C: OutputPin,
    R: InputPin,
{
    //cols: [C; CS],
    cols: [C; 4],
    rows: [R; RS],
    true_cols: usize,
    //pub uart: rp2040_hal::uart::UartPeripheral<rp2040_hal::uart::Enabled, UART0>,
}

impl<C: OutputPin, R: InputPin, const CS: usize, const RS: usize> DemuxMatrix<C, R, CS, RS>
where
    C: OutputPin,
    R: InputPin,
{
    pub fn new<E>(
        //cols: [C; CS],
        cols: [C; 4],
        rows: [R; RS],
        true_cols: usize,
        //uart: rp2040_hal::uart::UartPeripheral<rp2040_hal::uart::Enabled, UART0>,
    ) -> Result<Self, E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        let mut res = Self {
            cols,
            rows,
            true_cols,
            //uart,
        };
        res.clear()?;
        Ok(res)
    }
    pub fn clear<E>(&mut self) -> Result<(), E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        //dbg_msg(&mut self.uart, "clear");
        for c in self.cols.iter_mut() {
            c.set_low()?;
        }
        Ok(())
    }
    fn select_column<E>(&mut self, col: usize)
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        //dbg_msg(&mut self.uart, "select_column");
        for bit in 0..self.cols.len() {
            let state: u8 = ((col & (0b1 << bit)) >> bit).try_into().unwrap();
            //dbg_msg(&mut self.uart, &str::from_utf8(&[state]).unwrap());
            //dbg_msg(&mut self.uart, &str::from_utf8(&[state]).unwrap());
            if state == 0 {
                //dbg_msg(&mut self.uart, "state == 0");
                match self.cols[bit].set_state(PinState::Low) {
                    Ok(()) => {}
                    // This needs to panic
                    Err(e) => {
                        //dbg_msg(&mut self.uart, "error setting pin state");
                    }
                }
            } else if state == 1 {
                //dbg_msg(&mut self.uart, "state == 1");
                match self.cols[bit].set_state(PinState::High) {
                    Ok(()) => {}
                    // This needs to panic
                    Err(e) => {
                        //dbg_msg(&mut self.uart, "error setting pin state");
                    }
                }
            }
        }
    }
    pub fn get<E>(&mut self) -> Result<PressedKeys<CS, RS>, E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        //dbg_msg(&mut self.uart, "get");
        let mut keys = PressedKeys::default();

        for current_col in 0..self.true_cols {
            //dbg_msg(&mut self.uart, &str::from_utf8(&[current_col as u8]).unwrap());
            self.select_column(current_col);
            // Nibble QMK firmware sleeps 5 ms here
            // Should look into that
            for i in 0..1000 {
                cortex_m::asm::nop();
            }
            for (ri, row) in (&mut self.rows).iter_mut().enumerate() {
                //dbg_msg(&mut self.uart, &str::from_utf8(&[ri as u8]).unwrap());
                if !row.is_high()? {
                    //dbg_msg(&mut self.uart, "row is low");
                    keys.0[ri][current_col] = true;
                } else {
                    //dbg_msg(&mut self.uart, "row is high");
                    keys.0[ri][current_col] = false;
                }
            }
        }
        Ok(keys)
    }
}
