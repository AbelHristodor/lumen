//! Pure math for compositing layers. Turns the scene's transition state into
//! `LayerParams` for the outgoing and current layers and computes the cover
//! scale for bitmaps. No wgpu here so it is unit tested without a GPU.

use crate::layer::LayerParams;
use sda_core::theme::{Direction, Transition, TransitionKind};

/// Uniform slot of the image background.
pub const BACKGROUND: usize = 0;
/// Uniform slot of the layer on its way out.
pub const OUTGOING: usize = 1;
/// Uniform slot of the layer being shown.
pub const CURRENT: usize = 2;

/// Blur radius in texels at the strongest point of a blur transition.
const BLUR_PX: f32 = 8.0;

/// Params for the outgoing and the current layer at raw `progress` in 0..1.
/// `None` for the outgoing layer means it is not drawn.
pub fn transition(t: &Transition, progress: f32) -> (Option<LayerParams>, LayerParams) {
    let p = t.easing.apply(progress.clamp(0.0, 1.0));
    let mut out = LayerParams::opaque();
    let mut cur = LayerParams::opaque();
    match t.kind {
        TransitionKind::None => return (None, cur),
        TransitionKind::Fade => {
            out.alpha = 1.0 - p;
            cur.alpha = p;
        }
        TransitionKind::Slide(dir) => {
            let (dx, dy) = match dir {
                Direction::Left => (-1.0, 0.0),
                Direction::Right => (1.0, 0.0),
                Direction::Up => (0.0, 1.0),
                Direction::Down => (0.0, -1.0),
            };
            // Outgoing leaves toward `dir`, current enters from the far side.
            out.offset = [dx * p, dy * p];
            cur.offset = [-dx * (1.0 - p), -dy * (1.0 - p)];
        }
        TransitionKind::Scale => {
            out.scale = [1.0 + 0.2 * p; 2];
            out.alpha = 1.0 - p;
            cur.scale = [0.8 + 0.2 * p; 2];
            cur.alpha = p;
        }
        TransitionKind::Blur => {
            out.blur = p * BLUR_PX;
            out.alpha = 1.0 - p;
            cur.blur = (1.0 - p) * BLUR_PX;
            cur.alpha = p;
        }
    }
    (Some(out), cur)
}

/// Per axis quad scale that keeps the aspect of a `tex` sized image while
/// covering an `out` sized viewport. Equal aspects give 1 on both axes.
pub fn cover(tex: (u32, u32), out: (u32, u32)) -> [f32; 2] {
    let ta = tex.0 as f32 / tex.1.max(1) as f32;
    let oa = out.0 as f32 / out.1.max(1) as f32;
    if ta > oa {
        [ta / oa, 1.0]
    } else {
        [1.0, oa / ta]
    }
}

/// Scale factors that fit `tex` inside `out` with its aspect kept. The
/// shorter side stays at 1.0, the other shrinks. The opposite of `cover`.
pub fn contain(tex: (u32, u32), out: (u32, u32)) -> [f32; 2] {
    let ta = tex.0 as f32 / tex.1.max(1) as f32;
    let oa = out.0 as f32 / out.1.max(1) as f32;
    if ta > oa {
        [1.0, oa / ta]
    } else {
        [ta / oa, 1.0]
    }
}

/// How a bitmap meets the output: fill it and crop, or fit inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Cover,
    Contain,
}

/// Multiply the quad scale of `params` by `by` per axis.
pub fn scaled(mut params: LayerParams, by: [f32; 2]) -> LayerParams {
    params.scale = [params.scale[0] * by[0], params.scale[1] * by[1]];
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use sda_core::theme::Easing;

    fn linear(kind: TransitionKind) -> Transition {
        Transition {
            kind,
            duration_ms: 400,
            easing: Easing::Linear,
        }
    }

    #[test]
    fn fade_half_splits_alpha() {
        let (out, cur) = transition(&linear(TransitionKind::Fade), 0.5);
        assert_eq!(out.unwrap().alpha, 0.5);
        assert_eq!(cur.alpha, 0.5);
    }

    #[test]
    fn slide_left_moves_both() {
        let (out, cur) = transition(&linear(TransitionKind::Slide(Direction::Left)), 0.25);
        assert_eq!(out.unwrap().offset, [-0.25, 0.0]);
        assert_eq!(cur.offset, [0.75, 0.0]);
    }

    #[test]
    fn none_draws_current_only() {
        let (out, cur) = transition(&linear(TransitionKind::None), 0.5);
        assert!(out.is_none());
        assert_eq!(cur, LayerParams::opaque());
    }

    #[test]
    fn cover_scales_the_short_axis() {
        assert_eq!(cover((64, 36), (320, 180)), [1.0, 1.0]);
        let [x, y] = cover((8, 8), (320, 180));
        assert_eq!(x, 1.0);
        assert!((y - 320.0 / 180.0).abs() < 1e-5);
        let [x, y] = cover((40, 10), (20, 10));
        assert_eq!(y, 1.0);
        assert_eq!(x, 2.0);
    }

    #[test]
    fn contain_fits_inside_and_cover_fills() {
        // 4:3 into 16:9: width shrinks to 0.75, height stays.
        let c = contain((400, 300), (1600, 900));
        assert!((c[0] - 0.75).abs() < 1e-5 && (c[1] - 1.0).abs() < 1e-5);
        // 21:9 into 16:9: height shrinks.
        let c = contain((2100, 900), (1600, 900));
        assert!((c[0] - 1.0).abs() < 1e-5 && (c[1] - 16.0 / 21.0).abs() < 1e-5);
        // Same aspect: no change.
        assert_eq!(contain((16, 9), (1920, 1080)), [1.0, 1.0]);
    }
}
