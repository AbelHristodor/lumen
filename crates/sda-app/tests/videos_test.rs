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

    let _ = app.update(Msg::Videos(VideosMsg::Event(DownloadEvent::Failed {
        message: "boom".into(),
    })));
    assert_eq!(app.videos.phase, Phase::Failed);
    assert!(app.videos.message.contains("boom"));
}
