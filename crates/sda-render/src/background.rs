//! Background pipelines: solid color, drifting gradient, and floating
//! particles. All three are vertex buffer free passes that share one uniform
//! block layout, so `UniformPass` holds the wgpu plumbing and each pipeline
//! only fills its uniform. The uniform buffer itself lives in a `UniformSlot`
//! owned by the caller, so many scenes can be prepared before any draws.

use crate::{UniformSlot, srgb_constant};
use sda_core::theme::Rgba;

/// Most stops a gradient can carry. Matches the array in `gradient.wgsl`.
pub const MAX_STOPS: usize = 8;

/// One pipeline with a uniform block layout at group 0.
struct UniformPass {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform_size: u64,
    label: &'static str,
}

impl UniformPass {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        label: &'static str,
        source: &str,
        uniform_size: u64,
        blend: Option<wgpu::BlendState>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let constants = srgb_constant(format);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        UniformPass {
            pipeline,
            layout,
            uniform_size,
            label,
        }
    }

    fn slot(&self, device: &wgpu::Device) -> UniformSlot {
        UniformSlot::new(device, &self.layout, self.uniform_size, self.label)
    }

    fn write<T: bytemuck::Pod>(&self, queue: &wgpu::Queue, slot: &UniformSlot, value: &T) {
        queue.write_buffer(&slot.buffer, 0, bytemuck::bytes_of(value));
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, slot: &UniformSlot, vertices: u32) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &slot.group, &[]);
        pass.draw(0..vertices, 0..1);
    }
}

/// Encoded sRGB channels scaled to 0..1 with `a` in the last slot.
fn encoded(c: Rgba, a: f32) -> [f32; 4] {
    [
        c.r as f32 / 255.0,
        c.g as f32 / 255.0,
        c.b as f32 / 255.0,
        a,
    ]
}

/// Uniform block for `solid.wgsl`. Color is encoded sRGB in 0..1 and the
/// alpha channel carries the opacity.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SolidUniform {
    color: [f32; 4],
}

pub struct SolidPipeline(UniformPass);

impl SolidPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        SolidPipeline(UniformPass::new(
            device,
            format,
            "solid",
            include_str!("../shaders/solid.wgsl"),
            size_of::<SolidUniform>() as u64,
            None,
        ))
    }

    pub fn slot(&self, device: &wgpu::Device) -> UniformSlot {
        self.0.slot(device)
    }

    pub fn prepare(&self, queue: &wgpu::Queue, slot: &UniformSlot, color: Rgba, opacity: f32) {
        self.0.write(
            queue,
            slot,
            &SolidUniform {
                color: encoded(color, opacity.clamp(0.0, 1.0)),
            },
        );
    }

    /// Fill the whole viewport with the color prepared in `slot`.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, slot: &UniformSlot) {
        self.0.draw(pass, slot, 3);
    }
}

/// Uniform block for `gradient.wgsl`. Each stop is rgb plus position in a.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GradientUniform {
    stops: [[f32; 4]; MAX_STOPS],
    count: u32,
    angle: f32,
    phase: f32,
    opacity: f32,
}

pub struct GradientPipeline(UniformPass);

impl GradientPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        GradientPipeline(UniformPass::new(
            device,
            format,
            "gradient",
            include_str!("../shaders/gradient.wgsl"),
            size_of::<GradientUniform>() as u64,
            None,
        ))
    }

    pub fn slot(&self, device: &wgpu::Device) -> UniformSlot {
        self.0.slot(device)
    }

    /// `angle` is in degrees. `phase` is drift times seconds.
    pub fn prepare(
        &self,
        queue: &wgpu::Queue,
        slot: &UniformSlot,
        stops: &[(f32, Rgba)],
        angle: f32,
        phase: f32,
        opacity: f32,
    ) {
        let mut sorted: Vec<(f32, Rgba)> = stops.iter().copied().take(MAX_STOPS).collect();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut u = GradientUniform {
            stops: [[0.0; 4]; MAX_STOPS],
            count: sorted.len() as u32,
            angle: angle.to_radians(),
            phase,
            opacity: opacity.clamp(0.0, 1.0),
        };
        for (slot, (pos, c)) in u.stops.iter_mut().zip(sorted) {
            *slot = encoded(c, pos);
        }
        self.0.write(queue, slot, &u);
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, slot: &UniformSlot) {
        self.0.draw(pass, slot, 3);
    }
}

/// Uniform block for `particles.wgsl`. Color is rgb plus opacity in a.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ParticlesUniform {
    color: [f32; 4],
    viewport: [f32; 2],
    speed: f32,
    time: f32,
}

/// Points of light drawn over the solid base color. Call
/// `SolidPipeline::draw` with the base first.
pub struct ParticlesPipeline(UniformPass);

impl ParticlesPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        ParticlesPipeline(UniformPass::new(
            device,
            format,
            "particles",
            include_str!("../shaders/particles.wgsl"),
            size_of::<ParticlesUniform>() as u64,
            Some(additive),
        ))
    }

    pub fn slot(&self, device: &wgpu::Device) -> UniformSlot {
        self.0.slot(device)
    }

    /// The particle count is not a uniform. The caller keeps it and passes
    /// it to `draw`.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &self,
        queue: &wgpu::Queue,
        slot: &UniformSlot,
        color: Rgba,
        speed: f32,
        time: f32,
        size: (u32, u32),
        opacity: f32,
    ) {
        self.0.write(
            queue,
            slot,
            &ParticlesUniform {
                color: encoded(color, opacity.clamp(0.0, 1.0)),
                viewport: [size.0 as f32, size.1 as f32],
                speed,
                time,
            },
        );
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, slot: &UniformSlot, count: u32) {
        if count > 0 {
            self.0.draw(pass, slot, count * 6);
        }
    }
}
