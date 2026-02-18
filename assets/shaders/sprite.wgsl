// sprite.wgsl
// Draws textured, tinted quads to the 320×180 pixel-art canvas.

// ─── Bind Group 0: Camera ────────────────────────────────────────────────────
struct Globals {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> globals: Globals;

// ─── Bind Group 1: Sprite Texture ────────────────────────────────────────────
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

// ─── Vertex ──────────────────────────────────────────────────────────────────
struct VertexInput {
    @location(0) position : vec2<f32>,
    @location(1) uv       : vec2<f32>,
    @location(2) color    : vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
    @location(1)       color    : vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = globals.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.uv       = in.uv;
    out.color    = in.color;
    return out;
}

// ─── Fragment ────────────────────────────────────────────────────────────────
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(sprite_texture, sprite_sampler, in.uv);
    // Punch-through alpha — discard fully transparent pixels
    if sample.a < 0.01 {
        discard;
    }
    return sample * in.color;
}
