use iced_test::Simulator;
use sda_app::msg::VideosMsg;
use sda_app::state::videos::Phase;
use sda_app::{App, Msg, Tab};
use sda_core::downloader::{DownloadEvent, DownloadProgress, parse_probe};
use std::path::PathBuf;

fn videos_app() -> App {
    let mut app = App::for_test();
    let _ = app.update(Msg::Tab(Tab::Videos));
    app
}

fn probed_app() -> App {
    let mut app = videos_app();
    let probe = parse_probe(include_str!(
        "../../sda-core/tests/fixtures/ytdlp_probe.json"
    ))
    .unwrap();
    let _ = app.update(Msg::Videos(VideosMsg::Url("https://youtu.be/x".into())));
    let _ = app.update(Msg::Videos(VideosMsg::Probed(Ok(probe))));
    app
}

#[test]
fn quality_radio_sets_format_id() {
    let mut app = probed_app();
    assert_eq!(app.videos.phase, Phase::Ready);
    let mut ui = Simulator::new(app.view_operator());
    ui.click("720p").unwrap();
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.videos.format_id.as_deref(), Some("136"));
}

#[test]
fn download_button_disabled_until_probed() {
    let app = videos_app();
    let label = app.core.strings.videos_download.clone();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(&*label).unwrap();
    assert!(ui.into_messages().next().is_none());
    assert_eq!(app.videos.phase, Phase::Idle);

    let app = probed_app();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(&*label).unwrap();
    assert!(
        ui.into_messages()
            .any(|m| matches!(m, Msg::Videos(VideosMsg::Download)))
    );
}

#[test]
fn download_events_drive_phase_and_history() {
    let mut app = probed_app();
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Resolving)));
    assert_eq!(app.videos.phase, Phase::Downloading);
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Progress(
        DownloadProgress {
            percent: 42.0,
            speed: "1MiB/s".into(),
            eta: "00:10".into(),
            title: "Sample".into(),
        },
    ))));
    assert_eq!(app.videos.percent, 42.0);
    assert_eq!(app.videos.eta, "00:10");
    let dir = PathBuf::from("/tmp/vid");
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Done {
        path: dir.clone(),
    })));
    assert_eq!(app.videos.phase, Phase::Done(dir.clone()));
    assert_eq!(app.core.downloads.len(), 1);
    assert_eq!(app.core.downloads[0].title, "Sample");
    assert_eq!(app.core.downloads[0].path, dir);
    assert!(app.core.paths.downloads.exists());
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find("Sample").is_ok());
    }
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Done {
        path: dir.clone(),
    })));
    assert_eq!(
        app.core.downloads.len(),
        1,
        "the same file and title is one history row"
    );

    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Failed {
        message: "boom".into(),
    })));
    assert_eq!(app.videos.phase, Phase::Failed);
    assert!(app.videos.message.contains("boom"));
}

#[test]
fn fetch_is_ignored_while_downloading() {
    let mut app = probed_app();
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Resolving)));
    assert_eq!(app.videos.phase, Phase::Downloading);
    let _ = app.update(Msg::Videos(VideosMsg::Url("https://youtu.be/other".into())));
    let _ = app.update(Msg::Videos(VideosMsg::Fetch));
    assert_eq!(app.videos.phase, Phase::Downloading);
    assert!(app.videos.probe.is_some());
    assert_eq!(app.videos.format_id.as_deref(), Some("299"));

    let label = app.core.strings.videos_fetch.clone();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(&*label).unwrap();
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn stream_end_without_done_fails_the_download() {
    let mut app = probed_app();
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Resolving)));
    let _ = app.update(Msg::Videos(VideosMsg::Ended));
    assert_eq!(app.videos.phase, Phase::Failed);
    assert!(
        app.videos
            .message
            .starts_with(&app.core.strings.download_failed_prefix)
    );

    let dir = PathBuf::from("/tmp/vid");
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Resolving)));
    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Done {
        path: dir.clone(),
    })));
    let _ = app.update(Msg::Videos(VideosMsg::Ended));
    assert_eq!(app.videos.phase, Phase::Done(dir));
}

#[test]
fn thumbnail_bytes_are_shown_after_probe() {
    let mut app = probed_app();
    assert!(app.videos.thumbnail.is_none());
    let png = {
        let mut buf = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]))
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        buf.into_inner()
    };
    let _ = app.update(Msg::Videos(VideosMsg::Thumbnail(None)));
    assert!(app.videos.thumbnail.is_none());
    let _ = app.update(Msg::Videos(VideosMsg::Thumbnail(Some(png))));
    assert!(app.videos.thumbnail.is_some());
    let _ = Simulator::new(app.view_operator());
    let _ = app.update(Msg::Videos(VideosMsg::Fetch));
    assert!(app.videos.thumbnail.is_none());
}

/// Guards the env vars the fake ffmpeg reads. Every test that sets
/// `LUMEN_NO_AUDIO` or `FAKE_FRAMES` holds it for its whole body.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV.lock().unwrap_or_else(|e| e.into_inner())
}

fn app() -> App {
    videos_app()
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
    fn scrub_only_stores_and_release_seeks() {
        let _env = env_lock();
        let (mut app, player) = opened_app();
        let _ = app.update(Msg::Videos(VideosMsg::Scrub(1.0)));
        assert_eq!(app.videos.scrub, Some(1.0));
        assert_eq!(player.position(), Duration::ZERO);
        let _ = app.update(Msg::Videos(VideosMsg::Seek(1.0)));
        assert_eq!(player.position(), Duration::from_secs(1));
        assert_eq!(app.videos.scrub, None);
    }

    #[test]
    fn seek_uses_the_last_scrub_over_a_stale_release_value() {
        let _env = env_lock();
        let (mut app, player) = opened_app();
        let _ = app.update(Msg::Videos(VideosMsg::Scrub(1.0)));
        let _ = app.update(Msg::Videos(VideosMsg::Seek(0.2)));
        assert_eq!(player.position(), Duration::from_secs(1));
        assert_eq!(app.videos.scrub, None);
    }

    #[test]
    fn a_late_open_for_an_older_pick_is_dropped() {
        let _env = env_lock();
        unsafe { std::env::set_var("LUMEN_NO_AUDIO", "1") };
        unsafe { std::env::set_var("FAKE_FRAMES", "60") };
        let player =
            Arc::new(Player::open(&fake(), std::path::Path::new("clip.mp4"), (320, 180)).unwrap());
        unsafe { std::env::remove_var("FAKE_FRAMES") };
        let mut app = app();
        let _ = app.update(Msg::Videos(VideosMsg::FileChosen(Some(PathBuf::from(
            "clip.mp4",
        )))));
        let _ = app.update(Msg::Videos(VideosMsg::FileChosen(Some(PathBuf::from(
            "other.mp4",
        )))));
        let _ = app.update(Msg::Videos(VideosMsg::Opened(Ok(player))));
        assert!(app.videos.player.is_none());
        assert!(app.videos.opening);
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
