//! Metal shader source — vertex and fragment shaders for terminal rendering.
//!
//! Two pipeline variants:
//! 1. **Solid**: Background colors, cursor, selection overlays (no texture sampling)
//! 2. **Textured**: Glyph rendering (samples atlas texture, applies foreground color)
//!
//! Both use instanced rendering — one draw call per pass.

/// MSL (Metal Shading Language) source for all terminal render passes.
///
/// Compiled at runtime via `MTLDevice::newLibraryWithSource`.
pub const SHADER_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

// Per-instance data shared between solid and textured passes.
struct Instance {
    // Grid position in pixels (top-left of quad).
    float2 position;
    // Quad size in pixels.
    float2 size;
    // Atlas UV origin and size (only used by textured pass).
    float4 uv_rect;   // (u, v, u_width, v_height)
    // Color (RGBA, 0.0-1.0).
    float4 color;
};

// Uniforms: projection from pixel coordinates to clip space.
struct Uniforms {
    float2 viewport_size;
};

// Vertex output for solid pass.
struct SolidVertex {
    float4 position [[position]];
    float4 color;
};

// Vertex output for textured pass.
struct TexturedVertex {
    float4 position [[position]];
    float2 uv;
    float4 color;
};

// Unit quad vertices (two triangles).
constant float2 quad_vertices[] = {
    float2(0.0, 0.0), float2(1.0, 0.0), float2(0.0, 1.0),
    float2(1.0, 0.0), float2(1.0, 1.0), float2(0.0, 1.0),
};

// Convert pixel position to clip space (-1 to 1, Y flipped for Metal).
float4 pixel_to_clip(float2 pixel_pos, float2 viewport) {
    float2 ndc = pixel_pos / viewport * 2.0 - 1.0;
    return float4(ndc.x, -ndc.y, 0.0, 1.0);
}

// === Solid Pass (backgrounds, cursor, selection) ===

vertex SolidVertex solid_vertex(
    uint vid [[vertex_id]],
    uint iid [[instance_id]],
    const device Instance* instances [[buffer(0)]],
    constant Uniforms& uniforms [[buffer(1)]]
) {
    Instance inst = instances[iid];
    float2 local = quad_vertices[vid];
    float2 pixel_pos = inst.position + local * inst.size;

    SolidVertex out;
    out.position = pixel_to_clip(pixel_pos, uniforms.viewport_size);
    out.color = inst.color;
    return out;
}

fragment float4 solid_fragment(SolidVertex in [[stage_in]]) {
    return in.color;
}

// === Textured Pass (glyphs) ===

vertex TexturedVertex text_vertex(
    uint vid [[vertex_id]],
    uint iid [[instance_id]],
    const device Instance* instances [[buffer(0)]],
    constant Uniforms& uniforms [[buffer(1)]]
) {
    Instance inst = instances[iid];
    float2 local = quad_vertices[vid];
    float2 pixel_pos = inst.position + local * inst.size;

    TexturedVertex out;
    out.position = pixel_to_clip(pixel_pos, uniforms.viewport_size);
    out.uv = inst.uv_rect.xy + local * inst.uv_rect.zw;
    out.color = inst.color;
    return out;
}

fragment float4 text_fragment(
    TexturedVertex in [[stage_in]],
    texture2d<float> atlas [[texture(0)]]
) {
    constexpr sampler atlas_sampler(mag_filter::linear, min_filter::linear);
    float4 tex = atlas.sample(atlas_sampler, in.uv);

    // For mask glyphs: tex is (1,1,1,alpha), apply foreground color
    // For color glyphs: tex is already colored, use directly
    // We blend: output = fg_color * tex_alpha (works for both)
    float4 result;
    result.rgb = in.color.rgb * tex.a + tex.rgb * (1.0 - tex.a);
    result.a = tex.a;
    return result;
}
"#;

/// Names of shader functions.
pub const SOLID_VERTEX_FN: &str = "solid_vertex";
pub const SOLID_FRAGMENT_FN: &str = "solid_fragment";
pub const TEXT_VERTEX_FN: &str = "text_vertex";
pub const TEXT_FRAGMENT_FN: &str = "text_fragment";

/// Per-instance data matching the MSL `Instance` struct layout.
/// Must be `#[repr(C)]` for GPU buffer compatibility.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Instance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_rect: [f32; 4],
    pub color: [f32; 4],
}

/// Uniform data matching the MSL `Uniforms` struct.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Uniforms {
    pub viewport_size: [f32; 2],
}
