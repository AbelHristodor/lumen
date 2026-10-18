use iced_test::{Simulator, selector};
use sda_app::msg::PresentMsg;
use sda_app::{App, Msg};
use sda_core::resolve::SlideContent;
use std::sync::Arc;

fn app() -> App {
    App::for_test()
}

fn messages(ui: Simulator<'_, Msg>) -> Vec<Msg> {
    ui.into_messages().collect()
}

fn drain(app: &mut App, msgs: Vec<Msg>) {
    for m in msgs {
        let _ = app.update(m);
    }
}

#[test]
fn typing_filters_and_selects_first() {
    let mut app = app();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(selector::id(iced::widget::Id::new(
        sda_app::state::present::SEARCH_ID,
    )))
    .unwrap();
    ui.typewrite("664");
    let msgs = messages(ui);
    drain(&mut app, msgs);
    assert_eq!(app.present.hits.len(), 1);
    assert_eq!(app.present.selected, Some(0));
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("664b").is_ok());
}

#[test]
fn enter_goes_live_and_strip_shows_title() {
    let mut app = app();
    let _ = app.update(Msg::Search("150".into()));
    let _ = app.update(Msg::GoLive);
    assert!(app.live.present.is_loaded());
    assert_eq!(app.live.present.index, 0);
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("150").is_ok());
    assert_eq!(app.core.history[0].number.as_deref(), Some("150"));
}

#[test]
fn recent_shows_on_empty_query() {
    let mut app = app();
    let _ = app.update(Msg::Search("150".into()));
    let _ = app.update(Msg::GoLive);
    let _ = app.update(Msg::Search(String::new()));
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find(&*app.core.strings.recent.to_uppercase()).is_ok());
}

#[test]
fn clicking_a_row_selects_it() {
    let mut app = app();
    let _ = app.update(Msg::Search(String::new()));
    assert_eq!(app.present.selected, Some(0));
    let mut ui = Simulator::new(app.view_operator());
    ui.click("356").unwrap();
    let msgs = messages(ui);
    drain(&mut app, msgs);
    let idx = app.present.hits[app.present.selected.unwrap()];
    let hymn = app.core.searcher.as_ref().unwrap().entry(idx).unwrap();
    assert_eq!(hymn.number.as_deref(), Some("356"));
}

#[test]
fn go_live_at_starts_on_that_slide_with_a_prefixed_title() {
    let mut app = app();
    let _ = app.update(Msg::Search("150".into()));
    let _ = app.update(Msg::PreviewSlide(1));
    let _ = app.update(Msg::GoLiveAt(app.present.preview_slide));
    assert_eq!(app.live.present.index, 1);
    assert!(app.live.present.title.starts_with("150 "));
    assert!(matches!(
        app.live.present.slides[0],
        SlideContent::Styled(_)
    ));
}

#[test]
fn as_is_toggle_saves_the_override_and_rendered_swaps_live_slides() {
    use sda_app::msg::SettingsMsg;
    use sda_core::asis::Converter;
    let mut app = app();
    let _ = app.update(Msg::Settings(SettingsMsg::Detected(Some(
        Converter::LibreOffice("/nonexistent/soffice".into()),
    ))));
    let _ = app.update(Msg::Search("150".into()));
    let path = {
        let idx = app.present.hits[0];
        app.core
            .searcher
            .as_ref()
            .unwrap()
            .entry(idx)
            .unwrap()
            .path
            .clone()
    };
    let _ = app.update(Msg::Present(PresentMsg::AsIsToggle(true)));
    assert_eq!(app.core.overrides.as_is.get(&path), Some(&true));
    assert!(app.present.as_is_busy);
    let saved: sda_core::overrides::Overrides =
        sda_core::store::json_load(&app.core.paths.overrides);
    assert_eq!(saved.as_is.get(&path), Some(&true));

    let _ = app.update(Msg::GoLive);
    let png = app.core.paths.asis_cache.join("1.png");
    let img = Arc::new(image::RgbaImage::new(4, 4));
    let _ = app.update(Msg::Present(PresentMsg::AsIsRendered(Ok((
        path.clone(),
        vec![(png.clone(), img)],
    )))));
    assert!(!app.present.as_is_busy);
    assert_eq!(
        app.live.present.slides,
        vec![SlideContent::Bitmap(png.clone())]
    );
    assert!(app.live.bitmaps.contains_key(&png));

    let _ = app.update(Msg::Present(PresentMsg::AsIsToggle(false)));
    assert_eq!(app.core.overrides.as_is.get(&path), None);
}

#[test]
fn second_toggle_while_busy_does_not_restart() {
    let mut app = app_with_converter();
    let _ = app.update(Msg::Search("150".into()));
    let _ = app.update(Msg::Present(PresentMsg::AsIsToggle(true)));
    assert!(app.present.as_is_busy);
    let _ = app.update(Msg::Present(PresentMsg::AsIsToggle(true)));
    assert!(app.present.as_is_busy);
}

#[test]
fn rendered_after_toggle_off_leaves_live_styled() {
    let mut app = app_with_converter();
    let _ = app.update(Msg::Search("150".into()));
    let path = selected_path(&app);
    let _ = app.update(Msg::GoLive);
    let _ = app.update(Msg::Present(PresentMsg::AsIsToggle(true)));
    let _ = app.update(Msg::Present(PresentMsg::AsIsToggle(false)));
    let png = app.core.paths.asis_cache.join("1.png");
    let img = Arc::new(image::RgbaImage::new(4, 4));
    let _ = app.update(Msg::Present(PresentMsg::AsIsRendered(Ok((
        path,
        vec![(png.clone(), img)],
    )))));
    assert!(app.live.bitmaps.contains_key(&png));
    assert!(matches!(
        app.live.present.slides[0],
        SlideContent::Styled(_)
    ));
}

#[test]
fn rendered_error_clears_busy_and_leaves_live_alone() {
    let mut app = app();
    let _ = app.update(Msg::Search("150".into()));
    let _ = app.update(Msg::GoLive);
    app.present.as_is_busy = true;
    let _ = app.update(Msg::Present(PresentMsg::AsIsRendered(Err("boom".into()))));
    assert!(!app.present.as_is_busy);
    assert!(matches!(
        app.live.present.slides[0],
        SlideContent::Styled(_)
    ));
}

#[test]
fn rendered_error_shows_under_the_toggle_until_the_next_toggle() {
    let mut app = app_with_converter();
    let _ = app.update(Msg::Search("150".into()));
    app.present.as_is_busy = true;
    let _ = app.update(Msg::Present(PresentMsg::AsIsRendered(Err("boom".into()))));
    assert_eq!(app.present.as_is_error.as_deref(), Some("boom"));
    {
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find("boom").is_ok());
    }

    let _ = app.update(Msg::Present(PresentMsg::AsIsToggle(false)));
    assert_eq!(app.present.as_is_error, None);
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("boom").is_err());
}

#[test]
fn go_live_with_a_cold_cache_goes_styled_and_queues_a_load() {
    use sda_app::state::present::ASIS_SIZE;
    use sda_core::asis;
    let mut app = app_with_converter();
    let _ = app.update(Msg::Search("150".into()));
    let (path, mtime) = {
        let idx = app.present.hits[0];
        let h = app.core.searcher.as_ref().unwrap().entry(idx).unwrap();
        (h.path.clone(), h.mtime)
    };
    let dir = asis::cache_dir_for(&app.core.paths.asis_cache, &path, mtime, ASIS_SIZE);
    std::fs::create_dir_all(&dir).unwrap();
    image::RgbaImage::new(4, 4).save(dir.join("1.png")).unwrap();
    std::fs::write(dir.join(".done"), "").unwrap();
    app.core.overrides.as_is.insert(path.clone(), true);

    let _ = app.update(Msg::GoLive);
    assert!(app.present.as_is_busy);
    assert!(
        !app.present.has_pending_load(),
        "update starts the queued load right away"
    );
    assert!(matches!(
        app.live.present.slides[0],
        SlideContent::Styled(_)
    ));
}

fn app_with_converter() -> App {
    use sda_app::msg::SettingsMsg;
    use sda_core::asis::Converter;
    let mut app = app();
    let _ = app.update(Msg::Settings(SettingsMsg::Detected(Some(
        Converter::LibreOffice("/nonexistent/soffice".into()),
    ))));
    app
}

fn selected_path(app: &App) -> std::path::PathBuf {
    let idx = app.present.hits[app.present.selected.unwrap()];
    app.core
        .searcher
        .as_ref()
        .unwrap()
        .entry(idx)
        .unwrap()
        .path
        .clone()
}
