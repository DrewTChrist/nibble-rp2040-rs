use embedded_hal::digital::v2::InputPin;
use keyberon::layout::Event;

const QUADRATURE_LUT: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];

pub enum Direction {
    Left = 1,
    Still = 0,
    Right = -1,
}

impl Direction {
    fn from_i8(value: i8) -> Direction {
        match value {
            1 => Direction::Left,
            0 => Direction::Still,
            -1 => Direction::Right,
            _ => Direction::Still,
        }
    }
}

pub struct Encoder<A, B>
where
    A: InputPin,
    B: InputPin,
{
    pad_a: A,
    pad_b: B,
    left_event: (u8, u8),
    right_event: (u8, u8),
    value: u8,
    state: u8,
    pulses: i8,
    resolution: i8,
}

impl<A: InputPin, B: InputPin> Encoder<A, B>
where
    A: InputPin,
    B: InputPin,
{
    pub fn new<E>(
        pad_a: A,
        pad_b: B,
        left_event: (u8, u8),
        right_event: (u8, u8),
    ) -> Result<Self, E>
    where
        A: InputPin<Error = E>,
        B: InputPin<Error = E>,
    {
        let mut s = Self {
            pad_a,
            pad_b,
            left_event,
            right_event,
            value: 0,
            state: 0,
            pulses: 0,
            resolution: 4,
        };
        s.init_state()?;
        Ok(s)
    }
    fn init_state<E>(&mut self) -> Result<(), E>
    where
        A: InputPin<Error = E>,
        B: InputPin<Error = E>,
    {
        self.state = self.pad_a.is_high()? as u8 | (self.pad_b.is_high()? as u8) << 1;
        Ok(())
    }
    fn read_direction<E>(&mut self) -> Result<Direction, E>
    where
        A: InputPin<Error = E>,
        B: InputPin<Error = E>,
    {
        self.state <<= 2;
        self.state |= self.pad_a.is_high()? as u8 | (self.pad_b.is_high()? as u8) << 1;

        self.pulses += QUADRATURE_LUT[(self.state & 0xF) as usize];
        if self.pulses >= self.resolution {
            self.value += 1;
        }
        if self.pulses <= -self.resolution {
            self.value -= 1;
        }
        Ok(Direction::from_i8(
            QUADRATURE_LUT[(self.state & 0xF) as usize],
        ))
    }
    pub fn read_events<E>(&mut self) -> Result<Option<[Event; 2]>, E>
    where
        A: InputPin<Error = E>,
        B: InputPin<Error = E>,
    {
        match self.read_direction() {
            Ok(Direction::Left) => Ok(Some([
                Event::Press(self.left_event.0, self.left_event.1),
                Event::Release(self.left_event.0, self.left_event.1),
            ])),
            Ok(Direction::Right) => Ok(Some([
                Event::Press(self.right_event.0, self.right_event.1),
                Event::Release(self.right_event.0, self.right_event.1),
            ])),
            Ok(Direction::Still) => Ok(None),
            _ => Ok(None),
        }
    }
}
