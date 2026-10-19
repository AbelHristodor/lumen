use iced::{event, keyboard, window};
use iced_test::Simulator;
use sda_app::{App, Msg, Tab};
use sda_core::present::{Presentation, Source};
use sda_core::resolve::SlideContent;
use std::time::{Duration, Instant};

fn app() -> App {
    App::for_test()
}

fn press(k: keyboard::Key, modifiers: keyboard::Modifiers) -> iced::Event {
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: k.clone(),
        modified_key: k,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

fn plain(k: keyboard::Key) -> iced::Event {
    press(k, keyboard::Modifiers::default())
}

fn three_slides(app: &App) -> Presentation {
    let hymn = app
        .core
        .searcher
        .as_ref()
        .unwrap()
        .entry(0)
        .unwrap()
        .clone();
    let slides: Vec<SlideContent> = hymn
        .slides
        .iter()
        .take(3)
        .cloned()
        .map(SlideContent::Styled)
        .collect();
    assert_eq!(slides.len(), 3, "fixture hymn needs three slides");
    let mut p = Presentation::default();
    p.load(
        hymn.title.clone(),
        None,
        slides,
        Source::Hymn(hymn.path.clone()),
    );
    p
}

#[test]
fn for_test_indexes_the_four_fixture_hymns() {
    let app = app();
    assert_eq!(app.core.searcher.as_ref().unwrap().len(), 4);
    assert!(!app.gpu_ok);
    assert_eq!(app.displays.len(), 1);
    assert_eq!(app.tab, Tab::Present);
}

#[test]
fn tab_buttons_switch_and_strip_stays() {
    let mut app = app();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(&*app.core.strings.tab_videos).unwrap();
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.tab, Tab::Videos);
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find(&*app.core.strings.nothing_live).is_ok());
}

#[test]
fn shortcut_filter_ignores_captured_and_maps_keys() {
    use keyboard::{Key, key::Named};
    assert!(matches!(
        sda_app::shortcuts::filter(
            plain(Key::Named(Named::ArrowRight)),
            event::Status::Ignored,
            window::Id::unique(),
            Tab::Present
        ),
        Some(Msg::LiveNext)
    ));
    assert!(
        sda_app::shortcuts::filter(
            plain(Key::Named(Named::ArrowRight)),
            event::Status::Captured,
            window::Id::unique(),
            Tab::Present
        )
        .is_none()
    );
    assert!(matches!(
        sda_app::shortcuts::filter(
            plain(Key::Character("3".into())),
            event::Status::Ignored,
            window::Id::unique(),
            Tab::Present
        ),
        Some(Msg::LiveJump(3))
    ));
    let cmd = keyboard::Modifiers::COMMAND;
    assert!(matches!(
        sda_app::shortcuts::filter(
            press(Key::Character("3".into()), cmd),
            event::Status::Ignored,
            window::Id::unique(),
            Tab::Present
        ),
        Some(Msg::Tab(Tab::Videos))
    ));
    assert!(matches!(
        sda_app::shortcuts::filter(
            press(Key::Character("k".into()), cmd),
            event::Status::Ignored,
            window::Id::unique(),
            Tab::Present
        ),
        Some(Msg::FocusSearch)
    ));
    assert!(
        sda_app::shortcuts::filter(
            plain(Key::Character("x".into())),
            event::Status::Ignored,
            window::Id::unique(),
            Tab::Present
        )
        .is_none()
    );
}

#[test]
fn enter_goes_live_only_on_the_present_tab() {
    use keyboard::{Key, key::Named};
    let enter = || plain(Key::Named(Named::Enter));
    assert!(matches!(
        sda_app::shortcuts::filter(
            enter(),
            event::Status::Ignored,
            window::Id::unique(),
            Tab::Present
        ),
        Some(Msg::GoLive)
    ));
    for tab in [Tab::Bible, Tab::Videos, Tab::Themes, Tab::Settings] {
        assert!(
            sda_app::shortcuts::filter(enter(), event::Status::Ignored, window::Id::unique(), tab)
                .is_none(),
            "{tab:?} ignores Enter"
        );
    }
    assert!(matches!(
        sda_app::shortcuts::filter(
            plain(Key::Named(Named::ArrowRight)),
            event::Status::Ignored,
            window::Id::unique(),
            Tab::Bible
        ),
        Some(Msg::LiveNext)
    ));
}

#[test]
fn live_messages_drive_the_presentation_and_strip() {
    let mut app = app();
    let p = three_slides(&app);
    let title = p.title.clone();
    app.live.go(p);
    let _ = app.update(Msg::LiveNext);
    assert_eq!(app.live.present.index, 1);
    let _ = app.update(Msg::LiveJump(3));
    assert_eq!(app.live.present.index, 2);
    let _ = app.update(Msg::LivePrev);
    assert_eq!(app.live.present.index, 1);
    let _ = app.update(Msg::LiveBlank);
    assert!(app.live.present.blank);
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find(&*title).is_ok());
        assert!(ui.find("2 / 3").is_ok());
    }
    let _ = app.update(Msg::LiveStop);
    assert!(!app.live.present.is_loaded());
}

#[test]
fn live_scene_runs_a_transition_then_settles() {
    let mut app = app();
    app.live.go(three_slides(&app));
    app.live.next();
    let now = Instant::now();
    let scene = app.live.scene((160, 90), now);
    assert_eq!(scene.size, (160, 90));
    assert!(scene.current.is_some());
    assert!(
        scene.outgoing.is_some(),
        "a fresh step has an outgoing slide"
    );
    assert!(app.live.animating());
    let later = now + Duration::from_millis(app.live.theme.transition.duration_ms as u64 + 50);
    let scene = app.live.scene((160, 90), later);
    assert!(
        scene.outgoing.is_none(),
        "outgoing drops after the duration"
    );
    let later_again = Instant::now() + Duration::from_secs(5);
    let scene = app.live.scene((160, 90), later_again);
    assert!(scene.outgoing.is_none());
}

#[test]
fn move_selection_only_acts_on_present_tab() {
    let mut app = app();
    let _ = app.update(Msg::Search(String::new()));
    let before = app.present.selected;
    let _ = app.update(Msg::Tab(Tab::Themes));
    let _ = app.update(Msg::MoveSelection(1));
    assert_eq!(app.present.selected, before);
    let _ = app.update(Msg::Tab(Tab::Present));
    let _ = app.update(Msg::MoveSelection(1));
    assert_ne!(app.present.selected, before);
}

#[test]
fn default_display_prefers_a_non_primary_screen() {
    use sda_app::displays::{Display, default_display};
    let d = |i: usize, primary: bool| Display {
        index: i,
        label: format!("Display {}", i + 1),
        rect: (0, 0, 100, 100),
        scale: 1.0,
        is_primary: primary,
    };
    assert_eq!(default_display(&[d(0, true), d(1, false)]), Some(1));
    assert_eq!(default_display(&[d(0, true)]), None);
    assert_eq!(default_display(&[]), None);
}

#[test]
fn settings_detected_and_themes_set_active_update_state() {
    use sda_app::msg::{SettingsMsg, ThemesMsg};
    use sda_core::asis::Converter;
    let mut app = app();
    let conv = Converter::LibreOffice("/usr/bin/soffice".into());
    let _ = app.update(Msg::Settings(SettingsMsg::Detected(Some(conv.clone()))));
    assert_eq!(app.core.converter, Some(conv));
    let _ = app.update(Msg::Themes(ThemesMsg::SetActive("Default".into())));
    assert_eq!(app.core.active_theme.name, "Default");
    assert_eq!(app.live.theme.name, "Default");
    assert_eq!(app.core.cfg.active_theme.as_deref(), Some("Default"));
}

#[test]
fn gpu_banner_shows_only_when_the_probe_failed() {
    let mut app = app();
    let _ = app.update(Msg::GpuProbed(false));
    assert!(!app.gpu_ok);
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find(&*app.core.strings.gpu_unavailable).is_ok());
    }
    let _ = app.update(Msg::GpuProbed(true));
    assert!(app.gpu_ok);
}

#[test]
fn live_scene_resolves_a_preloaded_bitmap_slide() {
    use sda_render::Layer;
    use std::path::PathBuf;
    use std::sync::Arc;
    let mut app = app();
    let png = PathBuf::from("/tmp/asis/1.png");
    let img = Arc::new(image::RgbaImage::new(4, 4));
    app.live.bitmaps.insert(png.clone(), img.clone());
    let mut p = Presentation::default();
    p.load(
        "150 Test".into(),
        None,
        vec![SlideContent::Bitmap(png)],
        Source::Hymn(PathBuf::from("/tmp/150.pptx")),
    );
    app.live.go(p);
    let scene = app.live.scene((160, 90), Instant::now());
    match &scene.current {
        Some(Layer::Bitmap(b)) => assert!(Arc::ptr_eq(b, &img)),
        other => panic!("expected a bitmap layer, got {other:?}"),
    }
}

#[test]
fn video_background_installs_for_the_matching_theme_and_drops_on_change() {
    use sda_app::msg::{BackgroundAsset, BgTarget};
    use sda_core::theme::{Background, Theme};
    use sda_render::{BackgroundState, FrameSource};
    use std::path::PathBuf;
    let mut app = app();
    let theme = Theme {
        background: Background::Video {
            path: PathBuf::from("/tmp/bg.mp4"),
        },
        ..Theme::default()
    };
    app.live.set_theme(theme.clone());
    assert!(app.live.background_pending());

    let stale = Background::Video {
        path: PathBuf::from("/tmp/other.mp4"),
    };
    let _ = app.update(Msg::BackgroundReady(
        BgTarget::Live,
        stale,
        Ok(BackgroundAsset::Video(FrameSource::new((16, 9)))),
    ));
    assert!(
        app.live.video.is_none(),
        "a late result for another file is dropped"
    );

    let src = FrameSource::new((16, 9));
    let _ = app.update(Msg::BackgroundReady(
        BgTarget::Live,
        theme.background.clone(),
        Ok(BackgroundAsset::Video(src.clone())),
    ));
    assert!(!app.live.background_pending());
    let shows_video = matches!(
        app.live.scene((160, 90), Instant::now()).background,
        BackgroundState::Video(_)
    );
    assert!(shows_video);

    app.live.set_theme(Theme::default());
    assert!(
        app.live.video.is_none(),
        "a new background drops the decoder"
    );

    let library = app.status.library.clone();
    app.live.set_theme(theme.clone());
    let _ = app.update(Msg::BackgroundReady(
        BgTarget::Live,
        theme.background.clone(),
        Err("ffmpeg not found".into()),
    ));
    assert!(
        !app.status.library_error,
        "a background error is not a library error"
    );
    assert_eq!(
        app.status.library, library,
        "the hymn count stays in the chip"
    );
    let notice = app.status.notice.clone().expect("notice set");
    assert!(notice.contains("ffmpeg not found"));
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find(&*notice).is_ok());
    }

    let _ = app.update(Msg::BackgroundReady(
        BgTarget::Live,
        theme.background.clone(),
        Ok(BackgroundAsset::Video(FrameSource::new((16, 9)))),
    ));
    assert!(
        app.status.notice.is_none(),
        "a later success clears the notice"
    );
}

#[test]
fn preview_and_live_share_one_decoder_for_the_same_video() {
    use sda_app::msg::{BackgroundAsset, BgTarget, ThemesMsg};
    use sda_core::theme::{Background, Theme, store};
    use sda_render::FrameSource;
    use std::path::PathBuf;
    use std::sync::Arc;
    let mut app = app();
    let theme = Theme {
        name: "Clip".into(),
        background: Background::Video {
            path: PathBuf::from("/tmp/bg.mp4"),
        },
        ..Theme::default()
    };
    store::save_theme(&app.core.paths.themes_dir, &theme).unwrap();
    app.core.reload_themes();
    let _ = app.update(Msg::Themes(ThemesMsg::SetActive("Clip".into())));
    let src = FrameSource::new((16, 9));
    let _ = app.update(Msg::BackgroundReady(
        BgTarget::Live,
        theme.background.clone(),
        Ok(BackgroundAsset::Video(src.clone())),
    ));
    let i = sda_app::state::themes::sorted_themes(&app.core)
        .iter()
        .position(|t| t.name == "Clip")
        .unwrap();
    let _ = app.update(Msg::Themes(ThemesMsg::Select(i)));
    let shared = app
        .themes
        .preview
        .video
        .as_ref()
        .expect("preview adopted the live decoder");
    assert!(Arc::ptr_eq(shared, &src));
    assert!(!app.themes.preview.background_pending());

    let other = FrameSource::new((16, 9));
    let _ = app.update(Msg::BackgroundReady(
        BgTarget::ThemePreview,
        theme.background.clone(),
        Ok(BackgroundAsset::Video(other)),
    ));
    assert!(
        Arc::ptr_eq(app.themes.preview.video.as_ref().unwrap(), &src),
        "a second decoder for the same file is dropped"
    );
}

#[test]
fn one_video_worker_in_flight_and_no_retry_after_a_failure() {
    use sda_app::msg::themes::DraftEdit;
    use sda_app::msg::{BgTarget, ThemesMsg};
    use sda_core::theme::{Background, Theme};
    use std::path::PathBuf;
    let mut app = app();
    let bg = Background::Video {
        path: PathBuf::from("/tmp/bg.mp4"),
    };
    app.themes.draft = Theme {
        background: bg.clone(),
        ..Theme::default()
    };
    // Off the Themes tab the preview clock stays idle, so every task unit
    // the update returns is one background worker.
    let first = app.update(Msg::Themes(ThemesMsg::Draft(DraftEdit::BgOpacity(0.9))));
    assert_eq!(first.units(), 1, "the first message spawns one worker");
    assert!(!app.themes.preview.background_pending());
    let second = app.update(Msg::Themes(ThemesMsg::Draft(DraftEdit::BgOpacity(0.8))));
    assert_eq!(second.units(), 0, "a worker in flight blocks a second one");

    let _ = app.update(Msg::BackgroundReady(
        BgTarget::ThemePreview,
        bg.clone(),
        Err("ffmpeg not found".into()),
    ));
    assert!(
        app.status.notice.is_none(),
        "a preview error is not a notice"
    );
    let third = app.update(Msg::Themes(ThemesMsg::Draft(DraftEdit::BgOpacity(0.7))));
    assert_eq!(third.units(), 0, "a failed file is not retried");

    app.themes.draft.background = Background::Video {
        path: PathBuf::from("/tmp/other.mp4"),
    };
    let fourth = app.update(Msg::Themes(ThemesMsg::Draft(DraftEdit::BgOpacity(0.6))));
    assert_eq!(fourth.units(), 1, "a new file gets its own worker");
}

#[test]
fn window_geometry_saves_only_the_latest_change_and_loads_back() {
    use iced::{Point, Size};
    use sda_app::app::WindowGeom;
    use sda_core::store::json_load;
    let mut app = app();
    let path = app.core.paths.window.clone();
    let other = iced::window::Id::unique();
    let _ = app.update(Msg::WindowMoved(other, Point::new(5.0, 5.0)));
    assert!(app.window_geom.is_none(), "other windows are ignored");
    assert_eq!(app.window_geom_seq, 0);

    let _ = app.update(Msg::WindowMoved(app.operator, Point::new(40.0, 60.0)));
    let first = app.window_geom_seq;
    let _ = app.update(Msg::WindowResized(app.operator, Size::new(1000.0, 700.0)));
    let second = app.window_geom_seq;
    assert!(second > first);

    let _ = app.update(Msg::SaveWindow(first));
    assert!(!path.exists(), "a stale debounce timer writes nothing");
    let _ = app.update(Msg::SaveWindow(second));
    let saved: Option<WindowGeom> = json_load(&path);
    assert_eq!(
        saved,
        Some(WindowGeom {
            x: 40.0,
            y: 60.0,
            w: 1000.0,
            h: 700.0,
        })
    );
}

#[test]
fn status_chip_colors_follow_the_library_state_and_cuts_long_text() {
    use sda_app::view::shell::{CHIP_CHARS, shorten};
    let mut app = app();
    let long = "x".repeat(CHIP_CHARS + 20);
    let _ = app.update(Msg::Synced(0, Err(long.clone())));
    let short = shorten(&app.status.library, CHIP_CHARS);
    assert_eq!(short.chars().count(), CHIP_CHARS);
    assert!(short.ends_with('…'));
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find(&*short).is_ok(), "the chip shows the cut text");
    }
    let _ = app.update(Msg::UpdateChecked("3.1.0".into(), true));
    assert!(app.status.library_error);
    let _ = app.update(Msg::ForceSync);
    assert!(app.status.sync_busy);
    assert!(!app.status.library_error, "a new sync clears the error");
    let mut ui = Simulator::new(app.view_operator());
    assert!(
        ui.find(&*app.core.strings.update_available).is_ok(),
        "update button shows once the library is fine"
    );
}

#[test]
fn sync_error_chip_shows_a_short_reason_and_keeps_the_full_error() {
    let mut app = app();
    let chain =
        "HEAD https://github.com/x/hymns.zip: https://github.com/x/hymns.zip: status code 404";
    let _ = app.update(Msg::Synced(0, Err(chain.into())));
    assert_eq!(
        app.status.library,
        app.core
            .strings
            .status_sync_failed_fmt
            .replace("{}", "HTTP 404")
    );
    assert!(!app.status.library.contains("https"));
    assert_eq!(app.status.library_detail.as_deref(), Some(chain));
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find(&*app.status.library).is_ok());
}

#[test]
fn sync_error_marks_the_chip_and_hides_the_update_chip() {
    let mut app = app();
    let up_to_date = app.core.strings.update_up_to_date.clone();
    let _ = app.update(Msg::UpdateChecked(up_to_date.clone(), false));
    let _ = app.update(Msg::Synced(0, Err("HEAD failed".into())));
    assert!(app.status.library_error);
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find(&*app.status.library).is_ok());
        assert!(ui.find(&*up_to_date).is_err());
    }
    let _ = app.update(Msg::Synced(0, Ok(None)));
    assert!(
        app.status.library_error,
        "unchanged sync keeps the last state"
    );
}
