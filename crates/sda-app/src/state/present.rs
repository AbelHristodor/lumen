//! Present tab state: search, selection, preview. The Present tab task
//! replaces the views. These functions keep the names `app.rs` calls.

use crate::app::App;
use crate::msg::{Msg, PresentMsg};
use iced::Task;
use iced::widget;
use sda_core::history::{self, HistoryEntry};
use sda_core::present::{Presentation, Source};
use sda_core::resolve::SlideContent;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PresentState {
    pub query: String,
    /// Searcher indices of the current hits, best first.
    pub hits: Vec<usize>,
    /// Position inside `hits`.
    pub selected: Option<usize>,
    pub preview_slide: usize,
    pub scroll: widget::Id,
    pub as_is_busy: bool,
}

impl Default for PresentState {
    fn default() -> Self {
        PresentState {
            query: String::new(),
            hits: Vec::new(),
            selected: None,
            preview_slide: 0,
            scroll: widget::Id::new("results"),
            as_is_busy: false,
        }
    }
}

pub fn update(_app: &mut App, msg: PresentMsg) -> Task<Msg> {
    match msg {
        PresentMsg::Noop => Task::none(),
    }
}

/// Rank hymns against `app.present.query` and select the first hit.
pub fn run_search(app: &mut App) {
    let Some(searcher) = app.core.searcher.as_mut() else {
        app.present.hits.clear();
        app.present.selected = None;
        return;
    };
    app.present.hits = searcher
        .search(&app.present.query)
        .into_iter()
        .map(|h| h.index)
        .collect();
    app.present.selected = (!app.present.hits.is_empty()).then_some(0);
    app.present.preview_slide = 0;
}

pub fn select(app: &mut App, i: usize) {
    if i < app.present.hits.len() {
        app.present.selected = Some(i);
        app.present.preview_slide = 0;
    }
}

/// Move the selection by `delta` rows, clamped to the hit list.
pub fn move_selection(app: &mut App, delta: i32) {
    let len = app.present.hits.len();
    if len == 0 {
        return;
    }
    let cur = app.present.selected.unwrap_or(0) as i64;
    let next = (cur + delta as i64).clamp(0, len as i64 - 1) as usize;
    select(app, next);
}

/// Put the selected hymn live starting at `slide`.
pub fn go_live(app: &mut App, slide: usize) {
    let Some(hymn) = app
        .present
        .selected
        .and_then(|i| app.present.hits.get(i))
        .and_then(|&idx| app.core.searcher.as_ref()?.entry(idx))
        .cloned()
    else {
        return;
    };
    let title = match &hymn.number {
        Some(n) => format!("{n} {}", hymn.title),
        None => hymn.title.clone(),
    };
    let slides: Vec<SlideContent> = hymn
        .slides
        .iter()
        .cloned()
        .map(SlideContent::Styled)
        .collect();
    let mut p = Presentation::default();
    p.load(
        title,
        Some(hymn.library.clone()),
        slides,
        Source::Hymn(hymn.path.clone()),
    );
    p.jump(slide + 1);
    app.live.go(p);
    let opened_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    history::record(
        &mut app.core.history,
        HistoryEntry {
            path: hymn.path,
            number: hymn.number,
            title: hymn.title,
            opened_at,
        },
    );
    app.core.save_history();
}
