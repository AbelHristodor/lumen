//! Offscreen rendering for tests. Returns `None` when the machine has no GPU
//! adapter so tests can skip instead of fail.

use crate::{Renderer, Scene, Viewport};
use image::RgbaImage;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Request any hardware adapter and a default device. A software adapter
/// such as WARP or llvmpipe is skipped, its output does not match the
/// goldens drawn on a real GPU.
pub fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .ok()?;
    let info = adapter.get_info();
    if info.device_type == wgpu::DeviceType::Cpu {
        eprintln!("software adapter {}, skipping", info.name);
        return None;
    }
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

/// Render `scene` into an RGBA image of `scene.size`.
pub fn render_to_image(scene: &Scene) -> Option<RgbaImage> {
    let (w, h) = scene.size;
    let full = Viewport { x: 0, y: 0, w, h };
    render_many((w, h), wgpu::Color::BLACK, &[(scene.clone(), full)])
}

/// Clear a `size` texture to `clear`, then prepare every scene under its
/// own key and render each into its viewport, in the order Iced uses. The
/// scissor equals the viewport.
pub fn render_many(
    size: (u32, u32),
    clear: wgpu::Color,
    scenes: &[(Scene, Viewport)],
) -> Option<RgbaImage> {
    let both: Vec<(Scene, Viewport, Viewport)> =
        scenes.iter().map(|(s, v)| (s.clone(), *v, *v)).collect();
    render_scissored(size, clear, &both)
}

/// One scene mapped onto `viewport` with only `scissor` written.
pub fn render_clipped(
    size: (u32, u32),
    clear: wgpu::Color,
    scene: &Scene,
    viewport: Viewport,
    scissor: Viewport,
) -> Option<RgbaImage> {
    render_scissored(size, clear, &[(scene.clone(), viewport, scissor)])
}

fn render_scissored(
    size: (u32, u32),
    clear: wgpu::Color,
    scenes: &[(Scene, Viewport, Viewport)],
) -> Option<RgbaImage> {
    let (device, queue) = device()?;
    let (w, h) = size;
    let mut renderer = Renderer::new(&device, &queue, FORMAT);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("headless target"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Rows in the readback buffer are padded to the copy alignment.
    let row = w * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = row.div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("headless readback"),
        size: (padded * h) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    for (i, (scene, _, _)) in scenes.iter().enumerate() {
        renderer.prepare(&device, &queue, i as u64, scene);
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    // An empty pass clears the whole target to the given color.
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("headless clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    for (i, (_, vp, sc)) in scenes.iter().enumerate() {
        renderer.render(&mut encoder, &view, i as u64, *vp, *sc);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
    rx.recv().ok()?.ok()?;

    let data = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((row * h) as usize);
    for y in 0..h as usize {
        let start = y * padded as usize;
        pixels.extend_from_slice(&data[start..start + row as usize]);
    }
    drop(data);
    buffer.unmap();
    RgbaImage::from_raw(w, h, pixels)
}
