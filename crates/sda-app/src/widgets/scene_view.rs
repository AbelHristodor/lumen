//! Iced shader widget that draws a `Scene` through `sda_render`.

use iced::advanced::graphics::Viewport;
use iced::widget::shader;
use iced::{Element, Fill, Rectangle, mouse};
use sda_render::{Renderer, Scene};
use std::sync::Arc;

pub fn scene_view<'a, M: 'a>(scene: Arc<Scene>) -> Element<'a, M> {
    shader(SceneProgram { scene })
        .width(Fill)
        .height(Fill)
        .into()
}

struct SceneProgram {
    scene: Arc<Scene>,
}

impl<M> shader::Program<M> for SceneProgram {
    type State = ();
    type Primitive = ScenePrimitive;

    fn draw(&self, _: &(), _: mouse::Cursor, _: Rectangle) -> ScenePrimitive {
        ScenePrimitive {
            scene: self.scene.clone(),
        }
    }
}

#[derive(Debug)]
struct ScenePrimitive {
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

    fn trim(&mut self) {}
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
        p.renderer.prepare(device, queue, &scene);
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
            sda_render::Viewport {
                x: clip.x,
                y: clip.y,
                w: clip.width,
                h: clip.height,
            },
        );
    }
}
