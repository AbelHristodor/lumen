use iced::widget::Id;
use iced::{Settings, Size};
use iced_test::Simulator;
use sda_app::msg::themes::{BgKind, ColorField, DraftEdit, ThemesMsg};
use sda_app::state::themes::{COLOR_INHERIT_ID, sorted_themes};
use sda_app::{App, Msg, Tab};
use sda_core::theme::{Background, Prop, Rgba, Theme, TransitionKind};
use std::time::{Duration, Instant};

fn themes_app() -> App {
    let mut app = App::for_test();
    let _ = app.update(Msg::Tab(Tab::Themes));
    app
}

fn send(app: &mut App, m: ThemesMsg) {
    let _ = app.update(Msg::Themes(m));
}

/// A tall simulator so the whole form is visible without scrolling.
fn tall(app: &App) -> Simulator<'_, Msg> {
    Simulator::with_size(
        Settings::default(),
        Size::new(1400.0, 3000.0),
        app.view_operator(),
    )
}

#[test]
fn inherit_checkbox_flips_prop() {
    let mut app = themes_app();
    assert_eq!(app.themes.draft.text.color, Prop::Inherit);
    {
        let mut ui = tall(&app);
        ui.click(Id::new(COLOR_INHERIT_ID)).unwrap();
        for m in ui.into_messages() {
            let _ = app.update(m);
        }
    }
    assert!(matches!(app.themes.draft.text.color, Prop::Set(_)));
    {
        let mut ui = tall(&app);
        ui.click(Id::new(COLOR_INHERIT_ID)).unwrap();
        for m in ui.into_messages() {
            let _ = app.update(m);
        }
    }
    assert_eq!(app.themes.draft.text.color, Prop::Inherit);
}

#[test]
fn save_persists_theme_file() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    let mut app = App::for_test_with_dirs(themes_dir.clone());
    let _ = app.update(Msg::Tab(Tab::Themes));
    send(&mut app, ThemesMsg::New);
    send(&mut app, ThemesMsg::Draft(DraftEdit::Name("Blue".into())));
    send(
        &mut app,
        ThemesMsg::Draft(DraftEdit::BgKind(BgKind::Gradient)),
    );
    send(
        &mut app,
        ThemesMsg::Draft(DraftEdit::TransitionKind(TransitionKind::Slide(
            sda_core::theme::Direction::Left,
        ))),
    );
    send(&mut app, ThemesMsg::Save);
    let path = themes_dir.join("Blue.json");
    assert!(path.exists(), "theme file written");
    let saved = Theme::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(matches!(saved.background, Background::Gradient { .. }));
    assert_eq!(
        saved.transition.kind,
        TransitionKind::Slide(sda_core::theme::Direction::Left)
    );
    assert!(app.core.themes.iter().any(|t| t.name == "Blue"));
    assert_eq!(app.themes.draft.name, "Blue");
    // Renaming on save removes the old file.
    send(&mut app, ThemesMsg::Draft(DraftEdit::Name("Navy".into())));
    send(&mut app, ThemesMsg::Save);
    assert!(themes_dir.join("Navy.json").exists());
    assert!(!themes_dir.join("Blue.json").exists());
}

#[test]
fn new_and_duplicate_create_saved_themes() {
    let mut app = themes_app();
    let before = app.core.themes.len();
    send(&mut app, ThemesMsg::New);
    assert_eq!(app.core.themes.len(), before + 1);
    assert_ne!(app.themes.draft.name, "Default");
    let first = app.themes.draft.name.clone();
    send(&mut app, ThemesMsg::Duplicate);
    assert_eq!(app.core.themes.len(), before + 2);
    assert_ne!(app.themes.draft.name, first);
    assert!(
        app.core
            .paths
            .themes_dir
            .join(format!("{first}.json"))
            .exists(),
        "new theme is on disk"
    );
}

#[test]
fn delete_refuses_default_and_removes_others() {
    let mut app = themes_app();
    send(&mut app, ThemesMsg::Select(0));
    assert!(app.themes.draft.is_builtin_default());
    send(&mut app, ThemesMsg::Delete);
    assert!(app.core.themes.iter().any(|t| t.is_builtin_default()));
    send(&mut app, ThemesMsg::New);
    let name = app.themes.draft.name.clone();
    send(&mut app, ThemesMsg::SetActive(name.clone()));
    assert_eq!(app.core.active_theme.name, name);
    send(&mut app, ThemesMsg::Delete);
    assert!(!app.core.themes.iter().any(|t| t.name == name));
    assert!(app.themes.draft.is_builtin_default());
    assert_eq!(app.core.active_theme.name, "Default", "active falls back");
    assert_eq!(app.live.theme.name, "Default");
}

#[test]
fn favorites_sort_first() {
    let mut app = themes_app();
    send(&mut app, ThemesMsg::New);
    let name = app.themes.draft.name.clone();
    assert_eq!(sorted_themes(&app.core)[0].name, "Default");
    send(&mut app, ThemesMsg::Favorite(true));
    let sorted = sorted_themes(&app.core);
    assert_eq!(sorted[0].name, name);
    assert!(sorted[0].favorite);
    assert_eq!(app.themes.draft_index, 0);
    send(&mut app, ThemesMsg::Favorite(false));
    assert_eq!(sorted_themes(&app.core)[0].name, "Default");
}

#[test]
fn hex_input_applies_only_valid_colors() {
    let mut app = themes_app();
    send(&mut app, ThemesMsg::Draft(DraftEdit::ColorInherit(false)));
    send(&mut app, ThemesMsg::Hex(ColorField::Text, "#FF0000".into()));
    assert_eq!(
        app.themes.draft.text.color,
        Prop::Set(Rgba::new(255, 0, 0, 255))
    );
    send(&mut app, ThemesMsg::Hex(ColorField::Text, "#FF00".into()));
    assert_eq!(
        app.themes.draft.text.color,
        Prop::Set(Rgba::new(255, 0, 0, 255))
    );
    assert_eq!(
        app.themes.hex_edit,
        Some((ColorField::Text, "#FF00".to_string()))
    );
}

#[test]
fn tick_loops_the_preview_while_on_the_tab() {
    let mut app = themes_app();
    send(&mut app, ThemesMsg::Select(0));
    assert!(app.themes.ticking, "a themes message starts the loop");
    let flip = app.themes.preview_flip;
    app.themes.preview_started = Instant::now() - Duration::from_secs(4);
    send(&mut app, ThemesMsg::Tick);
    assert_ne!(app.themes.preview_flip, flip);
    assert!(app.themes.preview.present.is_loaded());
    let _ = app.update(Msg::Tab(Tab::Present));
    send(&mut app, ThemesMsg::Tick);
    assert!(!app.themes.ticking, "the loop stops off the tab");
}

#[test]
fn draft_edits_change_the_background_and_transition() {
    let mut app = themes_app();
    send(
        &mut app,
        ThemesMsg::Draft(DraftEdit::BgKind(BgKind::Particles)),
    );
    assert!(matches!(
        app.themes.draft.background,
        Background::Particles { .. }
    ));
    send(&mut app, ThemesMsg::Draft(DraftEdit::BgParticleCount(300)));
    assert!(matches!(
        app.themes.draft.background,
        Background::Particles { count: 300, .. }
    ));
    send(
        &mut app,
        ThemesMsg::Draft(DraftEdit::BgKind(BgKind::Gradient)),
    );
    send(&mut app, ThemesMsg::Draft(DraftEdit::BgAngle(90.0)));
    send(
        &mut app,
        ThemesMsg::Draft(DraftEdit::BgStop(1, Rgba::new(1, 2, 3, 255))),
    );
    match &app.themes.draft.background {
        Background::Gradient { stops, angle, .. } => {
            assert_eq!(*angle, 90.0);
            assert_eq!(stops[1].1, Rgba::new(1, 2, 3, 255));
        }
        other => panic!("expected a gradient, got {other:?}"),
    }
    send(&mut app, ThemesMsg::Draft(DraftEdit::Duration(900)));
    assert_eq!(app.themes.draft.transition.duration_ms, 900);
    assert_eq!(app.themes.preview.theme, app.themes.draft);
}

#[test]
fn save_refuses_rename_onto_another_theme() {
    let dir = tempfile::tempdir().unwrap();
    let themes_dir = dir.path().join("themes");
    let mut app = App::for_test_with_dirs(themes_dir.clone());
    let _ = app.update(Msg::Tab(Tab::Themes));
    send(&mut app, ThemesMsg::New);
    send(&mut app, ThemesMsg::Draft(DraftEdit::Name("A".into())));
    send(
        &mut app,
        ThemesMsg::Draft(DraftEdit::BgKind(BgKind::Gradient)),
    );
    send(&mut app, ThemesMsg::Save);
    let path = themes_dir.join("A.json");
    let first = std::fs::read_to_string(&path).unwrap();

    send(&mut app, ThemesMsg::New);
    assert_ne!(app.themes.draft.name, "A");
    send(&mut app, ThemesMsg::Draft(DraftEdit::Name("A".into())));
    send(&mut app, ThemesMsg::Save);
    assert!(app.themes.error.is_some(), "rename onto A is refused");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), first);
    assert_eq!(app.core.themes.iter().filter(|t| t.name == "A").count(), 1);

    // Two names that differ only in a character the file stem replaces.
    send(&mut app, ThemesMsg::Draft(DraftEdit::Name("A/B".into())));
    send(&mut app, ThemesMsg::Save);
    let shared = themes_dir.join("A_B.json");
    let second = std::fs::read_to_string(&shared).unwrap();
    send(&mut app, ThemesMsg::New);
    send(&mut app, ThemesMsg::Draft(DraftEdit::Name("A?B".into())));
    send(&mut app, ThemesMsg::Save);
    assert!(app.themes.error.is_some(), "same file stem is refused");
    assert_eq!(std::fs::read_to_string(&shared).unwrap(), second);
    assert!(!app.core.themes.iter().any(|t| t.name == "A?B"));
}

#[test]
fn inherit_toggle_restores_last_set_value() {
    let mut app = themes_app();
    let red = Rgba::new(255, 0, 0, 255);
    send(&mut app, ThemesMsg::Draft(DraftEdit::ColorInherit(false)));
    send(&mut app, ThemesMsg::Hex(ColorField::Text, "#FF0000".into()));
    assert_eq!(app.themes.draft.text.color, Prop::Set(red));
    send(&mut app, ThemesMsg::Draft(DraftEdit::ColorInherit(true)));
    assert_eq!(app.themes.draft.text.color, Prop::Inherit);
    assert_eq!(app.themes.hex_edit, None, "inherit clears the hex text");
    send(&mut app, ThemesMsg::Draft(DraftEdit::ColorInherit(false)));
    assert_eq!(app.themes.draft.text.color, Prop::Set(red));
    send(&mut app, ThemesMsg::Hex(ColorField::BgSolid, "#12".into()));
    send(
        &mut app,
        ThemesMsg::Draft(DraftEdit::BgKind(BgKind::Gradient)),
    );
    assert_eq!(app.themes.hex_edit, None, "kind change clears the hex text");
}

#[test]
fn thumbnail_clock_does_not_reset_on_flip() {
    let mut app = themes_app();
    let before = app.themes.clock;
    app.themes.preview_started = Instant::now() - Duration::from_secs(4);
    send(&mut app, ThemesMsg::Tick);
    assert_eq!(app.themes.clock, before);
}

#[test]
fn themes_tab_renders_list_and_form() {
    let app = themes_app();
    let s = &app.core.strings;
    let mut ui = tall(&app);
    assert!(ui.find(&*s.theme_new).is_ok());
    assert!(ui.find(&*s.theme_save).is_ok());
    assert!(ui.find(&*s.theme_background.to_uppercase()).is_ok());
    assert!(ui.find(&*s.theme_transition.to_uppercase()).is_ok());
    assert!(ui.find("Default").is_ok());
}

#[test]
fn save_enables_only_when_the_draft_changed() {
    let mut app = themes_app();
    send(&mut app, ThemesMsg::Duplicate);
    assert!(!app.themes.draft.is_builtin_default());
    let save = app.core.strings.theme_save.clone();
    {
        let mut ui = tall(&app);
        ui.click(&*save).unwrap();
        assert_eq!(
            ui.into_messages().count(),
            0,
            "an unchanged draft has nothing to save"
        );
    }
    send(&mut app, ThemesMsg::Draft(DraftEdit::Footer(false)));
    let mut ui = tall(&app);
    ui.click(&*save).unwrap();
    assert!(
        ui.into_messages()
            .any(|m| matches!(m, Msg::Themes(ThemesMsg::Save))),
        "a changed draft can be saved"
    );
}

#[test]
fn inherit_rows_show_the_file_value_and_previews_follow_the_hymn() {
    let mut app = themes_app();
    let _ = app.update(Msg::Search("150".into()));
    let hymn = sda_app::state::present::selected_hymn(&app)
        .map(|(_, h)| h.clone())
        .expect("hymn 150 selected");
    let run = hymn.slides[0].boxes[0].paragraphs[0].runs[0].clone();
    let font = run.font.clone().expect("fixture carries a font");
    send(&mut app, ThemesMsg::Draft(DraftEdit::Footer(true)));
    {
        let mut ui = tall(&app);
        assert!(ui.find(&*format!("·  {font}")).is_ok());
    }
    let (content, title, _) = sda_app::state::themes::sample(&app);
    assert_eq!(title, sda_app::state::present::title_of(&hymn));
    assert!(matches!(
        content,
        sda_core::resolve::SlideContent::Styled(_)
    ));
    assert_eq!(
        app.themes.preview.present.source,
        sda_core::present::Source::Hymn(hymn.path.clone()),
        "the Themes preview plays the selected hymn"
    );

    let _ = app.update(Msg::Search("zzzz-no-such-hymn".into()));
    send(&mut app, ThemesMsg::Draft(DraftEdit::Footer(false)));
    let (_, title, key) = sda_app::state::themes::sample(&app);
    assert_eq!(title, app.core.strings.theme_preview_sample);
    assert!(key.is_none());
    assert_eq!(
        app.themes.preview.present.source,
        sda_core::present::Source::None
    );
}

#[test]
fn opening_themes_resyncs_the_preview_with_the_hymn_picked_in_present() {
    let mut app = themes_app();
    let _ = app.update(Msg::Tab(Tab::Present));
    let _ = app.update(Msg::Search("150".into()));
    let hymn = sda_app::state::present::selected_hymn(&app)
        .map(|(_, h)| h.clone())
        .expect("hymn 150 selected");
    let _ = app.update(Msg::PreviewSlide(1));
    assert_ne!(
        app.themes.preview.present.title,
        sda_app::state::present::title_of(&hymn),
        "nothing on the Themes tab ran yet"
    );

    let _ = app.update(Msg::Tab(Tab::Themes));
    assert_eq!(
        app.themes.preview.present.title,
        sda_app::state::present::title_of(&hymn)
    );
    assert_eq!(app.themes.preview.present.index, 1);
    let default_key = app.core.themes[0].cache_key();
    let thumb = app.themes.thumbs.get(&default_key).cloned().unwrap();

    let _ = app.update(Msg::Tab(Tab::Present));
    let _ = app.update(Msg::PreviewSlide(0));
    let _ = app.update(Msg::Tab(Tab::Themes));
    assert_eq!(app.themes.preview.present.index, 0);
    let again = app.themes.thumbs.get(&default_key).cloned().unwrap();
    assert!(
        !std::sync::Arc::ptr_eq(&thumb, &again),
        "a new preview slide rebuilds the thumbnails"
    );
}

#[test]
fn favorite_saves_only_the_flag_and_keeps_the_draft_edits() {
    let mut app = themes_app();
    send(&mut app, ThemesMsg::New);
    let name = app.themes.draft.name.clone();
    send(&mut app, ThemesMsg::Draft(DraftEdit::Footer(false)));
    send(&mut app, ThemesMsg::Favorite(true));
    assert!(app.themes.draft.favorite);
    assert!(
        !app.themes.draft.footer,
        "the unsaved edit stays in the form"
    );
    let saved = app.core.theme_named(&name).expect("saved theme");
    assert!(saved.favorite);
    assert!(saved.footer, "the unsaved edit was not written");
    assert_eq!(sorted_themes(&app.core)[0].name, name);
}
