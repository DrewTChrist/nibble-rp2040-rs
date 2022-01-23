# nibble-rp2040-rs

## Description
This is a firmware created with [keyberon](https://github.com/TeXitoi/keyberon) for the [nibble](https://nullbits.co/nibble) keyboard paired with an [adafruit kb2040](https://www.adafruit.com/product/5302) 


## Table of Contents

1. [Installation](#installation)
2. [Usage](#usage)
3. [Build Photos](#build photos)

## Features (Lack of)
* Currently there is no OLED support
* Underglow LEDs have minimal functionality
* KB2040 neopixel has minimal functionality
* Rotary encoder works as a key, but the knob is not functional yet

## Installation

To install this firmware clone in any directory
```bash
git clone https://github.com/drewtchrist/nibble-rp2040-rs
```

```bash
cd nibble-rp2040-rs
```

Put your KB2040 into bootloader mode and execute the following command
```bash
cargo run --release
```

## Usage
The layout can be configured in `src/layout.rs` before flashing to the KB2040.

## Build Photos
![KB2040 Close Up](/images/kb2040.jpg?raw=true)
![Keyboard without lights](/images/no_under_light.jpg?raw=true)
![Keyboard with lights](/images/under_light.jpg?raw=true)
