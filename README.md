# nibble-rp2040-rs

## Description
This is a firmware created with [keyberon](https://github.com/TeXitoi/keyberon) for the [nibble](https://nullbits.co/nibble) keyboard paired with an [adafruit kb2040](https://www.adafruit.com/product/5302).
With slight modification to the code this firmware would likely work with the Sparkfun RP2040 Pro Micro as well. The Nibble is typically built with a Bit-C or
other Pro Micro compatible variant. There is currently no official QMK support for the RP2040 chip.


## Table of Contents

1. [Installation](#installation)
2. [Usage](#usage)
3. [Supported Features](#supportedfeatures)
4. [Build Photos](#buildphotos)

## Installation

### Dependencies
To build this firmware you will need to install the proper Rust target archetecture
and flip-link and elfuf2-rs.

```bash
rustup target install thumbv6m-none-eabi
cargo install flip-link
cargo install elf2uf2-rs
```

### Building the Firmware
Clone this repository in any directory
```bash
git clone https://github.com/drewtchrist/nibble-rp2040-rs
```

```bash
cd nibble-rp2040-rs
```

Putting the KB2040 into bootloader mode and executing the following command
will build the firmware and put it onto the device
```bash
cargo run --release
```

## Usage
The layout can be configured in `src/layout.rs` before flashing to the KB2040.

## Supported Features 
* OLED display

The KB2040 comes with a Stemma QT/Qwiic connector which I have personally used to wire up
my OLED display. This required a bit of modification to the FR4 plate to get the cable to
seat all the way. While this firmware is programmed to use the Stemma QT port the OLED 
display could likely be wired up the original way it was intended to be.

There are currently no fancy dislay animations like bongocat or snail map. I hope to try
supporting them at some point. Right now the OLED just says "Powered by Rust".

* Underglow LEDs 

Underwglow LEDs are customizable. They default to a solid purple, but no other patterns or
colors have been programmed in. I tend to leave them off.

* KB2040 Neopixel 

The KB2040 comes with an onboard RGB LED (Neopixel) that works as the caps lock indicator.
The default is green (off), red (on) and this can be customized however. There is an issue
currently where certain RTIC tasks, namely the OLED task makes this lag a bit. Caps lock always
works regardless of the Neopixel changing and it works correctly for the most part.

* Rotary Encoder 

The rotary encoder works both as a switch when pressed and when turned either direction. This is
set to up and down arrow by default. The current caveat with the rotary encoder is the actions need
to be defined in the layout somewhere even if it isn't a desired key. There are transparent actions 
in the layout array for padding and those can be filled with the desired key presses for the encoder.


## Build Photos
![KB2040 Close Up](/images/kb2040.jpg?raw=true)
![Keyboard without lights](/images/no_under_light.jpg?raw=true)
![Keyboard with lights](/images/under_light.jpg?raw=true)
