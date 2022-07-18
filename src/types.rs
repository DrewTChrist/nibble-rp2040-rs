#![allow(dead_code)]

mod kb2040 {
    use rp2040_hal::gpio::{bank0, Function, Pin, PullUpInput, I2C};
    use rp2040_hal::pac::PIO0;
    use rp2040_hal::pio::{SM0, SM1};
    use ws2812_pio::Ws2812Direct;
    pub type EncoderPadA = Pin<bank0::Gpio8, PullUpInput>;
    pub type EncoderPadB = Pin<bank0::Gpio9, PullUpInput>;
    pub type OnBoardLED = Ws2812Direct<PIO0, SM0, bank0::Gpio17>;
    pub type Underglow = Ws2812Direct<PIO0, SM1, bank0::Gpio7>;
    pub type Sda = Pin<bank0::Gpio12, Function<I2C>>;
    pub type Scl = Pin<bank0::Gpio13, Function<I2C>>;
}

mod rp2040_pro_micro {
    use rp2040_hal::gpio::{bank0, Function, Pin, PullUpInput, I2C};
    use rp2040_hal::pac::PIO0;
    use rp2040_hal::pio::{SM0, SM1};
    use ws2812_pio::Ws2812Direct;
    pub type EncoderPadA = Pin<bank0::Gpio8, PullUpInput>;
    pub type EncoderPadB = Pin<bank0::Gpio9, PullUpInput>;
    pub type OnBoardLED = Ws2812Direct<PIO0, SM0, bank0::Gpio25>;
    pub type Underglow = Ws2812Direct<PIO0, SM1, bank0::Gpio7>;
    pub type Sda = Pin<bank0::Gpio16, Function<I2C>>;
    pub type Scl = Pin<bank0::Gpio17, Function<I2C>>;
}

mod bit_c_rp2040 {
    //type EncoderPadA = todo!();
    //type EncoderPadB = todo!();
    //type OnBoardLED = todo!();
    //type Underglow = todo!();
    //type Sda = todo!();
    //type Sdl = todo!();
}

#[cfg(feature = "kb2040")]
pub mod active {
    use crate::types::kb2040;
    pub type EncoderPadA = kb2040::EncoderPadA;
    pub type EncoderPadB = kb2040::EncoderPadB;
    pub type OnBoardLED = kb2040::OnBoardLED;
    pub type Underglow = kb2040::Underglow;
    pub type Sda = kb2040::Sda;
    pub type Scl = kb2040::Scl;
}

#[cfg(feature = "rp2040-pro-micro")]
pub mod active {
    use crate::types::rp2040_pro_micro;
    pub type EncoderPadA = rp2040_pro_micro::EncoderPadA;
    pub type EncoderPadB = rp2040_pro_micro::EncoderPadB;
    pub type OnBoardLED = rp2040_pro_micro::OnBoardLED;
    pub type Underglow = rp2040_pro_micro::Underglow;
    pub type Sda = rp2040_pro_micro::Sda;
    pub type Scl = rp2040_pro_micro::Scl;
}

#[cfg(feature = "bit-c-rp2040")]
pub mod active {
    use crate::types::bit_c_rp2040;
    pub type EncoderPadA = bit_c_rp2040::EncoderPadA;
    pub type EncoderPadB = bit_c_rp2040::EncoderPadB;
    pub type OnBoardLED = bit_c_rp2040::OnBoardLED;
    pub type Underglow = bit_c_rp2040::Underglow;
    pub type Sda = bit_c_rp2040::Sda;
    pub type Scl = bit_c_rp2040::Scl;
}
