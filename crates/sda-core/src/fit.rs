//! Pure binary search for the largest font size whose measured text fits.
pub fn fit_size(
    mut measure: impl FnMut(f32) -> (f32, f32),
    bounds: (f32, f32),
    min: f32,
    max: f32,
) -> f32 {
    let fits = |m: (f32, f32)| m.0 <= bounds.0 && m.1 <= bounds.1;
    if fits(measure(max)) {
        return max;
    }
    if !fits(measure(min)) {
        return min;
    }
    let (mut lo, mut hi) = (min, max);
    while hi - lo > 0.5 {
        let mid = (lo + hi) / 2.0;
        if fits(measure(mid)) {
            lo = mid
        } else {
            hi = mid
        }
    }
    lo
}
