#![windows_subsystem = "windows"]

fn main() -> iced::Result {
    // GUI-subsystem builds do not have a console, so panics can otherwise be
    // invisible when the application is launched from Explorer.
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let diagnostic = format!("{info}\n\nBacktrace:\n{backtrace}\n");
        let path = std::env::temp_dir().join("lumen-panic.log");
        let _ = std::fs::write(path, diagnostic);
    }));

    sda_app::run()
}
