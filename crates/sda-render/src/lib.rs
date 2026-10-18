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
pub use video::{FrameSource, spawn_decoder};

use background::{GradientPipeline, ParticlesPipeline, SolidPipeline};
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

/// Which background passes run under the layers this frame.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BgPass {
    /// Only the clear to black, the image or video quad sits on top.
    None,
    Solid,
    Gradient,
    /// Solid base then the particle quads.
    Particles,
}

/// Owns every pipeline. Create once per device and reuse across frames.
pub struct Renderer {
    solid: SolidPipeline,
    gradient: GradientPipeline,
    particles: ParticlesPipeline,
    layer: LayerPipeline,
    text: TextRaster,
    cache: HashMap<u64, Cached>,
    lru: VecDeque<u64>,
    /// Draw list for the prepared frame as uniform slot and cache key.
    draws: Vec<(usize, u64)>,
    bg: BgPass,
    /// Cache key of the last uploaded video frame so it can be evicted when
    /// the next frame arrives instead of filling the LRU.
    video_key: Option<u64>,
}

impl Renderer {
    /// `format` is the format of the texture views later passed to `render`.
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Renderer {
            solid: SolidPipeline::new(device, format),
            gradient: GradientPipeline::new(device, format),
            particles: ParticlesPipeline::new(device, format),
            layer: LayerPipeline::new(device, format),
            text: TextRaster::new(),
            cache: HashMap::new(),
            lru: VecDeque::new(),
            draws: Vec::new(),
            bg: BgPass::Solid,
            video_key: None,
        }
    }

    /// Upload per frame data. Call once before `render`.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) {
        self.draws.clear();
        let opacity = scene.background_opacity.clamp(0.0, 1.0);
        self.bg = match &scene.background {
            BackgroundState::Solid(c) => {
                self.solid.prepare(queue, *c, opacity);
                BgPass::Solid
            }
            BackgroundState::Gradient {
                stops,
                angle,
                drift,
            } => {
                self.gradient
                    .prepare(queue, stops, *angle, drift * scene.now_secs, opacity);
                BgPass::Gradient
            }
            BackgroundState::Particles {
                base,
                color,
                count,
                speed,
            } => {
                self.solid.prepare(queue, *base, opacity);
                self.particles.prepare(
                    queue,
                    *color,
                    *count,
                    *speed,
                    scene.now_secs,
                    scene.size,
                    opacity,
                );
                BgPass::Particles
            }
            BackgroundState::Image(img) => {
                self.background_image(device, queue, img.clone(), opacity, scene.size);
                BgPass::None
            }
            BackgroundState::Video(src) => match src.latest() {
                Some(frame) => {
                    // One frame lives in the cache at a time.
                    let key = self.background_image(device, queue, frame, opacity, scene.size);
                    if let Some(old) = self.video_key.replace(key)
                        && old != key
                    {
                        self.cache.remove(&old);
                        self.lru.retain(|k| *k != old);
                    }
                    BgPass::None
                }
                None => {
                    self.solid
                        .prepare(queue, sda_core::theme::Rgba::BLACK, opacity);
                    BgPass::Solid
                }
            },
        };
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

    /// Queue `img` in the background slot at `opacity`, covering the output.
    /// The pass already clears to black so no solid pass runs under it.
    fn background_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: Arc<image::RgbaImage>,
        opacity: f32,
        size: (u32, u32),
    ) -> u64 {
        let mut params = LayerParams::opaque();
        params.alpha = opacity;
        self.queue_draw(
            device,
            queue,
            composite::BACKGROUND,
            &Layer::Bitmap(img),
            params,
            size,
        )
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
        match self.bg {
            BgPass::None => {}
            BgPass::Solid => self.solid.draw(&mut pass),
            BgPass::Gradient => self.gradient.draw(&mut pass),
            BgPass::Particles => {
                self.solid.draw(&mut pass);
                self.particles.draw(&mut pass);
            }
        }
        for (slot, key) in &self.draws {
            if let Some(c) = self.cache.get(key) {
                self.layer.draw(&mut pass, &c.group, *slot);
            }
        }
    }

    /// Make sure `layer` has a texture, write `params` into `slot`, and add
    /// the draw to this frame. Bitmaps are scaled to cover the output.
    /// Returns the cache key.
    fn queue_draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
        layer: &Layer,
        params: LayerParams,
        size: (u32, u32),
    ) -> u64 {
        let params = match layer {
            Layer::Text(_) => params,
            Layer::Bitmap(img) => {
                composite::scaled(params, composite::cover(img.dimensions(), size))
            }
        };
        let key = self.texture(device, queue, layer, size);
        self.layer.set_params(queue, slot, &params);
        self.draws.push((slot, key));
        key
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
