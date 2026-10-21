use iced_test::Simulator;
use sda_app::msg::VideosMsg;
use sda_app::{App, Msg, Tab};
use std::path::PathBuf;

/// Guards the env vars the fake ffmpeg reads. Every test that sets
/// `LUMEN_NO_AUDIO` or `FAKE_FRAMES` holds it for its whole body.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV.lock().unwrap_or_else(|e| e.into_inner())
}

fn app() -> App {
    let mut app = App::for_test();
    let _ = app.update(Msg::Tab(Tab::Videos));
    app
}

#[test]
fn empty_tab_offers_open_file_and_no_transport() {
    let app = app();
    let mut ui = Simulator::new(app.view_operator());
    let s = &app.core.strings;
    assert!(ui.find(&*s.videos_open_file).is_ok());
    assert!(ui.find(&*s.videos_pick_hint).is_ok());
    assert!(ui.find(&*s.play).is_err());
    assert!(ui.find(&*s.videos_download.to_uppercase()).is_ok());
}

#[test]
fn choosing_a_file_marks_it_opening_and_a_failure_shows_the_reason() {
    let mut app = app();
    let path = PathBuf::from("/clips/intro.mp4");
    let _ = app.update(Msg::Videos(VideosMsg::FileChosen(Some(path.clone()))));
    assert_eq!(app.videos.picked.as_deref(), Some(path.as_path()));
    assert!(app.videos.opening);
    let _ = app.update(Msg::Videos(VideosMsg::Opened(Err("Invalid data".into()))));
    assert!(!app.videos.opening);
    assert_eq!(app.videos.error.as_deref(), Some("Invalid data"));
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("Invalid data").is_ok());
    assert!(ui.find("intro.mp4").is_ok());
}

#[test]
fn volume_is_saved_to_config() {
    let mut app = app();
    let _ = app.update(Msg::Videos(VideosMsg::Volume(0.3)));
    assert_eq!(app.videos.volume, 0.3);
    assert_eq!(app.core.cfg.video_volume, 0.3);
}

#[cfg(unix)]
mod with_fake_ffmpeg {
    use super::*;
    use sda_media::{Player, State};
    use std::sync::Arc;
    use std::time::Duration;

    fn fake() -> PathBuf {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sda-media/tests/fixtures/fake_ffmpeg.sh"
        ))
    }

    /// The caller must hold `env_lock` for its whole body.
    fn opened_app() -> (App, Arc<Player>) {
        unsafe { std::env::set_var("LUMEN_NO_AUDIO", "1") };
        unsafe { std::env::set_var("FAKE_FRAMES", "60") };
        let player =
            Arc::new(Player::open(&fake(), std::path::Path::new("clip.mp4"), (320, 180)).unwrap());
        unsafe { std::env::remove_var("FAKE_FRAMES") };
        let mut app = app();
        let _ = app.update(Msg::Videos(VideosMsg::FileChosen(Some(PathBuf::from(
            "clip.mp4",
        )))));
        let _ = app.update(Msg::Videos(VideosMsg::Opened(Ok(player.clone()))));
        (app, player)
    }

    #[test]
    fn transport_drives_the_preview_player() {
        let _env = env_lock();
        let (mut app, player) = opened_app();
        assert!(!app.videos.opening);
        assert_eq!(player.state(), State::Paused);
        let _ = app.update(Msg::Videos(VideosMsg::Toggle));
        assert_eq!(player.state(), State::Playing);
        assert!(sda_app::state::videos::preview_playing(&app));
        let _ = app.update(Msg::Videos(VideosMsg::Toggle));
        assert_eq!(player.state(), State::Paused);
        let _ = app.update(Msg::Videos(VideosMsg::Seek(1.5)));
        assert_eq!(player.position(), Duration::from_millis(1500));
        let _ = app.update(Msg::Videos(VideosMsg::Step(-10)));
        assert_eq!(player.position(), Duration::ZERO);
        let _ = app.update(Msg::Videos(VideosMsg::Volume(0.5)));
        assert_eq!(player.volume(), 0.5);
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find("0:00 / 0:02").is_ok());
    }

    #[test]
    fn present_puts_the_same_player_live_and_enter_does_too() {
        let _env = env_lock();
        let (mut app, player) = opened_app();
        let _ = app.update(Msg::Videos(VideosMsg::Present));
        assert!(Arc::ptr_eq(app.live.player.as_ref().unwrap(), &player));
        assert_eq!(player.state(), State::Playing);
        assert_eq!(app.live.present.title, "clip.mp4");
        assert!(matches!(
            app.live.present.source,
            sda_core::present::Source::Video(ref p) if p.ends_with("clip.mp4")
        ));
        // Enter on the Videos tab is the same action.
        use iced::keyboard::{self, Key, key::Named};
        let enter = iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(Named::Enter),
            modified_key: Key::Named(Named::Enter),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });
        assert!(matches!(
            sda_app::shortcuts::filter(
                enter,
                iced::event::Status::Ignored,
                iced::window::Id::unique(),
                Tab::Videos
            ),
            Some(Msg::Videos(VideosMsg::Present))
        ));
        // Space pauses the live video, a second Space resumes it.
        let _ = app.update(Msg::LiveSpace);
        assert_eq!(player.state(), State::Paused);
        let _ = app.update(Msg::LiveSpace);
        assert_eq!(player.state(), State::Playing);
    }
}
