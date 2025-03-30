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

// Constants for Lanczos filter
const PI: f32 = 3.14159265359;
const LANCZOS_A: f32 = 2.0; // Lanczos-2 filter (good balance of quality and speed)

// Lanczos filter kernel calculation
fn lanczos(x: f32) -> f32 {
    let abs_x = abs(x);
    
    if (abs_x < 0.0001) {
        return 1.0;
    } else if (abs_x < LANCZOS_A) {
        let pi_x = PI * abs_x;
        return (LANCZOS_A * sin(pi_x) * sin(pi_x / LANCZOS_A)) / (pi_x * pi_x);
    } else {
        return 0.0;
    }
}

// Optimized Lanczos filter for clean image downsampling
fn lanczos_sample(tex: texture_2d<f32>, samp: sampler, uv: vec2<f32>) -> vec4<f32> {
    // Get texture dimensions
    let width = tex_info.x;
    let height = tex_info.y;
    
    // Calculate pixel position in texture
    let pixel = vec2<f32>(uv.x * width, uv.y * height) - 0.5;
    let center = floor(pixel);
    
    // Calculate the fractional offset
    let offset = pixel - center;
    
    // Sample size based on Lanczos parameter
    let radius = i32(ceil(LANCZOS_A));
    
    // Precompute weights for better numerical stability
    var weights_x: array<f32, 5>; // Large enough for LANCZOS_A = 2
    var weights_y: array<f32, 5>; 
    
    // Populate weights for x dimension
    for (var i = 0; i < 2*radius+1; i++) {
        if (i < 5) { // Safety check for array bounds
            weights_x[i] = lanczos(f32(i - radius) - offset.x);
        }
    }
    
    // Populate weights for y dimension
    for (var i = 0; i < 2*radius+1; i++) {
        if (i < 5) { // Safety check for array bounds
            weights_y[i] = lanczos(f32(i - radius) - offset.y);
        }
    }
    
    // Accumulate weighted samples
    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;
    
    for (var y = 0; y < 2*radius+1; y++) {
        if (y >= 5) { continue; } // Safety check for array bounds
        let y_weight = weights_y[y];
        if (y_weight == 0.0) { continue; }
        
        for (var x = 0; x < 2*radius+1; x++) {
            if (x >= 5) { continue; } // Safety check for array bounds
            let x_weight = weights_x[x];
            if (x_weight == 0.0) { continue; }
            
            // Calculate combined weight
            let weight = x_weight * y_weight;
            
            // Calculate texture coordinates
            let sample_x = center.x + f32(x - radius) + 0.5;
            let sample_y = center.y + f32(y - radius) + 0.5;
            let u = sample_x / width;
            let v = sample_y / height;
            
            // Clamp to valid texture coordinates
            let clamped_u = clamp(u, 0.0, 1.0);
            let clamped_v = clamp(v, 0.0, 1.0);
            
            // Sample and accumulate
            let texel = textureSampleLevel(tex, samp, vec2<f32>(clamped_u, clamped_v), 0.0);
            color += texel * weight;
            weight_sum += weight;
        }
    }
    
    // Handle edge case with zero weight (shouldn't happen with proper implementation)
    if (weight_sum < 0.0001) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0); // Magenta for debugging
    }
    
    // Normalize and return final color
    return color / weight_sum;
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Apply Lanczos filtering if downsampling
    if (tex_info.z > 1.0 || tex_info.w > 1.0) {
        return lanczos_sample(texture, tex_sampler, uv);
    } else {
        return textureSample(texture, tex_sampler, uv);
    }
}