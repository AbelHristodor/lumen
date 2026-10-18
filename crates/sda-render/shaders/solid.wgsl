// Fullscreen solid color. The vertex stage emits one triangle that covers
// the clip space square. The fragment stage writes the uniform color.
//
// The color arrives as encoded sRGB bytes scaled to 0..1 plus an opacity.
// Opacity multiplies the encoded color, see `Scene::background_opacity`.
// When the target is an sRGB format the result is converted to linear
// because the hardware encodes it again on store.

override srgb_target: u32 = 1u;

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

fn to_target(c: vec3<f32>) -> vec3<f32> {
    if srgb_target == 0u {
        return c;
    }
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    let dimmed = solid.color.rgb * solid.color.a;
    return vec4<f32>(to_target(dimmed), 1.0);
}
