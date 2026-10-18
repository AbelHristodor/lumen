//! What is live right now and how it is drawn. The Live strip and the
//! projector both read this state and build a `Scene` from it.

use crate::msg::BackgroundAsset;
use image::RgbaImage;
use sda_core::present::Presentation;
use sda_core::resolve::{ResolvedSlide, SlideContent, resolve};
use sda_core::theme::{Background, Rgba, Theme, TransitionKind};
use sda_render::{BackgroundState, FrameSource, Layer, Scene};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct LiveState {
    pub present: Presentation,
    pub theme: Theme,
    /// When the last slide change happened. Drives the transition.
    pub started: Option<Instant>,
    /// The slide on its way out.
    pub outgoing: Option<SlideContent>,
    pub ffmpeg: Option<PathBuf>,
    pub video: Option<Arc<FrameSource>>,
    /// Decoded images for bitmap slides and image backgrounds.
    pub bitmaps: HashMap<PathBuf, Arc<RgbaImage>>,
    /// The background a worker is loading right now. Blocks a second worker
    /// for the same file until the result lands.
    started_for: Option<Background>,
    /// The background whose worker failed. Not retried until the theme
    /// moves to another background.
    failed_for: Option<Background>,
    epoch: Instant,
    /// The outgoing slide resolved once when it left, so it never collides
    /// with the cache of the presentation that replaced it.
    outgoing_layer: Option<Layer>,
    /// Resolved slides by slide index and theme key, so the renderer sees
    /// the same `Arc` every frame and keeps its texture cache warm.
    resolved: Mutex<HashMap<(usize, u64), Arc<ResolvedSlide>>>,
}

impl LiveState {
    pub fn new(theme: Theme) -> Self {
        let mut live = LiveState {
            present: Presentation::default(),
            theme: Theme::default(),
            started: None,
            outgoing: None,
            ffmpeg: None,
            video: None,
            bitmaps: HashMap::new(),
            started_for: None,
            failed_for: None,
            epoch: Instant::now(),
            outgoing_layer: None,
            resolved: Mutex::new(HashMap::new()),
        };
        live.set_theme(theme);
        live
    }

    /// Switch themes. A changed background drops the running video. Image
    /// and video files are loaded by a worker, see `background_pending`.
    pub fn set_theme(&mut self, theme: Theme) {
        if theme.background != self.theme.background {
            self.video = None;
            self.started_for = None;
            self.failed_for = None;
        }
        self.theme = theme;
        self.clear_cache();
    }

    /// True when the background needs a worker: a video with no decoder
    /// and no still yet, or an image not decoded yet. False while a worker
    /// runs for it or after one failed for it.
    pub fn background_pending(&self) -> bool {
        let bg = &self.theme.background;
        if self.started_for.as_ref() == Some(bg) || self.failed_for.as_ref() == Some(bg) {
            return false;
        }
        match bg {
            Background::Video { path } => {
                self.video.is_none()
                    && !self.bitmaps.contains_key(path)
                    && !path.as_os_str().is_empty()
            }
            Background::Image(path) => {
                !self.bitmaps.contains_key(path) && !path.as_os_str().is_empty()
            }
            _ => false,
        }
    }

    /// A worker was started for the current background.
    pub fn background_started(&mut self) {
        self.started_for = Some(self.theme.background.clone());
    }

    /// The worker for `for_background` failed. Ignored when the theme moved
    /// on, else the file is not retried until the background changes.
    pub fn background_failed(&mut self, for_background: &Background) {
        if self.theme.background == *for_background {
            self.started_for = None;
            self.failed_for = Some(for_background.clone());
        }
    }

    /// Put a worker result in place. Ignored when the theme moved on to a
    /// different background while the worker ran.
    pub fn install_background(&mut self, for_background: &Background, asset: BackgroundAsset) {
        if self.theme.background != *for_background {
            return;
        }
        self.started_for = None;
        match asset {
            BackgroundAsset::Video(src) => {
                // Two workers for one path can both land. The first decoder
                // stays and the dropped Arc kills its ffmpeg.
                if self.video.is_none() {
                    self.video = Some(src);
                }
            }
            BackgroundAsset::Still(path, img) => {
                self.bitmaps.insert(path, img);
            }
        }
    }

    /// Reuse `other`'s running decoder when both show the same video, so
    /// one ffmpeg feeds the projector and the Themes preview.
    pub fn adopt_video(&mut self, other: &LiveState) {
        if self.video.is_some() || self.theme.background != other.theme.background {
            return;
        }
        if let (Background::Video { .. }, Some(src)) = (&self.theme.background, &other.video) {
            self.video = Some(src.clone());
        }
    }

    fn clear_cache(&self) {
        if let Ok(mut c) = self.resolved.lock() {
            c.clear();
        }
    }

    /// The current slide and its layer, captured before a change.
    fn snapshot(&self) -> (Option<SlideContent>, Option<Layer>) {
        let slide = self.present.current().cloned();
        let layer = slide
            .as_ref()
            .and_then(|s| self.layer(self.present.index, s));
        (slide, layer)
    }

    /// Remember the previous slide as outgoing and restart the transition.
    fn step(&mut self, prev: (Option<SlideContent>, Option<Layer>)) {
        self.outgoing = prev.0;
        self.outgoing_layer = prev.1;
        self.started = Some(Instant::now());
    }

    pub fn go(&mut self, presentation: Presentation) {
        let prev = self.snapshot();
        self.present = presentation;
        self.clear_cache();
        self.step(prev);
    }

    pub fn next(&mut self) {
        let prev = self.snapshot();
        let before = self.present.index;
        self.present.next();
        if self.present.index != before {
            self.step(prev);
        }
    }

    pub fn prev(&mut self) {
        let prev = self.snapshot();
        let before = self.present.index;
        self.present.prev();
        if self.present.index != before {
            self.step(prev);
        }
    }

    pub fn jump(&mut self, n: usize) {
        let prev = self.snapshot();
        let before = self.present.index;
        self.present.jump(n);
        if self.present.index != before {
            self.step(prev);
        }
    }

    pub fn blank(&mut self) {
        if !self.present.is_loaded() {
            return;
        }
        let prev = self.snapshot();
        self.present.toggle_blank();
        self.step(prev);
    }

    pub fn stop(&mut self) {
        if !self.present.is_loaded() {
            return;
        }
        let prev = self.snapshot();
        self.present = Presentation::default();
        self.clear_cache();
        self.step(prev);
    }

    /// Drop the outgoing slide once the transition has run its course at
    /// `now`, so `animating` turns false and the frame clock can stop.
    pub fn settle(&mut self, now: Instant) {
        if self.started.is_some() && self.progress(now).is_none() {
            self.started = None;
            self.outgoing = None;
            self.outgoing_layer = None;
        }
    }

    /// Progress of the running transition in 0..1, or `None` when settled.
    fn progress(&self, now: Instant) -> Option<f32> {
        let started = self.started?;
        if self.theme.transition.kind == TransitionKind::None {
            return None;
        }
        let duration = self.theme.transition.duration_ms as f32 / 1000.0;
        if duration <= 0.0 {
            return None;
        }
        let elapsed = now.saturating_duration_since(started).as_secs_f32();
        (elapsed < duration).then_some(elapsed / duration)
    }

    /// True while a transition runs or the background moves on its own.
    /// A started transition counts until `settle` clears it on a tick, so
    /// the frame clock always delivers the final frame.
    pub fn animating(&self) -> bool {
        self.started.is_some() || self.theme.background.is_animated()
    }

    fn footer(&self) -> Option<&str> {
        self.theme
            .footer
            .then_some(self.present.title.as_str())
            .filter(|t| !t.is_empty())
    }

    fn layer(&self, index: usize, content: &SlideContent) -> Option<Layer> {
        if let SlideContent::Bitmap(path) = content {
            return self.bitmaps.get(path).map(|b| Layer::Bitmap(b.clone()));
        }
        let key = (index, self.theme.cache_key());
        let mut cache = self.resolved.lock().ok()?;
        let slide = match cache.get(&key) {
            Some(s) => s.clone(),
            None => {
                let s = Arc::new(resolve(&self.theme, content, self.footer())?);
                cache.insert(key, s.clone());
                s
            }
        };
        Some(Layer::Text(slide))
    }

    fn background(&self) -> BackgroundState {
        match &self.theme.background {
            Background::Solid(c) => BackgroundState::Solid(*c),
            Background::Gradient {
                stops,
                angle,
                drift,
            } => BackgroundState::Gradient {
                stops: stops.clone(),
                angle: *angle,
                drift: *drift,
            },
            Background::Particles {
                base,
                color,
                count,
                speed,
            } => BackgroundState::Particles {
                base: *base,
                color: *color,
                count: *count,
                speed: *speed,
            },
            Background::Image(path) => self
                .bitmaps
                .get(path)
                .map(|b| BackgroundState::Image(b.clone()))
                .unwrap_or(BackgroundState::Solid(Rgba::BLACK)),
            Background::Video { path } => self
                .video
                .clone()
                .map(BackgroundState::Video)
                .or_else(|| {
                    self.bitmaps
                        .get(path)
                        .map(|b| BackgroundState::Image(b.clone()))
                })
                .unwrap_or(BackgroundState::Solid(Rgba::BLACK)),
        }
    }

    /// The frame to draw at `now` for a target of `size` pixels.
    pub fn scene(&self, size: (u32, u32), now: Instant) -> Arc<Scene> {
        let current = self
            .present
            .current()
            .and_then(|c| self.layer(self.present.index, c));
        let outgoing = self
            .progress(now)
            .and_then(|p| Some((self.outgoing_layer.clone()?, p)));
        Arc::new(Scene {
            size,
            background: self.background(),
            background_opacity: self.theme.background_opacity,
            current,
            outgoing,
            transition: self.theme.transition,
            blank: self.present.blank,
            now_secs: now.saturating_duration_since(self.epoch).as_secs_f32(),
        })
    }
}
