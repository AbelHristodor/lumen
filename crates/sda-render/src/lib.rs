//! GPU renderer for projected slides. The GUI builds a `Scene` and this crate
//! draws it with wgpu. No UI toolkit types appear here so the same renderer
//! serves the Iced window and the headless tests.
//!
//! One `Renderer` draws many scenes per frame. Each scene is prepared under a
//! caller chosen key and rendered into its own viewport of a target that
//! already holds the rest of the frame.

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
use layer::{LayerParams, LayerPipeline, SLOTS};
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use text::TextRaster;

/// How many uploaded layer textures stay resident. The Present tab shows
/// the preview, 12 thumbnails, the theme picker, and the live strip at once,
/// so the cap must hold more than one scene's worth of layers.
const LAYER_CACHE_CAP: usize = 48;

/// A prepared scene not prepared again within this many `trim` calls is
/// dropped.
const KEEP_GENERATIONS: u64 = 2;

/// Shader override that tells a pipeline whether the target format is sRGB.
/// Shaders convert encoded color to linear only when it is.
pub(crate) fn srgb_constant(format: wgpu::TextureFormat) -> [(&'static str, f64); 1] {
    [("srgb_target", if format.is_srgb() { 1.0 } else { 0.0 })]
}

/// One uniform buffer with its bind group. Every prepared scene owns its own
/// slots so writes for one scene never clobber another in the same frame.
pub struct UniformSlot {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) group: wgpu::BindGroup,
}

impl UniformSlot {
    pub(crate) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        size: u64,
        label: &str,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        UniformSlot { buffer, group }
    }
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
    /// Nothing under the image or video quad.
    None,
    Solid,
    Gradient,
    /// Solid base then the particle quads.
    Particles,
}

/// Everything `render` needs for one scene key.
struct Prepared {
    /// Generation of the last `prepare` for this key.
    generation: u64,
    bg: BgPass,
    solid: UniformSlot,
    gradient: UniformSlot,
    particles: UniformSlot,
    particle_count: u32,
    layers: [UniformSlot; SLOTS],
    /// Draw list as layer slot index and texture cache key.
    draws: Vec<(usize, u64)>,
    /// Cache key of the video frame this scene drew last. Replaced frames
    /// are evicted at `trim`, not here, because another scene prepared this
    /// frame may still draw the old one.
    video_key: Option<u64>,
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
    scenes: HashMap<u64, Prepared>,
    /// Bumped once per `trim`, which Iced calls at the end of every frame.
    generation: u64,
    /// Video frame textures to evict at `trim`.
    stale: Vec<u64>,
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
            scenes: HashMap::new(),
            generation: 0,
            stale: Vec::new(),
        }
    }

    fn new_prepared(&self, device: &wgpu::Device) -> Prepared {
        Prepared {
            generation: self.generation,
            bg: BgPass::Solid,
            solid: self.solid.slot(device),
            gradient: self.gradient.slot(device),
            particles: self.particles.slot(device),
            particle_count: 0,
            layers: self.layer.slots(device),
            draws: Vec::new(),
            video_key: None,
        }
    }

    /// Upload per frame data for the scene under `key`. Call once per scene
    /// before `render`. Keys are chosen by the caller and must differ for
    /// scenes drawn in the same frame.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, key: u64, scene: &Scene) {
        // Take the entry out so the helpers can borrow `self` and it at once.
        let mut p = self
            .scenes
            .remove(&key)
            .unwrap_or_else(|| self.new_prepared(device));
        p.generation = self.generation;
        p.draws.clear();
        let opacity = scene.background_opacity.clamp(0.0, 1.0);
        // A key that stops showing video no longer pins its last frame.
        if !matches!(scene.background, BackgroundState::Video(_))
            && let Some(old) = p.video_key.take()
        {
            self.stale.push(old);
        }
        p.bg = match &scene.background {
            BackgroundState::Solid(c) => {
                self.solid.prepare(queue, &p.solid, *c, opacity);
                BgPass::Solid
            }
            BackgroundState::Gradient {
                stops,
                angle,
                drift,
            } => {
                self.gradient.prepare(
                    queue,
                    &p.gradient,
                    stops,
                    *angle,
                    drift * scene.now_secs,
                    opacity,
                );
                BgPass::Gradient
            }
            BackgroundState::Particles {
                base,
                color,
                count,
                speed,
            } => {
                self.solid.prepare(queue, &p.solid, *base, opacity);
                self.particles.prepare(
                    queue,
                    &p.particles,
                    *color,
                    *speed,
                    scene.now_secs,
                    scene.size,
                    opacity,
                );
                p.particle_count = *count;
                BgPass::Particles
            }
            BackgroundState::Image(img) => {
                self.background_image(device, queue, &mut p, img.clone(), opacity, scene.size);
                BgPass::None
            }
            BackgroundState::Video(src) => match src.latest() {
                Some(frame) => {
                    let key =
                        self.background_image(device, queue, &mut p, frame, opacity, scene.size);
                    if let Some(old) = p.video_key.replace(key)
                        && old != key
                    {
                        self.stale.push(old);
                    }
                    BgPass::None
                }
                None => {
                    self.solid
                        .prepare(queue, &p.solid, sda_core::theme::Rgba::BLACK, opacity);
                    BgPass::Solid
                }
            },
        };
        if !scene.blank {
            let progress = scene.outgoing.as_ref().map_or(1.0, |(_, p)| *p);
            let (out_params, cur_params) = composite::transition(&scene.transition, progress);
            if let (Some((layer, _)), Some(params)) = (&scene.outgoing, out_params) {
                self.queue_draw(
                    device,
                    queue,
                    &mut p,
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
                    &mut p,
                    composite::CURRENT,
                    layer,
                    cur_params,
                    scene.size,
                );
            }
        }
        self.scenes.insert(key, p);
    }

    /// End of frame housekeeping. Drops scenes not prepared for
    /// `KEEP_GENERATIONS` frames and evicts replaced video frames.
    pub fn trim(&mut self) {
        self.generation += 1;
        let keep_from = self.generation.saturating_sub(KEEP_GENERATIONS);
        let stale = &mut self.stale;
        self.scenes.retain(|_, p| {
            let keep = p.generation >= keep_from;
            if !keep {
                stale.extend(p.video_key);
            }
            keep
        });
        for old in self.stale.drain(..) {
            self.cache.remove(&old);
            self.lru.retain(|k| *k != old);
        }
    }

    /// Queue `img` in the background slot at `opacity`, covering the output.
    /// Nothing draws under it. Returns the cache key.
    fn background_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        p: &mut Prepared,
        img: Arc<image::RgbaImage>,
        opacity: f32,
        size: (u32, u32),
    ) -> u64 {
        let mut params = LayerParams::opaque();
        params.alpha = opacity;
        self.queue_draw(
            device,
            queue,
            p,
            composite::BACKGROUND,
            &Layer::Bitmap(img),
            params,
            size,
        )
    }

    /// Record draw commands for the scene prepared under `key` into `view`
    /// inside `vp`. The pass loads what the target already holds, so the
    /// rest of the frame outside `vp` stays as it was. Inside `vp` the
    /// background fills every pixel. A key that was never prepared draws
    /// nothing.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        key: u64,
        vp: Viewport,
    ) {
        let Some(p) = self.scenes.get(&key) else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.w as f32, vp.h as f32, 0.0, 1.0);
        pass.set_scissor_rect(vp.x, vp.y, vp.w, vp.h);
        match p.bg {
            BgPass::None => {}
            BgPass::Solid => self.solid.draw(&mut pass, &p.solid),
            BgPass::Gradient => self.gradient.draw(&mut pass, &p.gradient),
            BgPass::Particles => {
                self.solid.draw(&mut pass, &p.solid);
                self.particles
                    .draw(&mut pass, &p.particles, p.particle_count);
            }
        }
        for (slot, key) in &p.draws {
            if let Some(c) = self.cache.get(key) {
                self.layer.draw(&mut pass, &c.group, &p.layers[*slot]);
            }
        }
    }

    /// Make sure `layer` has a texture, write `params` into the layer
    /// `slot` of `p`, and add the draw to `p`. Bitmaps are scaled to cover
    /// the output. Returns the cache key.
    #[allow(clippy::too_many_arguments)]
    fn queue_draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        p: &mut Prepared,
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
        self.layer.set_params(queue, &p.layers[slot], &params);
        p.draws.push((slot, key));
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
