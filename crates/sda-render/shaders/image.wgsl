// Textured quad for one slide layer. The texture holds encoded sRGB color
// with straight alpha. The fragment premultiplies in encoded space, which
// is the renderer wide opacity rule, then converts to linear for sRGB
// targets. The pipeline blends with one, one minus source alpha.
//
// `offset` moves the quad as a fraction of the viewport. `scale` grows or
// shrinks it around the center. `blur` is reserved for transitions.

override srgb_target: u32 = 1u;

struct Params {
    alpha: f32,
    offset: vec2<f32>,
    scale: f32,
    blur: f32,
};

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(1) @binding(0) var<uniform> params: Params;

struct Vertex {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Vertex {
    // Two triangles as a strip: 0 bottom left, 1 bottom right, 2 top left, 3 top right.
    let x = f32(i32(i & 1u) * 2 - 1);
    let y = f32(i32(i >> 1u) * 2 - 1);
    var out: Vertex;
    out.pos = vec4<f32>(vec2<f32>(x, y) * params.scale + params.offset * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
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
fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    let s = textureSample(tex, samp, in.uv);
    let a = s.a * params.alpha;
    return vec4<f32>(to_target(s.rgb * a), a);
}
