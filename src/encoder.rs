use embedded_hal::digital::v2::InputPin;
use rp2040_hal::{
    gpio::{bank0::Gpio8, bank0::Gpio9, Pin, PullUpInput},
};

const QUADRATURE_LUT: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];

pub struct Encoder {
    pub pad_a: Pin<Gpio8, PullUpInput>,
    pub pad_b: Pin<Gpio9, PullUpInput>,
    pub value: u8,
    pub state: u8,
    pub pulses: i8,
    pub resolution: i8,
}

impl Encoder {
    pub fn new(pad_a: Pin<Gpio8, PullUpInput>, pad_b: Pin<Gpio9, PullUpInput>) -> Self {
        let mut s = Self {
            pad_a: pad_a,
            pad_b: pad_b,
            value: 0,
            state: 0, 
            pulses: 0,
            resolution: 4,
        };
        s.init_state();
        s
    }
    fn init_state(&mut self) {
        self.state = (self.pad_a.is_high().unwrap() as u8) << 0 | (self.pad_b.is_high().unwrap() as u8) << 1;
    }
    pub fn read(&mut self) -> i8 {
        self.state <<= 2;
        self.state |= (self.pad_a.is_high().unwrap() as u8) << 0 | (self.pad_b.is_high().unwrap() as u8) << 1;

        self.pulses += QUADRATURE_LUT[(self.state & 0xF) as usize];
        if self.pulses >= self.resolution {
            self.value += 1;
        }
        if self.pulses <= -self.resolution {
            self.value -= 1;
        }
        QUADRATURE_LUT[(self.state & 0xF) as usize]
    }
}
