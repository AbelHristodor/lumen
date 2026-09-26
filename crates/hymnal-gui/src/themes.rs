//! Themes tab: thumbnail list, draft editor, active theme.

use crate::state::App;
use crate::AppWindow;
use hymnal_core::library::themes_dir;
use hymnal_core::theme::{store, Background, HAlign, Rgba, Theme};
use log::{info, warn};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;

pub const WEIGHT_OPTIONS: &[(&str, u16)] = &[
    ("Light", 300),
    ("Regular", 400),
    ("Medium", 500),
    ("Bold", 700),
    ("Black", 900),
];

pub fn to_color(c: Rgba) -> slint::Color {
    slint::Color::from_argb_u8(c.a, c.r, c.g, c.b)
}

/// The colour a theme thumbnail or projector paints behind the text.
pub fn bg_color(t: &Theme) -> Rgba {
    match &t.background.kind {
        Background::Solid { color } => *color,
        Background::Gradient { from, .. } => *from,
        Background::Image { .. } => t.text.color,
    }
}

pub fn h_align_name(a: HAlign) -> &'static str {
    match a {
        HAlign::Left => "left",
        HAlign::Center => "center",
        HAlign::Right => "right",
    }
}

fn strings_model(items: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    let v: Vec<SharedString> = items.into_iter().map(Into::into).collect();
    ModelRc::from(Rc::new(VecModel::from(v)))
}

fn weight_index(w: u16) -> i32 {
    WEIGHT_OPTIONS.iter().position(|(_, v)| *v == w).unwrap_or(3) as i32
}

/// Reload themes from disk into the thumbnail models.
fn refresh_list(ui: &AppWindow, app: &App) {
    let list = themes_dir()
        .map(|d| store::list_themes(&d))
        .unwrap_or_else(|| vec![Theme::default()]);
    let active = app.active_theme.borrow().name.clone();
    ui.set_theme_names(strings_model(list.iter().map(|t| t.name.clone())));
    ui.set_theme_fonts(strings_model(list.iter().map(|t| t.text.font_family.clone())));
    let colors = |f: fn(&Theme) -> Rgba| {
        let v: Vec<slint::Color> = list.iter().map(|t| to_color(f(t))).collect();
        ModelRc::from(Rc::new(VecModel::from(v)))
    };
    ui.set_theme_bgs(colors(bg_color));
    ui.set_theme_fgs(colors(|t| t.text.color));
    ui.set_active_index(list.iter().position(|t| t.name == active).unwrap_or(0) as i32);
    *app.themes.borrow_mut() = list;
}

fn load_into_editor(ui: &AppWindow, app: &App, t: &Theme) {
    let fidx = app
        .families
        .iter()
        .position(|f| f.eq_ignore_ascii_case(&t.text.font_family))
        .unwrap_or(0) as i32;
    ui.set_edit_font_index(fidx);
    ui.set_edit_weight_index(weight_index(t.text.font_weight));
    ui.set_edit_font_size(t.text.font_size_pt.unwrap_or(44.0));
    ui.set_edit_text_color(to_color(t.text.color));
    ui.set_edit_text_hex(t.text.color.to_hex().into());
    let bg = bg_color(t);
    ui.set_edit_bg_color(to_color(bg));
    ui.set_edit_bg_hex(bg.to_hex().into());
    ui.set_edit_h_align(h_align_name(t.text.h_align).into());
    ui.set_edit_name(t.name.clone().into());
}

fn set_active(ui: &AppWindow, app: &App, t: Theme) {
    app.update_cfg(|c| c.active_theme = Some(t.name.clone()));
    *app.active_theme.borrow_mut() = t;
    crate::control::push_to_projector(app);
    refresh_list(ui, app);
}

fn set_draft_text_color(ui: &AppWindow, app: &App, rgba: Rgba) {
    app.draft.borrow_mut().text.color = rgba;
    ui.set_edit_text_color(to_color(rgba));
    ui.set_edit_text_hex(rgba.to_hex().into());
}

fn set_draft_bg_color(ui: &AppWindow, app: &App, rgba: Rgba) {
    app.draft.borrow_mut().background.kind = Background::Solid { color: rgba };
    ui.set_edit_bg_color(to_color(rgba));
    ui.set_edit_bg_hex(rgba.to_hex().into());
}

fn slint_rgba(c: slint::Color) -> Rgba {
    Rgba::new(c.red(), c.green(), c.blue(), 255)
}

pub fn wire(ui: &AppWindow, app: &App) {
    ui.set_font_families(strings_model(app.families.iter().cloned()));
    ui.set_weight_options(strings_model(WEIGHT_OPTIONS.iter().map(|(l, _)| l.to_string())));
    refresh_list(ui, app);
    load_into_editor(ui, app, &app.draft.borrow());
    ui.set_selected_index(0);

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_theme_selected(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        let selected = a.themes.borrow().get(i.max(0) as usize).cloned();
        if let Some(t) = selected {
            load_into_editor(&ui, &a, &t);
            *a.draft.borrow_mut() = t;
            ui.set_selected_index(i);
        }
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_font_picked(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(f) = a.families.get(i.max(0) as usize) {
            a.draft.borrow_mut().text.font_family = f.clone();
            ui.set_edit_font_index(i);
        }
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_weight_picked(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        let w = WEIGHT_OPTIONS.get(i.max(0) as usize).map_or(400, |(_, v)| *v);
        a.draft.borrow_mut().text.font_weight = w;
        ui.set_edit_weight_index(i);
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_size_changed(move |v| {
        let Some(ui) = weak.upgrade() else { return };
        a.draft.borrow_mut().text.font_size_pt = Some(v);
        ui.set_edit_font_size(v);
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_align_picked(move |name| {
        let Some(ui) = weak.upgrade() else { return };
        a.draft.borrow_mut().text.h_align = match name.as_str() {
            "left" => HAlign::Left,
            "right" => HAlign::Right,
            _ => HAlign::Center,
        };
        ui.set_edit_h_align(name);
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_text_hex_edited(move |t| {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(rgba) = Rgba::from_hex(&t) {
            set_draft_text_color(&ui, &a, rgba);
        }
    });
    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_text_preset(move |c| {
        let Some(ui) = weak.upgrade() else { return };
        set_draft_text_color(&ui, &a, slint_rgba(c));
    });
    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_bg_hex_edited(move |t| {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(rgba) = Rgba::from_hex(&t) {
            set_draft_bg_color(&ui, &a, rgba);
        }
    });
    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_bg_preset(move |c| {
        let Some(ui) = weak.upgrade() else { return };
        set_draft_bg_color(&ui, &a, slint_rgba(c));
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_new_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let t = Theme { name: "Custom".into(), ..Theme::default() };
        load_into_editor(&ui, &a, &t);
        *a.draft.borrow_mut() = t;
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_save_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let mut t = a.draft.borrow().clone();
        t.name = ui.get_edit_name().to_string();
        if t.name.is_empty() || t.is_builtin_default() {
            t.name = "Custom".into();
        }
        if let Some(dir) = themes_dir() {
            match store::save_theme(&dir, &t) {
                Ok(()) => info!("saved theme {}", t.name),
                Err(e) => warn!("save theme failed: {e}"),
            }
        }
        *a.draft.borrow_mut() = t;
        refresh_list(&ui, &a);
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_delete_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let idx = ui.get_selected_index().max(0) as usize;
        let Some(name) = a.themes.borrow().get(idx).map(|t| t.name.clone()) else { return };
        if let Some(dir) = themes_dir() {
            if let Err(e) = store::delete_theme(&dir, &name) {
                warn!("delete theme failed: {e}");
            }
        }
        if a.active_theme.borrow().name == name {
            set_active(&ui, &a, Theme::default());
        } else {
            refresh_list(&ui, &a);
        }
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_set_active(move || {
        let Some(ui) = weak.upgrade() else { return };
        let idx = ui.get_selected_index().max(0) as usize;
        let selected = a.themes.borrow().get(idx).cloned();
        if let Some(t) = selected {
            set_active(&ui, &a, t);
        }
    });
}
