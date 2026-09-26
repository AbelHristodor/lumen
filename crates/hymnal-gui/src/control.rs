//! Control tab: what is projected, on which display, and the projector window.

use crate::state::App;
use crate::themes::{bg_color, h_align_name, to_color};
use crate::{projector, AppWindow};
use slint::ComponentHandle;

/// Mirror the presentation state into the Control tab.
fn refresh_view(ui: &AppWindow, app: &App) {
    let p = app.present.borrow();
    let title = match &p.number {
        Some(n) => format!("{n}. {}", p.title),
        None => p.title.clone(),
    };
    ui.set_ctl_title(title.into());
    ui.set_ctl_live(p.current_slide().unwrap_or("").into());
    ui.set_ctl_next(p.next_slide().unwrap_or("").into());
    let pos = if p.slide_count() > 0 {
        format!("{}/{}", p.index + 1, p.slide_count())
    } else {
        String::new()
    };
    ui.set_ctl_pos(pos.into());
    ui.set_ctl_blank(p.blank);
}

/// Push the current slide and active theme onto the projector, if open.
pub fn push_to_projector(app: &App) {
    let projector = app.projector.borrow();
    let Some(win) = projector.as_ref() else { return };
    let theme = app.active_theme.borrow();
    let p = app.present.borrow();
    win.set_slide_text(p.current_slide().unwrap_or("").into());
    win.set_blank(p.blank);
    win.set_text_color(to_color(theme.text.color));
    win.set_font_family(theme.text.font_family.clone().into());
    win.set_font_size(theme.text.font_size_pt.unwrap_or(44.0));
    win.set_font_weight(theme.text.font_weight as i32);
    win.set_margin(theme.layout.margin);
    win.set_h_align(h_align_name(theme.text.h_align).into());
    win.set_bg_color(to_color(bg_color(&theme)));
}

/// Refresh both the Control view and the projector after a state change.
pub fn refresh(ui: &AppWindow, app: &App) {
    refresh_view(ui, app);
    push_to_projector(app);
}

pub fn wire(ui: &AppWindow, app: &App) {
    let names: Vec<slint::SharedString> = app.displays.iter().map(|d| d.label.as_str().into()).collect();
    ui.set_display_names(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(names))));
    ui.set_display_index(
        app.cfg
            .borrow()
            .output_display
            .unwrap_or_else(|| projector::default_display_index(&app.displays)),
    );

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_ctl_start(move || {
        let Some(ui) = weak.upgrade() else { return };
        if a.projector.borrow().is_some() {
            return;
        }
        if let Some(win) = projector::open_projector(&a.displays, ui.get_display_index()) {
            *a.projector.borrow_mut() = Some(win);
            ui.set_ctl_projecting(true);
            push_to_projector(&a);
        }
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_ctl_stop(move || {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(win) = a.projector.borrow_mut().take() {
            let _ = win.hide();
        }
        ui.set_ctl_projecting(false);
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_ctl_go_next(move || {
        let Some(ui) = weak.upgrade() else { return };
        a.present.borrow_mut().next();
        refresh(&ui, &a);
    });
    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_ctl_prev(move || {
        let Some(ui) = weak.upgrade() else { return };
        a.present.borrow_mut().prev();
        refresh(&ui, &a);
    });
    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_ctl_blank_toggle(move || {
        let Some(ui) = weak.upgrade() else { return };
        a.present.borrow_mut().toggle_blank();
        refresh(&ui, &a);
    });

    let a = app.clone();
    ui.on_ctl_display_picked(move |i| a.update_cfg(|c| c.output_display = Some(i)));

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_ctl_search_changed(move |q| {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(model) = crate::library::search_rows(&a, &a.ctl_rows, &q) {
            ui.set_ctl_search_results(model);
        }
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_ctl_search_activated(move |row| {
        let Some(ui) = weak.upgrade() else { return };
        let loaded = a.with_entry(&a.ctl_rows, row, |e| {
            a.present.borrow_mut().load_hymn(e.number.clone(), e.title.clone(), e.slides.clone());
        });
        if loaded.is_some() {
            refresh(&ui, &a);
        }
    });
}
