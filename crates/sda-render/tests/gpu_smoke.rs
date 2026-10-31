//! GPU startup smoke test for CI and developer machines.
//!
//! Set LUMEN_REQUIRE_GPU=1 to make a missing adapter fail the test instead
//! of being treated as a local-machine skip. WGPU_BACKEND can constrain the
//! backend (for example, "dx12" on Windows CI).

#[test]
fn renderer_initializes_on_available_adapter() {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        },
    )) {
        Ok(adapter) => adapter,
        Err(error) => {
            if std::env::var_os("LUMEN_REQUIRE_GPU").is_some() {
                panic!("Lumen GPU smoke test requires an adapter, but none was found: {error}");
            }
            eprintln!("Skipping GPU smoke test: no adapter found: {error}");
            return;
        }
    };

    let info = adapter.get_info();
    println!(
        "GPU smoke test adapter: {} ({:?}), backend: {:?}, device: {:#x}, vendor: {:#x}",
        info.name, info.device_type, info.backend, info.device, info.vendor
    );

    let (device, queue) = pollster::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default()),
    )
    .expect("WGPU adapter was found but device creation failed");

    // Constructing the renderer creates every production shader module and
    // render pipeline. This catches shader validation and backend pipeline
    // creation failures without requiring an interactive desktop window.
    let _renderer = sda_render::Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );

    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("GPU device polling failed");

    println!("Lumen renderer initialized successfully.");
}
