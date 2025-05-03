<div align="center">

# Sleepy Hollow

[![Made with iced](https://iced.rs/badge.svg)](https://github.com/iced-rs/iced)

A shader filter exploration project comparing custom WGSL downsampling methods with native wgpu options.

*Warning: may cause uncontrollable excitement about shader programming and image processing techniques.*

</div>

## About

This project explores custom WGSL shaders for image downsampling in Rust, using a forked version of the `iced` UI framework. It was created to experiment with shader-based alternatives to wgpu's native image downscaling options (which only offers linear and nearest filters).

## Project Goals

- Explore how to write and implement custom WGSL shaders
- Compare custom shader downsampling to wgpu's native options
- Utilize the headless renderer from `iced` to process images with shaders
- Provide practical examples of cubic (Mitchell-Netravali) and Gaussian filters

## Implementation Details

### Core Components

1. **Filter Implementation** (`filter.rs`):
   - Manages different filter implementations
   - Handles shader setup and rendering

2. **Shader Files** (`filter/*.wgsl`):
   - Various WGSL shader implementations including:
     - Cubic (Mitchell-Netravali)
     - Gaussian
     - Lanczos

3. **Simulator** (`simulator.rs`):
   - Demo application comparing different filtering methods

## Getting Started

To run the project:

```bash
cargo run --release
```

The application demonstrates a toggler to compare different filtering methods.

## Performance Notes

Different shader filters have varying performance characteristics:
- Custom filters are more computationally expensive than built-in wgpu filters
- Results quality varies between implementations
- The project includes performance measurement for comparing approaches

## Educational Focus

This project is primarily educational, focusing on:
- Understanding shader development with WGSL
- Learning how to integrate custom shaders with the `iced` framework
- Exploring image processing techniques in GPU shaders

## License

MIT
