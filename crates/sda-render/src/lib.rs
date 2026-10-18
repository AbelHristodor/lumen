//! GPU renderer for projected slides. The GUI builds a `Scene` and this crate
//! draws it with wgpu. No UI toolkit types appear here so the same renderer
//! serves the Iced window and the headless tests.

pub mod background;
pub mod headless;
pub mod scene;
pub mod video;

pub use scene::{BackgroundState, Layer, Scene, Viewport};
pub use video::FrameSource;

use background::SolidPipeline;

/// Owns every pipeline. Create once per device and reuse across frames.
pub struct Renderer {
    solid: SolidPipeline,
}

impl Renderer {
    /// `format` is the format of the texture views later passed to `render`.
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Renderer {
            solid: SolidPipeline::new(device, format),
        }
    }

    /// Upload per frame data. Call once before `render`.
    pub fn prepare(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) {
        let color = match &scene.background {
            BackgroundState::Solid(c) => *c,
            // Other backgrounds land in later tasks. Draw black until then.
            _ => sda_core::theme::Rgba::BLACK,
        };
        self.solid.prepare(queue, color, scene.background_opacity);
    }

    /// Record draw commands for the last prepared scene into `view`.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: Viewport,
    ) {
        self.solid.render(encoder, view, viewport);
    }
}
