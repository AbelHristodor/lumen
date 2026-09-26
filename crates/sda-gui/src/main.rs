#![windows_subsystem = "windows"]

mod app;
mod core;
mod live;
mod preview;
mod projector;
mod search_panel;
mod settings;
mod shortcuts;
mod slide;
mod style;
mod theme_editor;
mod videos;

use std::sync::Arc;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("sda_manager=info,sda_core=info"),
    )
    .init();
    log::info!("starting SDA Manager {}", env!("CARGO_PKG_VERSION"));
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon.png"))
        .expect("icon.png");
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SDA Manager")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([900.0, 600.0])
            .with_icon(Arc::new(icon)),
        centered: true,
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "SDA Manager",
        opts,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
