use iced_test::Simulator;
use sda_app::msg::settings::{Section, SettingsMsg};
use sda_app::ui_theme::{self, AppTheme};
use sda_app::{App, Msg, Tab};
use sda_core::i18n::Language;

fn settings_app(section: Section) -> App {
    let mut app = App::for_test();
    let _ = app.update(Msg::Tab(Tab::Settings));
    let _ = app.update(Msg::Settings(SettingsMsg::Section(section)));
    app
}

fn drain(app: &mut App, msgs: &[Msg]) {
    for m in msgs.iter().cloned() {
        // Clicking Add folder would open a native dialog, so only apply the rest.
        if !matches!(m, Msg::Settings(SettingsMsg::AddFolder)) {
            let _ = app.update(m);
        }
    }
}

#[test]
fn add_folder_button_emits_message() {
    let mut app = settings_app(Section::Library);
    let label = app.core.strings.add_folder_button.clone();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(&*label).expect("add folder button");
    let msgs: Vec<Msg> = ui.into_messages().collect();
    drain(&mut app, &msgs);
    assert!(
        msgs.iter()
            .any(|m| matches!(m, Msg::Settings(SettingsMsg::AddFolder))),
        "got {msgs:?}"
    );
}

#[test]
fn language_pick_updates_strings() {
    let mut app = settings_app(Section::General);
    let _ = app.update(Msg::Settings(SettingsMsg::Language(Language::It)));
    assert_eq!(app.core.strings.tab_settings, "Impostazioni");
    assert_eq!(app.core.cfg.language.as_deref(), Some("it"));
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("Impostazioni").is_ok());
}

#[test]
fn app_theme_pick_applies_and_persists() {
    let mut app = settings_app(Section::Appearance);
    let mut ui = Simulator::new(app.view_operator());
    ui.click("Ember").expect("ember theme row");
    let msgs: Vec<Msg> = ui.into_messages().collect();
    drain(&mut app, &msgs);
    assert!(
        msgs.iter()
            .any(|m| matches!(m, Msg::Settings(SettingsMsg::AppTheme(AppTheme::Ember)))),
        "got {msgs:?}"
    );
    assert_eq!(app.core.cfg.app_theme.as_deref(), Some("ember"));
    assert_eq!(ui_theme::app_theme(), AppTheme::Ember);
    assert_eq!(ui_theme::theme().to_string(), "Ember");
    let _ = app.update(Msg::Settings(SettingsMsg::AppTheme(AppTheme::Lumen)));
}

#[test]
fn app_theme_code_round_trips() {
    for t in AppTheme::ALL {
        assert_eq!(AppTheme::from_code(Some(t.code())), t);
    }
    assert_eq!(AppTheme::from_code(None), AppTheme::Lumen);
    assert_eq!(AppTheme::from_code(Some("neon")), AppTheme::Lumen);
}

#[test]
fn section_buttons_switch_sections() {
    let mut app = settings_app(Section::General);
    let label = app.core.strings.settings_about.clone();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(&*label).expect("about section button");
    let msgs: Vec<Msg> = ui.into_messages().collect();
    drain(&mut app, &msgs);
    assert_eq!(app.settings.section, Section::About);
    let version = format!(
        "{}{}",
        app.core.strings.version_prefix,
        env!("CARGO_PKG_VERSION")
    );
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find(&*version).is_ok(), "about shows the version");
}

#[test]
fn folder_add_remove_and_enable_persist_in_config() {
    let mut app = settings_app(Section::Library);
    let dir = tempfile::tempdir().unwrap();
    let _ = app.update(Msg::Settings(SettingsMsg::FolderPicked(Some(
        dir.path().to_path_buf(),
    ))));
    assert_eq!(app.core.cfg.libraries.len(), 1);
    let path = app.core.cfg.libraries[0].path.clone();
    assert!(app.core.cfg.libraries[0].enabled);
    let _ = app.update(Msg::Settings(SettingsMsg::EnableFolder(
        path.clone(),
        false,
    )));
    assert!(!app.core.cfg.libraries[0].enabled);
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(
            ui.find(&*app.core.strings.remove).is_ok(),
            "row shows Remove"
        );
    }
    let _ = app.update(Msg::Settings(SettingsMsg::RemoveFolder(path)));
    assert!(app.core.cfg.libraries.is_empty());
    let mut ui = Simulator::new(app.view_operator());
    assert!(
        ui.find(&*app.core.strings.user_libraries_description)
            .is_ok(),
        "empty state shows the description"
    );
}

#[test]
fn display_and_presenting_messages_update_config() {
    let mut app = settings_app(Section::Display);
    let _ = app.update(Msg::Settings(SettingsMsg::DefaultDisplay(0)));
    assert_eq!(app.core.cfg.output_display, Some(0));
    assert_eq!(app.projector_display, Some(0));
    let _ = app.update(Msg::Settings(SettingsMsg::Section(Section::Presenting)));
    let _ = app.update(Msg::Settings(SettingsMsg::BibleDefault("en_kjv".into())));
    assert_eq!(app.core.cfg.bible_version.as_deref(), Some("en_kjv"));
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find(&*app.core.strings.converter_missing).is_ok());
    assert!(ui.find(&*app.core.strings.detect_again).is_ok());
}

#[test]
fn installed_bibles_lists_json_codes_without_the_index() {
    use sda_app::state::settings::installed_bibles;
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "index.json",
        "ro_cornilescu.json",
        "en_kjv.json",
        "en_kjv.etag",
    ] {
        std::fs::write(dir.path().join(name), "{}").unwrap();
    }
    assert_eq!(
        installed_bibles(dir.path()),
        vec!["en_kjv", "ro_cornilescu"]
    );
    assert!(installed_bibles(&dir.path().join("missing")).is_empty());
}
