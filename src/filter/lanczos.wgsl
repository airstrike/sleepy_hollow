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

// Lanczos filter implementation (a=3)
fn lanczos(x: f32) -> f32 {
    let a: f32 = 3.0; // Lanczos parameter (support width)
    
    let abs_x = abs(x);
    
    if (abs_x < 0.0001) {
        return 1.0;
    } else if (abs_x < a) {
        let pi_x = 3.14159265359 * abs_x;
        return a * sin(pi_x) * sin(pi_x / a) / (pi_x * pi_x);
    } else {
        return 0.0;
    }
}

// Sample the texture using Lanczos filtering
fn lanczos_sample(tex: texture_2d<f32>, samp: sampler, uv: vec2<f32>) -> vec4<f32> {
    // Get texture dimensions
    let width = tex_info.x;
    let height = tex_info.y;
    
    // Calculate pixel position in texture
    let pixel = vec2<f32>(uv.x * width, uv.y * height) - 0.5;
    let center = floor(pixel);
    
    // Calculate the fractional offset
    let offset = pixel - center;
    
    // For a=3 Lanczos, sample 6x6 grid of texels (from -2 to 3)
    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;
    
    for (var y: i32 = -2; y <= 3; y++) {
        let y_dist = abs(f32(y) - offset.y);
        let y_weight = lanczos(y_dist);
        
        for (var x: i32 = -2; x <= 3; x++) {
            let x_dist = abs(f32(x) - offset.x);
            let x_weight = lanczos(x_dist);
            
            let weight = x_weight * y_weight;
            
            // Calculate normalized texture coordinates
            let u = (center.x + f32(x) + 0.5) / width;
            let v = (center.y + f32(y) + 0.5) / height;
            
            // Sample the texture and apply weight
            color += textureSampleLevel(tex, samp, vec2<f32>(u, v), 0.0) * weight;
            weight_sum += weight;
        }
    }
    
    // Normalize result (in case weights don't sum exactly to 1.0)
    if (weight_sum > 0.0) {
        return color / weight_sum;
    } else {
        return textureSample(tex, tex_sampler, uv);
    }
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