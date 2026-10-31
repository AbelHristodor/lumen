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

/// Return the platform's persistent log directory.
fn log_directory() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(std::path::PathBuf::from))
            .unwrap_or_else(std::env::temp_dir);
        return base.join("Lumen").join("logs");
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        return home.join("Library").join("Logs").join("Lumen");
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let state = std::env::var_os("XDG_STATE_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(std::path::PathBuf::from)
                    .map(|home| home.join(".local").join("state"))
            })
            .unwrap_or_else(std::env::temp_dir);
        state.join("lumen").join("logs")
    }
}
/// Start persistent rotating logging, register fonts, and run the daemon.
pub fn run() -> iced::Result {
    let log_dir = log_directory();
    let log_filter = "lumen=info,sda_app=info,sda_core=info,sda_render=info";

    let logger = flexi_logger::Logger::try_with_env_or_str(log_filter)
        .expect("valid default log filter")
        .log_to_file(
            flexi_logger::FileSpec::default()
                .directory(&log_dir)
                .basename("lumen")
                .suffix("log"),
        )
        .rotate(
            flexi_logger::Criterion::Size(5 * 1024 * 1024),
            flexi_logger::Naming::Timestamps,
            flexi_logger::Cleanup::KeepLogFiles(5),
        )
        .duplicate_to_stderr(flexi_logger::Duplicate::Warn);

    // Keep the handle alive until the daemon exits; dropping it shuts down
    // flexi_logger's file writer and flushes its buffers.
    let _logger_handle = match logger.start() {
        Ok(handle) => Some(handle),
        Err(error) => {
            eprintln!(
                "Could not start persistent Lumen logging in {}: {error}",
                log_dir.display()
            );
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or(log_filter),
            )
            .try_init()
            .ok();
            None
        }
    };

    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("Application panic: {info}\nBacktrace:\n{backtrace}");
    }));
    log::info!("Starting Lumen {}", env!("CARGO_PKG_VERSION"));
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
