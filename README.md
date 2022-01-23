# nibble-rp2040-rs

## Description
This is a firmware created with [keyberon](https://github.com/TeXitoi/keyberon) for the [nibble](https://nullbits.co/nibble) keyboard paired with an [adafruit kb2040](https://www.adafruit.com/product/5302) 


## Table of Contents

1. [Installation](#installation)
2. [Usage](#usage)

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
