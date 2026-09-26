//! Library tab: search, preview, open, reveal, send to Control.

use crate::i18n::fill3;
use crate::state::App;
use crate::AppWindow;
use hymnal_core::i18n::Strings;
use hymnal_core::model::HymnEntry;
use log::{debug, info, warn};
use slint::{ComponentHandle, Model, ModelRc, SharedString, StandardListViewItem, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

/// Run `query`, fill `rows` with the hit -> entry map and return the list
/// model. Labels are precomputed by the searcher, so this is one allocation
/// per visible row and no formatting.
pub fn search_rows(app: &App, rows: &RefCell<Vec<usize>>, query: &str) -> Option<ModelRc<StandardListViewItem>> {
    let mut guard = app.searcher.borrow_mut();
    let s = guard.as_mut()?;
    let hits = s.search(query);
    debug!("query '{query}' -> {} hits", hits.len());
    let map: Vec<usize> = hits.iter().map(|h| h.index).collect();
    let items: Vec<StandardListViewItem> = map
        .iter()
        .map(|&i| StandardListViewItem::from(SharedString::from(s.label(i).unwrap_or(""))))
        .collect();
    *rows.borrow_mut() = map;
    Some(ModelRc::from(Rc::new(VecModel::from(items))))
}

/// Show slide `idx` of `entry` in the preview, clamped into range.
fn show_slide(ui: &AppWindow, strings: &Strings, entry: &HymnEntry, idx: i32) {
    let count = entry.slides.len() as i32;
    if count == 0 {
        ui.set_slide_text("".into());
        ui.set_slide_count(0);
        ui.set_slide_index(0);
        ui.set_preview_status(strings.slide_zero_fmt.replace("{}", &entry.title).into());
        return;
    }
    let idx = idx.clamp(0, count - 1);
    let titled = match &entry.number {
        Some(n) => format!("{n}. {}", entry.title),
        None => entry.title.clone(),
    };
    ui.set_slide_text(entry.slides[idx as usize].as_str().into());
    ui.set_slide_count(count);
    ui.set_slide_index(idx);
    ui.set_preview_status(
        fill3(&strings.slide_counter_fmt, &titled, &(idx + 1).to_string(), &count.to_string()).into(),
    );
}

fn open_path(path: &std::path::Path, verb: &str) {
    info!("{verb} {}", path.display());
    if let Err(e) = open::that(path) {
        warn!("failed to {verb} {}: {e}", path.display());
    }
}

pub fn wire(ui: &AppWindow, app: &App) {
    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_query_changed(move |q| {
        let Some(ui) = weak.upgrade() else { return };
        let Some(model) = search_rows(&a, &a.rows, &q) else {
            debug!("query '{q}' ignored: searcher not ready yet");
            return;
        };
        let sel = if model.row_count() > 0 { 0 } else { -1 };
        ui.set_results(model);
        ui.set_current_index(sel);
        ui.invoke_current_changed(sel);
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_current_changed(move |row| {
        let Some(ui) = weak.upgrade() else { return };
        let shown = a.with_entry(&a.rows, row, |e| show_slide(&ui, &a.strings.borrow(), e, 0));
        if shown.is_none() {
            ui.set_slide_text("".into());
            ui.set_slide_count(0);
            ui.set_preview_status("".into());
        }
    });

    for step in [-1, 1] {
        let (a, weak) = (app.clone(), ui.as_weak());
        let handler = move || {
            let Some(ui) = weak.upgrade() else { return };
            let target = ui.get_slide_index() + step;
            a.with_entry(&a.rows, ui.get_current_index(), |e| {
                show_slide(&ui, &a.strings.borrow(), e, target)
            });
        };
        if step < 0 {
            ui.on_prev_slide(handler);
        } else {
            ui.on_next_slide(handler);
        }
    }

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_open_current(move || {
        let Some(ui) = weak.upgrade() else { return };
        a.with_entry(&a.rows, ui.get_current_index(), |e| open_path(&e.path, "opening"));
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_reveal_current(move || {
        let Some(ui) = weak.upgrade() else { return };
        a.with_entry(&a.rows, ui.get_current_index(), |e| {
            if let Some(parent) = e.path.parent() {
                open_path(parent, "revealing");
            }
        });
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_project_current(move || {
        let Some(ui) = weak.upgrade() else { return };
        let loaded = a.with_entry(&a.rows, ui.get_current_index(), |e| {
            a.present.borrow_mut().load_hymn(e.number.clone(), e.title.clone(), e.slides.clone());
        });
        if loaded.is_some() {
            crate::control::refresh(&ui, &a);
            ui.set_active_tab(3);
        }
    });
}
