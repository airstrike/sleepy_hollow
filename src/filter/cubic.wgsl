// Texture and sampler bindings
@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> tex_info: vec4<f32>; // width, height, scale_x, scale_y

// Vertex shader for rendering a full-screen quad
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Vertex positions for a triangle strip (quad)
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0), // bottom-left
        vec2<f32>(1.0, -1.0),  // bottom-right
        vec2<f32>(-1.0, 1.0),  // top-left
        vec2<f32>(1.0, 1.0)    // top-right
    );
    
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

// Implementation of Mitchell-Netravali cubic filter
fn mitchell(t: f32) -> f32 {
    let B: f32 = 1.0 / 3.0; // Mitchell-Netravali parameters
    let C: f32 = 1.0 / 3.0;
    
    let abs_t = abs(t);
    
    if (abs_t < 1.0) {
        return ((12.0 - 9.0 * B - 6.0 * C) * abs_t * abs_t * abs_t +
                (-18.0 + 12.0 * B + 6.0 * C) * abs_t * abs_t +
                (6.0 - 2.0 * B)) / 6.0;
    } else if (abs_t < 2.0) {
        return ((-B - 6.0 * C) * abs_t * abs_t * abs_t +
                (6.0 * B + 30.0 * C) * abs_t * abs_t +
                (-12.0 * B - 48.0 * C) * abs_t +
                (8.0 * B + 24.0 * C)) / 6.0;
    } else {
        return 0.0;
    }
}


// Sample the texture using cubic filtering
fn cubic_sample(tex: texture_2d<f32>, samp: sampler, uv: vec2<f32>) -> vec4<f32> {
    // Get texture dimensions
    let width = tex_info.x;
    let height = tex_info.y;
    
    // Calculate pixel position in texture
    let pixel = vec2<f32>(uv.x * width, uv.y * height) - 0.5;
    let center = floor(pixel);
    
    // Calculate the fractional offset
    let offset = pixel - center;
    
    // Compute cubic weights
    var w: array<vec4<f32>, 2>; // Changed from 'let' to 'var'
    
    w[0] = vec4<f32>(
        mitchell(1.0 + offset.x),
        mitchell(offset.x),
        mitchell(1.0 - offset.x),
        mitchell(2.0 - offset.x)
    );
    
    w[1] = vec4<f32>(
        mitchell(1.0 + offset.y),
        mitchell(offset.y),
        mitchell(1.0 - offset.y),
        mitchell(2.0 - offset.y)
    );
    
    // Sample 16 texels from the texture
    var color = vec4<f32>(0.0);
    
    for (var y: i32 = -1; y <= 2; y++) {
        let v_weight = w[1][y + 1];
        
        for (var x: i32 = -1; x <= 2; x++) {
            let h_weight = w[0][x + 1];
            
            // Calculate normalized texture coordinates
            let u = (center.x + f32(x) + 0.5) / width;
            let v = (center.y + f32(y) + 0.5) / height;
            
            // Sample the texture and apply weight
            color += textureSampleLevel(tex, samp, vec2<f32>(u, v), 0.0) * h_weight * v_weight;
        }
    }
    
    return color;
}

// Get texture coordinates based on screen position
fn get_uv(pos: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // Convert position to UV coordinates in [0,1] range
    let tex_dims = vec2<f32>(f32(textureDimensions(texture).x), f32(textureDimensions(texture).y));
    let view_dims = vec2<f32>(tex_dims.x / tex_info.z, tex_dims.y / tex_info.w);
    
    // Normalize coordinates based on view dimensions
    let uv = vec2<f32>(
        pos.x / view_dims.x,        // No flipping on X axis
        1.0 - (pos.y / view_dims.y)  // Flip Y axis (WebGPU has Y=0 at top)
    );
    
    // Apply cubic filtering if downsampling
    if (tex_info.z > 1.0 || tex_info.w > 1.0) {
        return cubic_sample(texture, tex_sampler, uv);
    } else {
        return textureSample(texture, tex_sampler, uv);
    }
}