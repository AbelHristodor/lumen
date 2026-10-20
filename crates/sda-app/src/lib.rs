//! Lumen desktop app. One Iced daemon with an operator window and a
//! projector window. Logic lives in `sda-core`, drawing in `sda-render`.

pub mod app;
pub mod displays;
pub mod msg;
pub mod shortcuts;
pub mod state;
pub mod ui_theme;
pub mod view;
pub mod widgets;
pub mod workers;

pub use app::App;
pub use msg::{Msg, Tab};

/// Start logging, register the bundled fonts, and run the daemon.
pub fn run() -> iced::Result {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("lumen=info,sda_app=info,sda_core=info,sda_render=info"),
    )
    .init();
    let mut daemon = iced::daemon(App::boot, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .default_font(iced::Font::with_name("Inter"));
    for (_, bytes) in sda_render::fonts() {
        daemon = daemon.font(*bytes);
    }
    daemon = daemon.font(widgets::icon::BYTES);
    daemon.run()
}
