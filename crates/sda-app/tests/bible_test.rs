use iced::keyboard::Modifiers;
use iced::{Settings, Size};
use iced_test::{Simulator, selector};
use sda_app::msg::BibleMsg;
use sda_app::{App, Msg, Tab};
use sda_core::bible::{self, Bible};
use sda_core::present::Source;
use sda_core::theme::Prop;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn mini() -> Arc<Bible> {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sda-core/tests/fixtures/bible_mini.json"
    ));
    Arc::new(bible::load(path).expect("fixture bible"))
}

fn app() -> App {
    let mut app = App::for_test();
    app.bible.bible = Some(mini());
    app.bible.version = Some("bible_mini".into());
    app.bible.installed = vec!["bible_mini".into()];
    let _ = app.update(Msg::Tab(Tab::Bible));
    app
}

#[test]
fn input_jumps_on_short_reference() {
    let mut app = app();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(selector::id(sda_app::view::bible::INPUT_ID))
        .expect("input");
    ui.typewrite("joh 3 16-18");
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.bible.book, 42);
    assert_eq!(app.bible.chapter, 3);
    assert_eq!(app.bible.selected, BTreeSet::from([16, 17, 18]));
    assert_eq!(app.bible.preview_slide, 0);
}

#[test]
fn present_builds_verse_slides() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    app.bible.selected = BTreeSet::from([16, 17, 18]);
    let _ = app.update(Msg::Bible(BibleMsg::Present));
    let n = app.live.present.slides.len();
    assert!((1..=3).contains(&n), "packed into {n} slides");
    assert_eq!(app.live.present.title, "John 3:16-18");
    assert!(matches!(
        &app.live.present.source,
        Source::Bible { version, reference }
            if version == "bible_mini" && reference == "John 3:16-18"
    ));
    app.live.theme.verses.min_size = 60.0;
    app.live.theme.text.margin = Prop::Set(0.45);
    let _ = app.update(Msg::Bible(BibleMsg::Present));
    assert_eq!(
        app.live.present.slides.len(),
        3,
        "a tiny box gives one verse per slide"
    );
}

#[test]
fn version_pick_list_lists_installed() {
    let app = app();
    let mut ui = Simulator::new(app.view_operator());
    assert!(
        ui.find("MINI").is_ok(),
        "installed version abbreviation shows"
    );
}

fn press(app: &mut App, n: u32) {
    let _ = app.update(Msg::Bible(BibleMsg::Press(n)));
    let _ = app.update(Msg::Bible(BibleMsg::Release));
}

#[test]
fn click_toggles_shift_click_ranges() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    press(&mut app, 2);
    assert_eq!(app.bible.selected, BTreeSet::from([2]));
    press(&mut app, 4);
    assert_eq!(app.bible.selected, BTreeSet::from([2, 4]));
    press(&mut app, 2);
    assert_eq!(
        app.bible.selected,
        BTreeSet::from([4]),
        "second click deselects"
    );
    assert_eq!(app.bible.anchor, Some(2));

    let _ = app.update(Msg::Modifiers(Modifiers::SHIFT));
    press(&mut app, 6);
    assert_eq!(app.bible.selected, BTreeSet::from([2, 3, 4, 5, 6]));
    assert_eq!(app.bible.anchor, Some(2), "shift keeps the anchor");
    let _ = app.update(Msg::Modifiers(Modifiers::empty()));

    let _ = app.update(Msg::Bible(BibleMsg::ClearSelection));
    assert!(app.bible.selected.is_empty());
    let _ = app.update(Msg::Bible(BibleMsg::Book(0)));
    assert_eq!(app.bible.chapter, 1);
    assert_eq!(app.bible.anchor, None);
}

#[test]
fn drag_selects_a_range() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    let _ = app.update(Msg::Bible(BibleMsg::Press(3)));
    assert!(app.bible.dragging);
    let _ = app.update(Msg::Bible(BibleMsg::Enter(5)));
    assert_eq!(app.bible.selected, BTreeSet::from([3, 4, 5]));
    let _ = app.update(Msg::Bible(BibleMsg::Enter(1)));
    assert_eq!(
        app.bible.selected,
        BTreeSet::from([1, 2, 3]),
        "drag upward replaces the drag range"
    );
    let _ = app.update(Msg::Bible(BibleMsg::Release));
    assert!(!app.bible.dragging);
    let _ = app.update(Msg::Bible(BibleMsg::Enter(9)));
    assert_eq!(
        app.bible.selected,
        BTreeSet::from([1, 2, 3]),
        "hover after release does nothing"
    );
}

#[test]
fn verse_row_click_selects_without_a_checkbox() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    // Tall enough that verse 16 is on screen without scrolling.
    let mut ui = Simulator::with_size(
        Settings::default(),
        Size::new(1400.0, 3000.0),
        app.view_operator(),
    );
    assert!(ui.find("Select to here").is_err());
    ui.click("For God so loved the world").expect("verse row");
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    // The simulator click emits only the press, so end the drag here.
    let _ = app.update(Msg::Bible(BibleMsg::Release));
    assert!(!app.bible.dragging);
    assert_eq!(app.bible.selected, BTreeSet::from([16]));
}

#[test]
fn text_mode_searches_and_a_result_jumps() {
    let mut app = app();
    let _ = app.update(Msg::Bible(BibleMsg::TextMode(true)));
    let _ = app.update(Msg::Bible(BibleMsg::Input("so loved".into())));
    assert_eq!(app.bible.results.len(), 1);
    let mut ui = Simulator::new(app.view_operator());
    ui.click("John 3:16").expect("search result button");
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.bible.book, 42);
    assert_eq!(app.bible.chapter, 3);
    assert_eq!(app.bible.selected, BTreeSet::from([16]));
    let _ = app.update(Msg::Bible(BibleMsg::TextMode(false)));
    assert!(
        app.bible.results.is_empty(),
        "leaving text mode clears results"
    );
}

#[test]
fn submit_in_text_mode_jumps_to_the_first_result() {
    let mut app = app();
    let _ = app.update(Msg::Bible(BibleMsg::TextMode(true)));
    let _ = app.update(Msg::Bible(BibleMsg::Input("so loved".into())));
    let _ = app.update(Msg::Bible(BibleMsg::Submit));
    assert_eq!(app.bible.selected, BTreeSet::from([16]));
}

#[test]
fn preview_slides_follow_the_selection() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    assert!(
        !sda_app::state::bible::preview_slides(&app).is_empty(),
        "whole chapter when nothing is selected"
    );
    press(&mut app, 16);
    let slides = sda_app::state::bible::preview_slides(&app);
    assert_eq!(slides.len(), 1);
    match &slides[0] {
        sda_core::resolve::SlideContent::Verses { reference, verses } => {
            assert_eq!(reference, "John 3:16");
            assert_eq!(verses.len(), 1);
        }
        _ => panic!("verses slide"),
    }
    let _ = app.update(Msg::Bible(BibleMsg::PreviewSlide(5)));
    assert_eq!(app.bible.preview_slide, 0, "clamped to the slide count");
}

#[test]
fn preview_footer_matches_the_projector() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    app.bible.selected = BTreeSet::from([16, 17, 18]);
    app.live.theme.footer = true;
    let slides = sda_app::state::bible::preview_slides(&app);
    let scene =
        sda_app::state::bible::preview_scene(&app, &slides[0], (320, 180)).expect("preview scene");
    match &scene.current {
        Some(sda_render::Layer::Text(slide)) => {
            assert_eq!(slide.footer, Some("John 3:16-18".into()));
        }
        _ => panic!("text layer"),
    }
}

#[test]
fn stale_version_load_is_dropped() {
    let mut app = App::for_test();
    app.bible.installed = vec!["bible_mini".into(), "other".into()];
    let _ = app.update(Msg::Bible(BibleMsg::Version("other".into())));
    assert_eq!(app.bible.version.as_deref(), Some("other"));
    assert_eq!(app.bible.busy.as_deref(), Some("other"));

    let _ = app.update(Msg::Bible(BibleMsg::Loaded(
        "bible_mini".into(),
        Ok(mini()),
    )));
    assert!(
        app.bible.bible.is_none(),
        "a load for another code is dropped"
    );
    assert_eq!(app.bible.busy.as_deref(), Some("other"), "still loading");

    let _ = app.update(Msg::Bible(BibleMsg::Version("bible_mini".into())));
    assert_eq!(
        app.bible.version.as_deref(),
        Some("other"),
        "the picker is a no op while a load runs"
    );

    let _ = app.update(Msg::Bible(BibleMsg::Loaded("other".into(), Ok(mini()))));
    assert!(app.bible.bible.is_some());
    assert!(app.bible.busy.is_none());
}

#[test]
fn drag_keeps_verses_selected_before_it() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    press(&mut app, 5);
    let _ = app.update(Msg::Bible(BibleMsg::Press(3)));
    let _ = app.update(Msg::Bible(BibleMsg::Enter(6)));
    assert_eq!(app.bible.selected, BTreeSet::from([3, 4, 5, 6]));
    let _ = app.update(Msg::Bible(BibleMsg::Enter(4)));
    assert_eq!(
        app.bible.selected,
        BTreeSet::from([3, 4, 5]),
        "verse 5 stays selected"
    );
    let _ = app.update(Msg::Bible(BibleMsg::Release));
}

#[test]
fn left_release_anywhere_ends_the_drag() {
    use iced::{event, mouse, window};
    let release = sda_app::shortcuts::filter(
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        event::Status::Captured,
        window::Id::unique(),
        Tab::Present,
    );
    assert!(matches!(release, Some(Msg::Bible(BibleMsg::Release))));
    let mut app = app();
    app.bible.dragging = true;
    if let Some(m) = release {
        let _ = app.update(m);
    }
    assert!(!app.bible.dragging);
}

fn text_layer(scene: &sda_render::Scene) -> Arc<sda_core::resolve::ResolvedSlide> {
    match &scene.current {
        Some(sda_render::Layer::Text(slide)) => slide.clone(),
        _ => panic!("text layer"),
    }
}

#[test]
fn preview_scene_reuses_the_resolved_slide() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    app.bible.selected = BTreeSet::from([16, 17, 18]);
    let slides = sda_app::state::bible::preview_slides(&app);
    let scene = |app: &App, slides: &[sda_core::resolve::SlideContent]| {
        sda_app::state::bible::preview_scene(app, &slides[0], (320, 180)).expect("preview scene")
    };
    let a = text_layer(&scene(&app, &slides));
    let b = text_layer(&scene(&app, &slides));
    assert!(Arc::ptr_eq(&a, &b), "same slide keeps its Arc");

    press(&mut app, 19);
    let slides = sda_app::state::bible::preview_slides(&app);
    let c = text_layer(&scene(&app, &slides));
    assert!(!Arc::ptr_eq(&a, &c), "a new selection resolves anew");
}

#[test]
fn preview_slide_resets_from_slide_two() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    app.live.theme.verses.min_size = 60.0;
    app.live.theme.text.margin = Prop::Set(0.4);
    let all: BTreeSet<u32> = bible::verses(
        app.bible.bible.as_ref().expect("bible"),
        &app.bible.reference(),
    )
    .into_iter()
    .map(|(n, _)| n)
    .collect();
    app.bible.selected = all;
    let n = sda_app::state::bible::preview_slides(&app).len();
    assert!(n >= 2, "packed into {n} slides");
    let _ = app.update(Msg::Bible(BibleMsg::PreviewSlide(1)));
    assert_eq!(app.bible.preview_slide, 1);
    press(&mut app, 16);
    assert_eq!(app.bible.preview_slide, 0);
}
