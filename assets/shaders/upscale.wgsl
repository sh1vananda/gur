// upscale.wgsl
// Full-screen triangle that copies the 320×180 canvas → window surface
// using nearest-neighbour sampling (pixel-art crispness).

@group(0) @binding(0) var canvas         : texture_2d<f32>;
@group(0) @binding(1) var canvas_sampler : sampler;

struct VertexOutput {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
}

// Single oversized triangle — covers the entire NDC clip space.
// No vertex buffer needed; index drives position + UV directly.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // Positions: three points that form a triangle enclosing [-1..1]x[-1..1]
    var pos_table = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),   // bottom-left
        vec2<f32>( 3.0, -1.0),   // far right
        vec2<f32>(-1.0,  3.0),   // far top
    );
    // UV: maps NDC → [0,1] texture space (Y flipped for wgpu)
    var uv_table = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(pos_table[vi], 0.0, 1.0);
    out.uv       = uv_table[vi];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(canvas, canvas_sampler, in.uv);
}
