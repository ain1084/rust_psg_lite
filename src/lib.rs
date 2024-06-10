/*!
# PSG Lite

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
*/

#![no_std]

use bitflags::bitflags;
use core::{array, cmp};
use paste::paste;

const CHANNELS: usize = 3;

bitflags! {
/// Indicates the output of the channel.
    /// Used in the channel output setting (set_mode).
    ///
    /// These are bitwise flags. Use TONE | NOISE to output both noise and tone.
    #[derive(Clone, Copy)]
    pub struct Output: u8 {
        /// No output
        const NONE  = 0b00;
        /// Output tone
        const TONE  = 0b01;
        /// Output noise
        const NOISE = 0b10;
    }
}

struct ToneGenerator {
    clock_rate: u32,
    sample_rate_x8: u32,
    error: i64,
    period_min: u16,
    source: u64,
    output: Output,
}

impl ToneGenerator {
    fn new(clock_rate: u32, sample_rate: u32) -> Self {
        let sample_rate_x8 = sample_rate * 8;
        let period = (clock_rate / (sample_rate_x8 * 2) + 1) as u16;
        let source = period as u64 * sample_rate_x8 as u64;
        Self {
            clock_rate,
            sample_rate_x8,
            error: clock_rate as i64,
            period_min: period,
            source,
            output: Output::NONE,
        }
    }

    fn set_period(&mut self, period: u16) {
        assert!(period < 4096);
        let period = cmp::max(period, self.period_min);
        self.source = self.sample_rate_x8 as u64 * period as u64;
    }

    fn update(&mut self) -> Output {
        self.error -= self.clock_rate as i64;
        if self.error < 0 {
            self.error += self.source as i64;
            self.output.toggle(Output::TONE);
        }
        self.output
    }
}

struct Channel {
    generator: ToneGenerator,
    volume: u8,
    mode: Output,
}

impl Channel {
    fn new(clock_rate: u32, sample_rate: u32) -> Self {
        Self {
            generator: ToneGenerator::new(clock_rate, sample_rate),
            volume: 0,
            mode: Output::NONE,
        }
    }

    fn set_period(&mut self, period: u16) {
        self.generator.set_period(period)
    }

    fn set_mode(&mut self, mode: Output) {
        self.mode = mode
    }

    fn set_volume(&mut self, volume: u8) {
        assert!(volume < 16);
        self.volume = volume
    }

    fn update(&mut self, noise: Output) -> u8 {
        if (self.generator.update() | noise).contains(self.mode) {
            0
        } else {
            self.volume
        }
    }
}

struct NoiseGenerator {
    clock_rate: u32,
    sample_rate_x16: u32,
    error: i32,
    period_min: u8,
    source: u32,
    shift: u16,
}

impl NoiseGenerator {
    fn new(clock_rate: u32, sample_rate: u32) -> Self {
        let sample_rate_x16 = sample_rate * 16;
        let period_min = (clock_rate / sample_rate_x16) as u8;
        let source = (period_min + 1) as u32 * sample_rate_x16;
        Self {
            clock_rate,
            sample_rate_x16,
            error: clock_rate as i32,
            period_min,
            source,
            shift: 0b1,
        }
    }

    fn set_period(&mut self, period: u8) {
        assert!(period < 32);
        let period = cmp::max(self.period_min, period);
        self.source = (period + 1) as u32 * self.sample_rate_x16
    }

    fn update(&mut self) -> Output {
        self.error -= self.clock_rate as i32;
        if self.error < 0 {
            self.error += self.source as i32;
            self.shift = (self.shift >> 1) | (self.shift ^ (self.shift >> 3)) << 15;
        }
        if self.shift & 1 != 0 {
            Output::NOISE
        } else {
            Output::NONE
        }
    }
}

/// Generates waveforms for PSG.
pub struct SoundGenerator {
    clock_rate: u32,
    sample_rate: u32,
    channels: [Channel; CHANNELS],
    noise: NoiseGenerator,
}

impl SoundGenerator {
    /// Creates a new `SoundGenerator`.
    ///
    /// # Arguments
    /// - `clock_rate`: The clock rate in Hz. The standard value is 2MHz (2_000_000).
    /// - `sample_rate`: The sample rate in Hz. Specify values like 44100, 48000, or 250000.
    ///
    /// # Returns
    /// A new `SoundGenerator` instance.
    pub fn new(clock_rate: u32, sample_rate: u32) -> Self {
        Self {
            clock_rate,
            sample_rate,
            channels: array::from_fn(|_| Channel::new(clock_rate, sample_rate)),
            noise: NoiseGenerator::new(clock_rate, sample_rate),
        }
    }

    /// Returns the clock rate.
    ///
    /// # Returns
    /// The clock rate in Hz.
    pub fn clock_rate(&self) -> u32 {
        self.clock_rate
    }

    /// Returns the sample rate.
    ///
    /// # Returns
    /// The sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Sets the tone period for the specified channel.
    ///
    /// # Arguments
    /// - `channel`: The channel number (0-2).
    /// - `period`: The output period (1-4095).
    pub fn set_period(&mut self, channel: usize, period: u16) {
        self.channels[channel].set_period(period)
    }

    /// Sets the volume for the specified channel.
    ///
    /// # Arguments
    /// - `channel`: The channel number (0-2).
    /// - `volume`: The volume (0-15).
    pub fn set_volume(&mut self, channel: usize, volume: u8) {
        self.channels[channel].set_volume(volume)
    }

    /// Sets the mode for the specified channel.
    ///
    /// # Arguments
    /// - `channel`: The channel number (0-2).
    /// - `mode`: The mode (logical OR of TONE and NOISE).
    pub fn set_mode(&mut self, channel: usize, mode: Output) {
        self.channels[channel].set_mode(mode)
    }

    /// Sets the noise period.
    ///
    /// # Arguments
    /// - `period`: The noise period (0-31).
    pub fn set_noise_period(&mut self, period: u8) {
        self.noise.set_period(period)
    }

    /// Generates and returns the next sample value.
    ///
    /// # Returns
    /// A sample value of type T.
    ///
    /// # Note
    /// T must implement the `OutputSample` trait. `OutputSample` is implemented
    /// for `f32` and `i16`.
    pub fn next_sample<T: OutputSample<T>>(&mut self) -> T {
        T::next_sample(self)
    }
}

/// A trait for generating sample values.
///
pub trait OutputSample<T> {
    /// Generates and returns the next sample.
    ///
    /// # Arguments
    /// - `sg`: A reference to the `SoundGenerator`.
    ///
    /// # Returns
    /// A sample value of type T.
    fn next_sample(sg: &mut SoundGenerator) -> T;
}

macro_rules! output_mixer_table_impl {
    ($T:ty, $DIVIDER:expr, [$($VALUE:expr),*]) => {
        paste! {
            #[allow(non_upper_case_globals)]
            const [<OUTPUT_VOLUME_TABLE_$T>]: [$T; 16] =
            [
                $(
                    (($VALUE as $T)  / ($DIVIDER as $T)) as $T,
                )*
            ];
        }
    };
}

macro_rules! output_mixer_impl {
    ($({$T:ty, $DIVIDER:expr})*) => {
        $(
            // The following table generation was based on fmgen_008.lzh (Copyright (C) cisc 1997, 1999)
            // http://retropc.net/cisc/sound
            //
            // Table geeneration code in Rust
           // fn main() {
            //     const CHANNELS: u16 = 3;
            //     let mul = 1.0 / (4.0 as f32).powf(1.0 / 4.0);
            //     let ar: [u16; 16] = std::array::from_fn(|i|if i != 0 { (mul.powi((15 - i) as i32) * 65535.0 / CHANNELS as f32) as u16} else { 0 });
            //     println!("{:?}", ar);
            // }
            output_mixer_table_impl!($T, $DIVIDER, [
                0,
                170,
                241,
                341,
                482,
                682,
                965,
                1365,
                1930,
                2730,
                3861,
                5461,
                7723,
                10922,
                15446,
                21845
            ]);
            paste! {
                impl OutputSample<$T> for $T {
                    fn next_sample(generator: &mut SoundGenerator) -> $T {
                        let noise = generator.noise.update();
                        generator
                            .channels
                            .iter_mut()
                            .fold(Default::default(), |sum, channel| {
                                sum + unsafe { [<OUTPUT_VOLUME_TABLE_$T>].get_unchecked(channel.update(noise) as usize) }
                            })
                    }
                }
            }
        )*

    }
}

output_mixer_impl! {
    {i16, 1 << 1}
}

#[cfg(feature = "float")]
output_mixer_impl! {
    {f32, u16::MAX }
}

#[cfg(test)]
mod tests {
    use crate::{Output, SoundGenerator};

    #[test]
    fn test() {
        const CLOCK_RATE: u32 = 2_000_0000;
        const SAMPLE_RATE: u32 = CLOCK_RATE / 8;
        let mut generator = SoundGenerator::new(CLOCK_RATE, SAMPLE_RATE);

        generator.set_mode(0, Output::TONE);
        generator.set_volume(0, 15);
        generator.set_period(0, 1);

        // dummy
        generator.next_sample::<i16>();

        let mut zero = 0_usize;
        let mut non_zero = 0_usize;

        (0..SAMPLE_RATE).for_each(|_| {
            let v: i16 = generator.next_sample();
            if v != 0 {
                non_zero += 1;
            } else {
                zero += 1;
            }
        });
        assert_eq!(zero, non_zero);
    }
}
