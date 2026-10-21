//! Scene description handed to the renderer each frame. The GUI builds a
//! `Scene` from presentation state and the renderer draws it. Nothing here
//! knows about wgpu.

use crate::video::FrameSource;
use image::RgbaImage;
use sda_core::resolve::ResolvedSlide;
use sda_core::theme::{Rgba, Transition};
use std::sync::Arc;

/// Everything needed to draw one frame.
#[derive(Clone, Debug)]
pub struct Scene {
    /// Target size in pixels.
    pub size: (u32, u32),
    pub background: BackgroundState,
    /// 0 hides the background, 1 shows it at full strength.
    ///
    /// Renderer wide rule: opacity multiplies the encoded sRGB color before
    /// any conversion to linear, so half opacity over black stores half the
    /// byte value. Every pass applies opacity and alpha this same way.
    pub background_opacity: f32,
    pub current: Option<Layer>,
    /// The slide on its way out and its transition progress in 0..1.
    pub outgoing: Option<(Layer, f32)>,
    pub transition: Transition,
    pub blank: bool,
    pub now_secs: f32,
}

/// One drawable slide.
#[derive(Clone, Debug)]
pub enum Layer {
    Text(Arc<ResolvedSlide>),
    Bitmap(Arc<RgbaImage>),
    /// Live frames from a player. Drawn letterboxed over the background.
    Video(Arc<FrameSource>),
}

/// What sits behind the slide.
#[derive(Clone, Debug)]
pub enum BackgroundState {
    Solid(Rgba),
    Gradient {
        stops: Vec<(f32, Rgba)>,
        angle: f32,
        drift: f32,
    },
    Particles {
        base: Rgba,
        color: Rgba,
        count: u32,
        speed: f32,
    },
    Image(Arc<RgbaImage>),
    Video(Arc<FrameSource>),
}

/// Pixel rectangle in target space. Used both for the viewport a scene is
/// mapped into and for the scissor that limits the pixels it may touch. The
/// origin is signed because a scrolled widget can start above or left of the
/// target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Viewport {
    /// This rectangle cut to `0..w` by `0..h`. `None` when nothing is left.
    pub fn clamp(self, w: u32, h: u32) -> Option<Viewport> {
        let x0 = self.x.max(0);
        let y0 = self.y.max(0);
        let x1 = (self.x as i64 + self.w as i64).min(w as i64);
        let y1 = (self.y as i64 + self.h as i64).min(h as i64);
        if x1 <= x0 as i64 || y1 <= y0 as i64 {
            return None;
        }
        Some(Viewport {
            x: x0,
            y: y0,
            w: (x1 - x0 as i64) as u32,
            h: (y1 - y0 as i64) as u32,
        })
    }
}
