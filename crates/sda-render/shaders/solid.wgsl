// Fullscreen solid color. The vertex stage emits one triangle that covers
// the clip space square. The fragment stage writes the uniform color.
//
// The color arrives as encoded sRGB bytes scaled to 0..1 plus an opacity.
// Opacity is applied in encoded space so half opacity reads as half
// brightness in a saved frame. The result is converted to linear because
// the target is an sRGB format and the hardware encodes it on store.

struct Solid {
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> solid: Solid;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    let dimmed = solid.color.rgb * solid.color.a;
    return vec4<f32>(srgb_to_linear(dimmed), 1.0);
}
