//! GPU renderer for projected slides. The GUI builds a `Scene` and this crate
//! draws it with wgpu. No UI toolkit types appear here so the same renderer
//! serves the Iced window and the headless tests.

pub mod background;
pub mod composite;
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

/// One cached texture. `source` keeps the layer's `Arc` alive so its address
/// cannot be freed and handed to a new slide while this entry exists. That
/// makes the pointer based cache key safe.
struct Cached {
    group: wgpu::BindGroup,
    #[allow(dead_code)]
    source: Layer,
}

/// Owns every pipeline. Create once per device and reuse across frames.
pub struct Renderer {
    solid: SolidPipeline,
    layer: LayerPipeline,
    text: TextRaster,
    cache: HashMap<u64, Cached>,
    lru: VecDeque<u64>,
    /// Draw list for the prepared frame as uniform slot and cache key.
    draws: Vec<(usize, u64)>,
    /// Whether the solid pass runs under the layers.
    draw_solid: bool,
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
            draws: Vec::new(),
            draw_solid: true,
        }
    }

    /// Upload per frame data. Call once before `render`.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) {
        self.draws.clear();
        self.draw_solid = true;
        match &scene.background {
            BackgroundState::Solid(c) => self.solid.prepare(queue, *c, scene.background_opacity),
            BackgroundState::Image(img) => {
                // The pass already clears to black, the image goes straight on top.
                self.draw_solid = false;
                let mut params = LayerParams::opaque();
                params.alpha = scene.background_opacity.clamp(0.0, 1.0);
                let layer = Layer::Bitmap(img.clone());
                self.queue_draw(
                    device,
                    queue,
                    composite::BACKGROUND,
                    &layer,
                    params,
                    scene.size,
                );
            }
            // Other backgrounds land in later tasks. Draw black until then.
            _ => self.solid.prepare(
                queue,
                sda_core::theme::Rgba::BLACK,
                scene.background_opacity,
            ),
        }
        if scene.blank {
            return;
        }
        let progress = scene.outgoing.as_ref().map_or(1.0, |(_, p)| *p);
        let (out_params, cur_params) = composite::transition(&scene.transition, progress);
        if let (Some((layer, _)), Some(params)) = (&scene.outgoing, out_params) {
            self.queue_draw(
                device,
                queue,
                composite::OUTGOING,
                layer,
                params,
                scene.size,
            );
        }
        if let Some(layer) = &scene.current {
            self.queue_draw(
                device,
                queue,
                composite::CURRENT,
                layer,
                cur_params,
                scene.size,
            );
        }
    }

    /// Record draw commands for the last prepared scene into `view`. One
    /// pass clears to black and draws the background then the layers.
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
        if self.draw_solid {
            self.solid.draw(&mut pass);
        }
        for (slot, key) in &self.draws {
            if let Some(c) = self.cache.get(key) {
                self.layer.draw(&mut pass, &c.group, *slot);
            }
        }
    }

    /// Make sure `layer` has a texture, write `params` into `slot`, and add
    /// the draw to this frame. Bitmaps are scaled to cover the output.
    fn queue_draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
        layer: &Layer,
        params: LayerParams,
        size: (u32, u32),
    ) {
        let params = match layer {
            Layer::Text(_) => params,
            Layer::Bitmap(img) => {
                composite::scaled(params, composite::cover(img.dimensions(), size))
            }
        };
        let key = self.texture(device, queue, layer, size);
        self.layer.set_params(queue, slot, &params);
        self.draws.push((slot, key));
    }

    /// Upload `layer` at `size` unless a texture for it is cached. Returns
    /// the cache key.
    fn texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layer: &Layer,
        size: (u32, u32),
    ) -> u64 {
        let mut h = DefaultHasher::new();
        match layer {
            Layer::Text(s) => {
                (Arc::as_ptr(s) as usize).hash(&mut h);
                // Text is rasterized at the target size so the size is part of the key.
                size.hash(&mut h);
            }
            Layer::Bitmap(b) => (Arc::as_ptr(b) as usize).hash(&mut h),
        }
        let key = h.finish();
        if !self.cache.contains_key(&key) {
            let group = match layer {
                Layer::Text(slide) => {
                    let img = self.text.rasterize(slide, size);
                    self.layer.upload(device, queue, &img)
                }
                Layer::Bitmap(img) => self.layer.upload(device, queue, img),
            };
            self.cache.insert(
                key,
                Cached {
                    group,
                    source: layer.clone(),
                },
            );
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
