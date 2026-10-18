//! GPU renderer for projected slides. The GUI builds a `Scene` and this crate
//! draws it with wgpu. No UI toolkit types appear here so the same renderer
//! serves the Iced window and the headless tests.

pub mod background;
pub mod fonts;
pub mod headless;
pub mod layer;
pub mod scene;
pub mod text;
pub mod video;

pub use fonts::fonts;
pub use scene::{BackgroundState, Layer, Scene, Viewport};
pub use video::FrameSource;

use background::SolidPipeline;
use layer::{LayerParams, LayerPipeline};
use sda_core::resolve::ResolvedSlide;
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use text::TextRaster;

/// How many uploaded layer textures stay resident.
const LAYER_CACHE_CAP: usize = 16;

/// Shader override that tells a pipeline whether the target format is sRGB.
/// Shaders convert encoded color to linear only when it is.
pub(crate) fn srgb_constant(format: wgpu::TextureFormat) -> [(&'static str, f64); 1] {
    [("srgb_target", if format.is_srgb() { 1.0 } else { 0.0 })]
}

/// Owns every pipeline. Create once per device and reuse across frames.
pub struct Renderer {
    solid: SolidPipeline,
    layer: LayerPipeline,
    text: TextRaster,
    cache: HashMap<u64, wgpu::BindGroup>,
    lru: VecDeque<u64>,
    current: Option<u64>,
}

impl Renderer {
    /// `format` is the format of the texture views later passed to `render`.
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Renderer {
            solid: SolidPipeline::new(device, format),
            layer: LayerPipeline::new(device, format),
            text: TextRaster::new(),
            cache: HashMap::new(),
            lru: VecDeque::new(),
            current: None,
        }
    }

    /// Upload per frame data. Call once before `render`.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) {
        let color = match &scene.background {
            BackgroundState::Solid(c) => *c,
            // Other backgrounds land in later tasks. Draw black until then.
            _ => sda_core::theme::Rgba::BLACK,
        };
        self.solid.prepare(queue, color, scene.background_opacity);
        self.current = match &scene.current {
            Some(Layer::Text(slide)) => Some(self.text_layer(device, queue, slide, scene.size)),
            // Bitmap layers arrive with the pptx renderer wiring.
            _ => None,
        };
        self.layer.set_params(queue, &LayerParams::opaque());
    }

    /// Record draw commands for the last prepared scene into `view`. One
    /// pass clears to black and draws the background then the layer.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        vp: Viewport,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.w as f32, vp.h as f32, 0.0, 1.0);
        pass.set_scissor_rect(vp.x, vp.y, vp.w, vp.h);
        self.solid.draw(&mut pass);
        if let Some(texture) = self.current.and_then(|k| self.cache.get(&k)) {
            self.layer.draw(&mut pass, texture);
        }
    }

    /// Rasterize `slide` at `size` unless a texture for it is cached.
    /// Returns the cache key.
    fn text_layer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slide: &Arc<ResolvedSlide>,
        size: (u32, u32),
    ) -> u64 {
        // ponytail: keyed by pointer, a freed and reused Arc address could
        // alias a stale texture. Add a content hash if that ever shows.
        let mut h = DefaultHasher::new();
        (Arc::as_ptr(slide) as usize).hash(&mut h);
        size.hash(&mut h);
        let key = h.finish();
        if !self.cache.contains_key(&key) {
            let img = self.text.rasterize(slide, size);
            self.cache
                .insert(key, self.layer.upload(device, queue, &img));
        }
        self.lru.retain(|k| *k != key);
        self.lru.push_back(key);
        if self.lru.len() > LAYER_CACHE_CAP
            && let Some(old) = self.lru.pop_front()
        {
            self.cache.remove(&old);
        }
        key
    }
}
