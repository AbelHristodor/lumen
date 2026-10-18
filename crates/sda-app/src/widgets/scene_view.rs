//! Iced shader widget that draws a `Scene` through `sda_render`.
//!
//! Iced keeps one `ScenePipeline` per window set and calls `prepare` for
//! every `scene_view` on screen before any `render`. The `key` tells the
//! shared `Renderer` which prepared scene each widget draws, so every call
//! site on one screen must pass a different stable key.

use iced::advanced::graphics::Viewport;
use iced::widget::shader;
use iced::{Element, Fill, Rectangle, mouse};
use sda_render::{Renderer, Scene};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// Draw `scene` under `key`. Build the key with [`key`].
pub fn scene_view<'a, M: 'a>(key: u64, scene: Arc<Scene>) -> Element<'a, M> {
    shader(SceneProgram { key, scene })
        .width(Fill)
        .height(Fill)
        .into()
}

/// Stable scene key from anything hashable, such as `"preview"` or
/// `("thumb", index)`.
pub fn key(parts: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

struct SceneProgram {
    key: u64,
    scene: Arc<Scene>,
}

impl<M> shader::Program<M> for SceneProgram {
    type State = ();
    type Primitive = ScenePrimitive;

    fn draw(&self, _: &(), _: mouse::Cursor, _: Rectangle) -> ScenePrimitive {
        ScenePrimitive {
            key: self.key,
            scene: self.scene.clone(),
        }
    }
}

#[derive(Debug)]
struct ScenePrimitive {
    key: u64,
    scene: Arc<Scene>,
}

struct ScenePipeline {
    renderer: Renderer,
}

impl shader::Pipeline for ScenePipeline {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        ScenePipeline {
            renderer: Renderer::new(device, queue, format),
        }
    }

    fn trim(&mut self) {
        self.renderer.trim();
    }
}

impl shader::Primitive for ScenePrimitive {
    type Pipeline = ScenePipeline;

    fn prepare(
        &self,
        p: &mut ScenePipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let scale = viewport.scale_factor();
        let mut scene = (*self.scene).clone();
        scene.size = (
            (bounds.width * scale).round().max(1.0) as u32,
            (bounds.height * scale).round().max(1.0) as u32,
        );
        p.renderer.prepare(device, queue, self.key, &scene);
    }

    fn draw(&self, _: &ScenePipeline, _: &mut wgpu::RenderPass<'_>) -> bool {
        false
    }

    fn render(
        &self,
        p: &ScenePipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip: &Rectangle<u32>,
    ) {
        p.renderer.render(
            encoder,
            target,
            self.key,
            sda_render::Viewport {
                x: clip.x,
                y: clip.y,
                w: clip.width,
                h: clip.height,
            },
        );
    }
}
