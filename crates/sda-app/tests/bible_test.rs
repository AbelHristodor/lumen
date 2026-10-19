use iced::keyboard::{Key, key::Named};
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
fn reference_field_jumps_to_chapter() {
    let mut app = app();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(selector::id(sda_app::view::bible::REFERENCE_ID))
        .expect("reference input");
    ui.typewrite("John 3:16-18");
    ui.tap_key(Key::Named(Named::Enter));
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.bible.book, 42);
    assert_eq!(app.bible.chapter, 3);
    assert_eq!(app.bible.selected, BTreeSet::from([16, 17, 18]));
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

#[test]
fn toggle_and_range_select_verses() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    let _ = app.update(Msg::Bible(BibleMsg::ToggleVerse(2)));
    let _ = app.update(Msg::Bible(BibleMsg::SelectRange(2, 5)));
    assert_eq!(app.bible.selected, BTreeSet::from([2, 3, 4, 5]));
    let _ = app.update(Msg::Bible(BibleMsg::ToggleVerse(3)));
    assert_eq!(app.bible.selected, BTreeSet::from([2, 4, 5]));
    let _ = app.update(Msg::Bible(BibleMsg::ClearSelection));
    assert!(app.bible.selected.is_empty());
    let _ = app.update(Msg::Bible(BibleMsg::Book(0)));
    assert_eq!(app.bible.book, 0);
    assert_eq!(app.bible.chapter, 1);
}

#[test]
fn search_result_jumps_and_selects() {
    let mut app = app();
    let _ = app.update(Msg::Bible(BibleMsg::Search("so loved".into())));
    assert_eq!(app.bible.results.len(), 1);
    let mut ui = Simulator::new(app.view_operator());
    ui.click("John 3:16").expect("search result button");
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.bible.book, 42);
    assert_eq!(app.bible.chapter, 3);
    assert_eq!(app.bible.selected, BTreeSet::from([16]));
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
