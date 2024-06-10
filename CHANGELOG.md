# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2024-06-10
### Fixed
- Fix link to build status

## [0.1.0] - 2024-06-10
### Added
- Initial release of `psg_lite` crate.
- `SoundGenerator` for generating PCM waveforms similar to the AY-3-8910 and its compatible chips.
- Support for generating tone and noise outputs.
- `Output` enum for specifying tone and noise outputs.
- `set_period`, `set_volume`, and `set_mode` methods for configuring tone generators.
- `next_sample` method for generating the next audio sample.
- `float` feature for enabling `f32` format sample generation.
- Comprehensive documentation for using the crate.
- Basic example in the documentation to demonstrate usage.

### Notes
- The crate is designed to be used in resource-constrained environments, particularly on 8-bit CPUs.
- Differences from the AY-3-8910 include:
  - No support for hardware envelope generation.
  - Noise generator shift register is 16 bits instead of 17 bits.
  - Minimum tone period constrained by the clock rate and sample rate.

