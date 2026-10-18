// Textured quad for one slide layer or an image background. The texture
// holds encoded sRGB color with straight alpha. The fragment premultiplies
// in encoded space, which is the renderer wide opacity rule, then converts
// to linear for sRGB targets. The pipeline blends with one, one minus
// source alpha.
//
// `offset` moves the quad as a fraction of the viewport. `scale` grows or
// shrinks it around the center per axis so a bitmap can cover the output.
// `blur` is a radius in texels. Above zero the fragment averages a 9 by 9
// grid of taps spanning that radius with tent weights.

override srgb_target: u32 = 1u;

struct Params {
    alpha: f32,
    blur: f32,
    offset: vec2<f32>,
    scale: vec2<f32>,
    _pad: vec2<f32>,
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

// Premultiplied sample at `uv`.
fn tap(uv: vec2<f32>) -> vec4<f32> {
    let s = textureSampleLevel(tex, samp, uv, 0.0);
    return vec4<f32>(s.rgb * s.a, s.a);
}

@fragment
fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    var c = tap(in.uv);
    if params.blur > 0.0 {
        let step = params.blur / vec2<f32>(textureDimensions(tex)) / 4.0;
        var acc = vec4<f32>(0.0);
        var total = 0.0;
        for (var i = -4; i <= 4; i++) {
            for (var j = -4; j <= 4; j++) {
                let w = (5.0 - abs(f32(i))) * (5.0 - abs(f32(j)));
                acc += tap(in.uv + vec2<f32>(f32(i), f32(j)) * step) * w;
                total += w;
            }
        }
        c = acc / total;
    }
    return vec4<f32>(to_target(c.rgb * params.alpha), c.a * params.alpha);
}
