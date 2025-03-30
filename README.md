# High-Quality Image Downsampling with Custom Shader in Iced

This project implements a high-quality cubic filter for downsampling images in an Iced application. Instead of relying on the default Linear filter provided by wgpu, this implementation uses the Mitchell-Netravali cubic filter algorithm for superior downsampling results.

## Why Custom Downsampling?

WGPU (and by extension, Iced) only supports two basic filtering modes for image scaling:
- Nearest (creates pixelated results when downscaling)
- Linear (creates blurry results when downscaling)

For high-quality image downsampling, more sophisticated algorithms like cubic filters produce significantly better results by considering more neighboring pixels and using more complex weighting functions.

## Implementation Details

### Core Components

1. **CubicFilter Struct** (`filter.rs`):
   - A wrapper that holds the raw image data and size information
   - Provides a convenient API for creating filtered image elements

2. **Custom Shader** (`shader.rs`):
   - Implements the WGPU pipeline and shader program
   - Manages texture creation and rendering

3. **WGSL Shader Code** (`cubic.wgsl`):
   - Implements the Mitchell-Netravali cubic filter algorithm
   - Samples 16 texels for each output pixel to compute high-quality results

### Mitchell-Netravali Filter

The implemented cubic filter uses the Mitchell-Netravali algorithm with parameters B=1/3, C=1/3, which provides a good balance between sharpness and artifact reduction. The filter works by:

1. Sampling a 4×4 grid of texels around each target pixel
2. Weighting each texel's contribution based on its distance using the Mitchell-Netravali cubic function
3. Combining these weighted values to produce the final pixel color

## Usage

To use the high-quality downsampling, replace standard image elements:

```rust
// Standard image with linear filtering
let image_element = iced::widget::image(image_handle)
    .content_fit(ContentFit::Contain)
    .width(container_width);

// Replace with cubic filtered image
let image_element = filter::cubic_filtered_image(
    image_data,              // Raw RGBA data
    image_size,              // Original image size
    Size::new(width, height) // Target size
);
```

## Performance Considerations

The custom cubic filter is more computationally expensive than the built-in Linear filter:
- It samples 16 texels per output pixel (vs. 4 for bilinear)
- It performs more complex calculations per pixel

However, modern GPUs can handle this filtering efficiently for most use cases. For performance-critical applications, consider:

1. Only using the cubic filter for final output or when quality is important
2. Falling back to the Linear filter for real-time operations

## Future Improvements

Potential enhancements could include:
- Support for different cubic filter parameters
- Optimizations for multi-pass filtering on extremely large images
- Additional high-quality filter algorithms (Lanczos, etc.)
