use embedded_hal::digital::v2::InputPin;
use keyberon::layout::Event;
use rp2040_hal::gpio::{bank0::Gpio8, bank0::Gpio9, Pin, PullUpInput};

const QUADRATURE_LUT: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];

pub struct Encoder {
    pad_a: Pin<Gpio8, PullUpInput>,
    pad_b: Pin<Gpio9, PullUpInput>,
    left_event: (u8, u8),
    right_event: (u8, u8),
    value: u8,
    state: u8,
    pulses: i8,
    resolution: i8,
}

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

impl Encoder {
    pub fn new(
        pad_a: Pin<Gpio8, PullUpInput>,
        pad_b: Pin<Gpio9, PullUpInput>,
        left_event: (u8, u8),
        right_event: (u8, u8),
    ) -> Self {
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
        s.init_state();
        s
    }
    fn init_state(&mut self) {
        self.state =
            self.pad_a.is_high().unwrap() as u8  | (self.pad_b.is_high().unwrap() as u8) << 1;
    }
    pub fn read_direction(&mut self) -> Direction {
        self.state <<= 2;
        self.state |=
            self.pad_a.is_high().unwrap() as u8  | (self.pad_b.is_high().unwrap() as u8) << 1;

        self.pulses += QUADRATURE_LUT[(self.state & 0xF) as usize];
        if self.pulses >= self.resolution {
            self.value += 1;
        }
        if self.pulses <= -self.resolution {
            self.value -= 1;
        }
        Direction::from_i8(QUADRATURE_LUT[(self.state & 0xF) as usize])
    }
    pub fn read_events(&mut self) -> Option<[Event; 2]> {
        match self.read_direction() {
            Direction::Left => Some([
                Event::Press(self.left_event.0, self.left_event.1),
                Event::Release(self.left_event.0, self.left_event.1),
            ]),
            Direction::Right => Some([
                Event::Press(self.right_event.0, self.right_event.1),
                Event::Release(self.right_event.0, self.right_event.1),
            ]),
            Direction::Still => None,
        }
    }
}
