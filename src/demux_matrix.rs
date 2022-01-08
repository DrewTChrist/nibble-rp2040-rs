#![allow(missing_docs)]

use embedded_hal::digital::v2::{PinState, InputPin, OutputPin};
use keyberon::matrix::PressedKeys;
use core::convert::TryInto;

const COL_SHIFTER: u32 = 1;

pub struct DemuxMatrix<C, R, const CS: usize, const RS: usize>
where
    C: OutputPin,
    R: InputPin,
{
    cols: [C; CS],
    rows: [R; RS],
    true_cols: usize,
}

impl<C: OutputPin, R: InputPin, const CS: usize, const RS: usize> DemuxMatrix<C, R, CS, RS>
where
    C: OutputPin,
    R: InputPin,
{
    pub fn new<E>(cols: [C; CS], rows: [R; RS], true_cols: usize) -> Result<Self, E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        let mut res = Self { cols, rows, true_cols };
        res.clear()?;
        Ok(res)
    }
    pub fn clear<E>(&mut self) -> Result<(), E>
    where
        C: OutputPin<Error = E>, 
        R: InputPin<Error = E>,

    {
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
        for bit in 0..self.cols.len() {
            let state: u8 = ((col & (0b1 << bit)) >> bit).try_into().unwrap();
            if state == 0 {
                match self.cols[bit].set_state(PinState::Low) {
                    Ok(()) => {},
                    // This needs to panic
                    Err(e) => {},
                }
            } else if state == 1 {
                match self.cols[bit].set_state(PinState::High) {
                    Ok(()) => {},
                    // This needs to panic
                    Err(e) => {},
                }
            }
        }
    }
    pub fn get<E>(&mut self) -> Result<PressedKeys<CS, RS>, E>
    where
        C: OutputPin<Error = E>,
        R: InputPin<Error = E>,
    {
        let mut keys = PressedKeys::default();

        for current_col in 0..self.true_cols {
            self.select_column(current_col);
            // Nibble QMK firmware sleeps 5 ms here
            // Should look into that
            for (ri, row) in (&mut self.rows).iter_mut().enumerate() {
                if !row.is_high()? {
                    let tmp = keys.0[ri][current_col] as u32 | COL_SHIFTER << current_col;
                    keys.0[ri][current_col] = tmp != 0;
                } else {
                    let tmp = keys.0[ri][current_col] as u32 & !(COL_SHIFTER << current_col);
                    keys.0[ri][current_col] = tmp != 0;
                }
            }
        }
        Ok(keys)
    }
}
