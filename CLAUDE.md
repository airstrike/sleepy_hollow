# Shader Filter Project

## Development
- Use `cargo check` to verify compile errors
- Never use `cargo run`, `cargo test`, or `cargo build` - it's a GUI app that should be run manually

## Project Notes
- Uses custom WGSL shader for Mitchell-Netravali cubic filtering
- Integrates with Iced UI framework
- Target is high-quality image downsampling beyond standard linear filtering
- Shader parameters: B=1/3, C=1/3 for Mitchell-Netravali filter
- Processes 16 texels per output pixel for superior downsampling quality
- Core files: filter.rs, shader.rs, cubic.wgsl
- Main application demonstrates comparison between linear and cubic filtering