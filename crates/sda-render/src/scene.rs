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

/// Pixel rectangle inside the target the renderer draws into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}
