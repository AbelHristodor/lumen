use iced::{event, keyboard, window};
use sda_app::displays::Display;
use sda_app::{App, Msg};
use std::time::{Duration, Instant};

fn two_displays() -> Vec<Display> {
    vec![
        Display {
            index: 0,
            label: "Display 1 · 1440x900".into(),
            rect: (0, 0, 1440, 900),
            scale: 2.0,
            is_primary: true,
        },
        Display {
            index: 1,
            label: "Display 2 1920x1080".into(),
            rect: (1440, 0, 1920, 1080),
            scale: 1.0,
            is_primary: false,
        },
    ]
}

fn app() -> App {
    let mut app = App::for_test();
    let _ = app.update(Msg::DisplaysListed(two_displays()));
    app
}

#[test]
fn displays_listed_picks_the_second_screen() {
    let app = app();
    assert_eq!(app.displays.len(), 2);
    assert_eq!(app.projector_display, Some(1));
}

#[test]
fn toggle_opens_the_projector_and_the_close_event_clears_it() {
    let mut app = app();
    let _ = app.update(Msg::ProjectorToggle);
    let id = app.projector.expect("projector opens on display 2");
    assert_ne!(id, app.operator);
    assert_eq!(app.core.cfg.output_display, Some(1));
    assert_eq!(app.title(id), "SDA Manager Projector");

    let _ = app.update(Msg::ProjectorClosed(id));
    assert!(app.projector.is_none());
}

#[test]
fn toggle_closes_an_open_projector() {
    let mut app = app();
    let _ = app.update(Msg::ProjectorToggle);
    assert!(app.projector.is_some());
    let _ = app.update(Msg::ProjectorToggle);
    assert!(app.projector.is_none());
}

#[test]
fn toggle_without_a_display_does_nothing() {
    let mut app = App::for_test();
    app.displays.clear();
    app.projector_display = None;
    let _ = app.update(Msg::ProjectorToggle);
    assert!(app.projector.is_none());
}

#[test]
fn stop_closes_the_projector() {
    let mut app = app();
    let _ = app.update(Msg::ProjectorToggle);
    let _ = app.update(Msg::LiveStop);
    assert!(app.projector.is_none());
}

#[test]
fn escape_maps_to_stop() {
    let esc = iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(keyboard::key::Named::Escape),
        modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    });
    assert!(matches!(
        sda_app::shortcuts::filter(esc, event::Status::Ignored, window::Id::unique()),
        Some(Msg::LiveStop)
    ));
}

#[test]
fn identify_opens_one_window_per_display_then_closes_them() {
    let mut app = app();
    let _ = app.update(Msg::Identify);
    assert_eq!(app.identify.len(), 2);
    assert_ne!(app.identify[0], app.identify[1]);
    assert!(!app.identify.contains(&app.operator));

    let first = app.identify[0];
    let _ = app.update(Msg::ProjectorClosed(first));
    assert_eq!(app.identify.len(), 1, "a closed identify window drops out");

    let _ = app.update(Msg::IdentifyDone);
    assert!(app.identify.is_empty());
}

#[test]
fn identify_twice_does_not_stack_windows() {
    let mut app = app();
    let _ = app.update(Msg::Identify);
    let _ = app.update(Msg::Identify);
    assert_eq!(app.identify.len(), 2);
}

#[test]
fn tick_stores_the_clock_and_settles_the_transition() {
    let mut app = app();
    let _ = app.update(Msg::GoLive);
    assert!(app.live.present.is_loaded());
    let _ = app.update(Msg::LiveNext);
    assert!(app.live.started.is_some());

    let now = Instant::now();
    let _ = app.update(Msg::Tick(now));
    assert_eq!(app.now, now);
    assert!(app.live.started.is_some(), "mid transition stays running");

    let later = now + Duration::from_millis(app.live.theme.transition.duration_ms as u64 + 50);
    let _ = app.update(Msg::Tick(later));
    assert_eq!(app.now, later);
    assert!(app.live.started.is_none(), "a finished transition settles");
    assert!(app.live.outgoing.is_none());
}

#[test]
fn blank_and_stop_do_not_start_a_transition_when_nothing_is_loaded() {
    let mut app = App::for_test();
    let _ = app.update(Msg::LiveBlank);
    assert!(app.live.started.is_none());
    let _ = app.update(Msg::LiveStop);
    assert!(app.live.started.is_none());
}
