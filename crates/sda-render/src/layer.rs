//! Textured quad pipeline that draws one slide layer or an image
//! background. Text layers are rasterized on the CPU into an RGBA image and
//! uploaded here. Each drawn quad owns one uniform slot so the background,
//! the outgoing layer, and the current layer can carry different
//! `LayerParams` inside a single render pass.

use crate::srgb_constant;
use image::RgbaImage;

/// Number of uniform slots. One per quad a frame can draw.
pub const SLOTS: usize = 3;

/// Uniform block for `image.wgsl`. Padding matches the WGSL layout.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LayerParams {
    pub alpha: f32,
    /// Blur radius in texels. 0 samples once.
    pub blur: f32,
    /// Shift as a fraction of the viewport. 1 moves one full width.
    pub offset: [f32; 2],
    /// Size of the quad per axis. 1 fills the viewport.
    pub scale: [f32; 2],
    _pad: [f32; 2],
}

impl LayerParams {
    pub fn opaque() -> Self {
        LayerParams {
            alpha: 1.0,
            blur: 0.0,
            offset: [0.0, 0.0],
            scale: [1.0, 1.0],
            _pad: [0.0, 0.0],
        }
    }
}

pub struct LayerPipeline {
    pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    params: [wgpu::Buffer; SLOTS],
    params_groups: [wgpu::BindGroup; SLOTS],
}

impl LayerPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("layer"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/image.wgsl").into()),
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("layer texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let params_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("layer params layout"),
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
        let params: [wgpu::Buffer; SLOTS] = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("layer params"),
                size: size_of::<LayerParams>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let params_groups = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("layer params group"),
                layout: &params_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params[i].as_entire_binding(),
                }],
            })
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("layer sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("layer pipeline layout"),
            bind_group_layouts: &[&texture_layout, &params_layout],
            push_constant_ranges: &[],
        });
        let constants = srgb_constant(format);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("layer pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
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
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        LayerPipeline {
            pipeline,
            texture_layout,
            sampler,
            params,
            params_groups,
        }
    }

    /// Upload `img` as a texture and return the bind group that draws it.
    /// The bind group keeps the texture alive.
    pub fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &RgbaImage,
    ) -> wgpu::BindGroup {
        let (w, h) = img.dimensions();
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("layer texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Plain Unorm so the shader reads encoded bytes as they are.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            img.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("layer texture group"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Write the uniforms for `slot`. Call before the pass is recorded.
    pub fn set_params(&self, queue: &wgpu::Queue, slot: usize, params: &LayerParams) {
        queue.write_buffer(&self.params[slot], 0, bytemuck::bytes_of(params));
    }

    /// Draw `texture` with the uniforms of `slot`.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, texture: &wgpu::BindGroup, slot: usize) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, texture, &[]);
        pass.set_bind_group(1, &self.params_groups[slot], &[]);
        pass.draw(0..4, 0..1);
    }
}
