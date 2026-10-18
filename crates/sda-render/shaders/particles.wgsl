// Floating points of light drawn over the base color pass. Each particle is
// a small quad of two triangles. The vertex stage hashes the particle index
// into a seed for its start position, size, and speed, then moves it up the
// screen with time and wraps. The fragment draws a soft circle. The pipeline
// blends additively so overlapping points brighten.
//
// `color` holds encoded sRGB in rgb and the background opacity in a, which
// multiplies the encoded color, see `Scene::background_opacity`.

override srgb_target: u32 = 1u;

struct Particles {
    color: vec4<f32>,
    viewport: vec2<f32>,
    speed: f32,
    time: f32,
};

@group(0) @binding(0) var<uniform> p: Particles;

struct Vertex {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
};

// Integer hash from PCG.
fn hash(n: u32) -> u32 {
    var x = n * 747796405u + 2891336453u;
    x = ((x >> ((x >> 28u) + 4u)) ^ x) * 277803737u;
    return (x >> 22u) ^ x;
}

fn rnd(n: u32) -> f32 {
    return f32(hash(n)) / 4294967295.0;
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Vertex {
    let id = i / 6u;
    let corner = i % 6u;
    // Two triangles over the corners of a square: 0 1 2 and 1 3 2.
    var table = array<u32, 6>(0u, 1u, 2u, 1u, 3u, 2u);
    let ci = table[corner];
    let local = vec2<f32>(f32(ci & 1u), f32(ci >> 1u)) * 2.0 - 1.0;

    let seed = vec2<f32>(rnd(id * 4u), rnd(id * 4u + 1u));
    let size_px = 2.0 + 3.0 * rnd(id * 4u + 2u);
    let rate = 0.5 + 0.5 * rnd(id * 4u + 3u);
    // Rise by a quarter of the screen per second at speed 1.
    let center = fract(seed + vec2<f32>(0.0, p.speed * p.time * 0.25 * rate));
    let half = size_px / p.viewport;
    var out: Vertex;
    out.pos = vec4<f32>(center * 2.0 - 1.0 + local * half, 0.0, 1.0);
    out.local = local;
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
    let a = smoothstep(1.0, 0.5, length(in.local)) * p.color.a;
    return vec4<f32>(to_target(p.color.rgb * a), a);
}
