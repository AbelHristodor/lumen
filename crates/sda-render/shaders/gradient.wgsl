// Fullscreen linear gradient that drifts back and forth over time.
//
// Each stop is a vec4 with encoded sRGB color in rgb and its position along
// the gradient in a. Stops are sorted by position. `angle` is in radians,
// `phase` is drift times seconds, `opacity` multiplies the encoded color,
// see `Scene::background_opacity`.

override srgb_target: u32 = 1u;

struct Gradient {
    stops: array<vec4<f32>, 8>,
    count: u32,
    angle: f32,
    phase: f32,
    opacity: f32,
};

@group(0) @binding(0) var<uniform> g: Gradient;

struct Vertex {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Vertex {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i >> 1u) * 4 - 1);
    var out: Vertex;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
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

// Smooth triangle wave of `x` with period 1, in 0..1.
fn wave(x: f32) -> f32 {
    let tri = 1.0 - abs(2.0 * fract(x) - 1.0);
    return tri * tri * (3.0 - 2.0 * tri);
}

@fragment
fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    let dir = vec2<f32>(cos(g.angle), sin(g.angle));
    // Project onto the axis so the corners map to 0 and 1, then slide by
    // up to a quarter of the span in each direction.
    let span = abs(dir.x) + abs(dir.y);
    var t = dot(in.uv - 0.5, dir) / span + 0.5;
    t = t + (wave(g.phase) - 0.5) * 0.5;

    var color = g.stops[0].rgb;
    if g.count == 0u {
        color = vec3<f32>(0.0);
    }
    for (var i = 1u; i < g.count; i++) {
        let a = g.stops[i - 1u];
        let b = g.stops[i];
        if t >= a.a {
            let f = clamp((t - a.a) / max(b.a - a.a, 0.0001), 0.0, 1.0);
            color = mix(a.rgb, b.rgb, f);
        }
    }
    return vec4<f32>(to_target(color * g.opacity), 1.0);
}
