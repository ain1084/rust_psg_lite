# PSG Lite

[![Crates.io](https://img.shields.io/crates/v/psg_lite.svg)](https://crates.io/crates/psg_lite)
[![Documentation](https://docs.rs/psg_lite/badge.svg)](https://docs.rs/psg_lite)
[![Build Status](https://github.com/ain1084/psg_lite/workflows/Rust/badge.svg)](https://github.com/ain1084/psg_lite/actions?query=workflow%3ARust)
![Crates.io License](https://img.shields.io/crates/l/psg_lite)

This crate generates PCM waveforms similar to those of the AY-3-8910 and its
compatible chips. This crate is not intended for high-quality audio applications.
The main use case is generating simple sound effects and tones in resource-
constrained environments. It is designed with a focus on speed to be usable
on 8-bit CPUs (such as AVR[^1]). Emulating the chip is not the goal. As such,
there are significant differences from the AY-3-8910 in terms of functionality.

* **Hardware Envelope:** Not implemented. Channel volume ranges from 0 to 15.
* **Noise Generator:** The number of bits in the shift register differs. Specifically,
  it is 16 bits instead of 17 bits.
* **Tone Period:** 0 cannot be set. The minimum value is constrained by the clock rate
  and sample rate.

[^1]: Although the C++ implementation (same structure) worked on ATTiny, it has not
been confirmed with Rust.

## Sample Rate

PSG has an extremely simple structure, but the upper limit of the frequency that can
be generated is 125KHz at a clock rate of 2MHz. This crate reduces processing load
by simply thinning out waveforms without performing downsampling. Therefore, at low
sample rates, especially in the high-frequency range, the sound quality deteriorates
significantly. At a sample rate of around 48000Hz, the sound quality is generally
acceptable. If a sample rate of 250KHz is specified, waveforms are generated with
almost no degradation.

In a PC environment, most audio frameworks automatically perform sample rate
conversion before outputting to the device. Therefore, it is often possible to play
back a sample rate of 250KHz without any issues.

## Features

This crate has the following `features` flags:

`float`: Enables generating samples in `f32` format. Enabled by default. If this flag
is not set, floating-point operations are not performed.

## Usage

To use this crate, create a `SoundGenerator` instance, configure the tone period and
mode for the channels, and then generate samples. The interface is simple and
designed for use in embedded systems. Below is a basic example.

```rust
use psg_lite::{SoundGenerator, Output};

fn main() {
    const CLOCK_RATE: u32 = 2_000_000;
    const SAMPLE_RATE: u32 = 48_000;

    let mut generator = SoundGenerator::new(CLOCK_RATE, SAMPLE_RATE);

    generator.set_mode(0, Output::TONE);
    generator.set_volume(0, 15);
    generator.set_period(0, 123);

    for _ in 0..SAMPLE_RATE {
        let sample: i16 = generator.next_sample();
        // Process the sample (e.g., send to DAC or audio buffer)
    }
}
```

## License

Licensed under either of
- Apache License, Version 2.0
([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
