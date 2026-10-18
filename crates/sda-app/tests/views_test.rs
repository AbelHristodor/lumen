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
            window::Id::unique()
        ),
        Some(Msg::LiveNext)
    ));
    assert!(
        sda_app::shortcuts::filter(
            plain(Key::Named(Named::ArrowRight)),
            event::Status::Captured,
            window::Id::unique()
        )
        .is_none()
    );
    assert!(matches!(
        sda_app::shortcuts::filter(
            plain(Key::Character("3".into())),
            event::Status::Ignored,
            window::Id::unique()
        ),
        Some(Msg::LiveJump(3))
    ));
    let cmd = keyboard::Modifiers::COMMAND;
    assert!(matches!(
        sda_app::shortcuts::filter(
            press(Key::Character("3".into()), cmd),
            event::Status::Ignored,
            window::Id::unique()
        ),
        Some(Msg::Tab(Tab::Videos))
    ));
    assert!(matches!(
        sda_app::shortcuts::filter(
            press(Key::Character("k".into()), cmd),
            event::Status::Ignored,
            window::Id::unique()
        ),
        Some(Msg::FocusSearch)
    ));
    assert!(
        sda_app::shortcuts::filter(
            plain(Key::Character("x".into())),
            event::Status::Ignored,
            window::Id::unique()
        )
        .is_none()
    );
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
