# nibble-rp2040-rs

## Description
This is a firmware created with [keyberon](https://github.com/TeXitoi/keyberon) for the [nibble](https://nullbits.co/nibble) keyboard. This firmware is meant to 
target an [adafruit kb2040](https://www.adafruit.com/product/5302) or [sparkfun rp2040 pro micro](https://www.sparkfun.com/products/18288).
The Nibble is typically built with a [Bit-C](https://nullbits.co/bit-c/) (mega32u4). 

### Why use the RP2040 and why use Rust?
* The RP2040 is a great chip with a lot of features
* Rust is a great programming language
* I wanted a practical project to learn more about embedded programming
* I can tell people my keyboard is powered by Rust

## Table of Contents

1. [Installing Dependencies](#installing dependencies)
2. [Layout](#layout)
3. [Building/Flashing](#building/flashing)
4. [Supported Features](#supported features)
5. [Build Photos](#buildphotos)

## Installing Dependencies
To build this firmware you will need to install the proper Rust target architecture
as well as flip-link and elfuf2-rs.

```bash
rustup target install thumbv6m-none-eabi
cargo install flip-link
cargo install elf2uf2-rs
```

## Layout
The layout can be configured in `src/layout.rs` before flashing to the KB2040.

## Building/Flashing
Clone this repository in any directory
```bash
git clone https://github.com/drewtchrist/nibble-rp2040-rs
```

```bash
cd nibble-rp2040-rs
```
#### If you are using an Adafruit KB2040 you'll want to build with this command:
```bash
cargo run --release --features kb2040
```

#### If you are using a Sparkfun RP2040 Pro Micro you'll want to build with this command:
```bash
cargo run --release --features rp2040-pro-micro
```

## Supported Features 
* OLED display

OLED support is still work in progress.

The KB2040 and the Sparkfun RP2040 Pro Micro come with Stemma QT/Qwiic connectors. **For now** this firmware expects
the OLED to be connected to this port, **not soldered to the four pads below the rotary encoder**.

There are currently no fancy display animations like bongocat or snail map. I hope to try
supporting them at some point. 

* Underglow LEDs 

Underglow LEDs are customizable. They default to a solid purple, but no other patterns or
colors have been programmed in. I tend to leave them off.

* KB2040 Neopixel 

The KB2040 comes with an onboard RGB LED (Neopixel) that works as the caps lock indicator.
The default colors are green (off), red (on) and this can be customized however. 

* Rotary Encoder 

The rotary encoder works both as a switch when pressed and when turned either direction. This is
set to up and down arrow by default. The current caveat with the rotary encoder is the actions need
to be defined in the layout somewhere even if it isn't a desired key. There are transparent actions 
in the layout array for padding and those can be filled with the desired key presses for the encoder.


## Build Photos
![KB2040 Close Up](/images/kb2040.jpg?raw=true)
![Keyboard without lights](/images/no_under_light.jpg?raw=true)
![Keyboard with lights](/images/under_light.jpg?raw=true)
