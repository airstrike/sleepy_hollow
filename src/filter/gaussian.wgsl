// Texture and sampler bindings
@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> tex_info: vec4<f32>; // width, height, scale_x, scale_y

// Output from vertex shader to fragment shader
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

// Vertex shader for rendering a full-screen quad
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Vertex positions for a triangle strip (quad)
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0), // bottom-left
        vec2<f32>(1.0, -1.0),  // bottom-right
        vec2<f32>(-1.0, 1.0),  // top-left
        vec2<f32>(1.0, 1.0)    // top-right
    );
    
    // UV coordinates corresponding to each vertex
    var uvs = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0), // bottom-left
        vec2<f32>(1.0, 1.0), // bottom-right
        vec2<f32>(0.0, 0.0), // top-left
        vec2<f32>(1.0, 0.0)  // top-right
    );
    
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    
    return output;
}

// Constants for Gaussian filter
const PI: f32 = 3.14159265359;
const SIGMA: f32 = 1.5; // Standard deviation (controls blur amount)
const KERNEL_RADIUS: i32 = 3; // Practical radius for the kernel

// Gaussian function
fn gaussian(x: f32, sigma: f32) -> f32 {
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sigma * sqrt(2.0 * PI));
}

// Sample the texture using a Gaussian filter
fn gaussian_sample(tex: texture_2d<f32>, samp: sampler, uv: vec2<f32>) -> vec4<f32> {
    // Get texture dimensions
    let width = tex_info.x;
    let height = tex_info.y;
    
    // Calculate pixel position in texture
    let pixel = vec2<f32>(uv.x * width, uv.y * height) - 0.5;
    let center = floor(pixel);
    
    // Calculate the fractional offset
    let offset = pixel - center;
    
    // Precompute Gaussian weights for better accuracy
    var weights_x: array<f32, 7>; // Large enough for radius 3
    var weights_y: array<f32, 7>;
    
    // Calculate Gaussian weights for x and y directions
    var total_weight_x = 0.0;
    var total_weight_y = 0.0;
    
    for (var i = -KERNEL_RADIUS; i <= KERNEL_RADIUS; i++) {
        let idx = i + KERNEL_RADIUS;
        let distance_x = f32(i) - offset.x;
        let distance_y = f32(i) - offset.y;
        
        // Calculate Gaussian weights
        weights_x[idx] = gaussian(distance_x, SIGMA);
        weights_y[idx] = gaussian(distance_y, SIGMA);
        
        // Track total weights for normalization
        total_weight_x += weights_x[idx];
        total_weight_y += weights_y[idx];
    }
    
    // Normalize weights to ensure they sum to 1.0
    for (var i = 0; i < 2*KERNEL_RADIUS+1; i++) {
        weights_x[i] /= total_weight_x;
        weights_y[i] /= total_weight_y; 
    }
    
    // Accumulate weighted samples
    var color = vec4<f32>(0.0);
    
    for (var y = -KERNEL_RADIUS; y <= KERNEL_RADIUS; y++) {
        let y_idx = y + KERNEL_RADIUS;
        let y_weight = weights_y[y_idx];
        
        for (var x = -KERNEL_RADIUS; x <= KERNEL_RADIUS; x++) {
            let x_idx = x + KERNEL_RADIUS;
            let x_weight = weights_x[x_idx];
            
            // Combined weight for this sample
            let weight = x_weight * y_weight;
            
            // Calculate normalized texture coordinates
            let sample_x = (center.x + f32(x) + 0.5) / width;
            let sample_y = (center.y + f32(y) + 0.5) / height;
            
            // Clamp to valid texture coordinates
            let clamped_uv = clamp(vec2<f32>(sample_x, sample_y), vec2<f32>(0.0), vec2<f32>(1.0));
            
            // Sample and accumulate
            color += textureSampleLevel(tex, samp, clamped_uv, 0.0) * weight;
        }
    }
    
    return color;
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Apply Gaussian filtering if downsampling
    if (tex_info.z > 1.0 || tex_info.w > 1.0) {
        // Return the pure Gaussian filtered result without tinting
        return gaussian_sample(texture, tex_sampler, uv);
    } else {
        // Return the unfiltered result when not downsampling
        return textureSample(texture, tex_sampler, uv);
    }
}