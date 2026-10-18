//! Themes tab view. Owned by the Themes tab task. Theme cards on the left,
//! the live preview and the form on the right.

use crate::app::App;
use crate::msg::{Msg, ThemesMsg};
use crate::state::themes::{PREVIEW_SAMPLE, sorted_themes, thumb_slide};
use crate::ui_theme;
use crate::widgets::card::card;
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{
    Column, button, checkbox, column, container, row, scrollable, space, stack, text,
};
use iced::{Alignment, Border, Color, Element, Fill};
use sda_core::resolve::ResolvedSlide;
use sda_core::theme::{Background, Rgba, Theme};
use sda_render::{BackgroundState, Layer, Scene};
use std::sync::Arc;
use std::time::Instant;

/// Lives next to this file. Declared here so `view/mod.rs` stays untouched.
#[path = "theme_form.rs"]
pub mod theme_form;

const LIST_WIDTH: f32 = 300.0;
const THUMB_W: f32 = 96.0;
const THUMB_H: f32 = 54.0;

pub fn show(app: &App) -> Element<'_, Msg> {
    row![theme_list(app), editor(app)]
        .spacing(ui_theme::SPACING)
        .padding([0, ui_theme::SPACING as u16])
        .height(Fill)
        .into()
}

pub fn to_color(c: Rgba) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0)
}

/// The renderer side of a background with no decoded files. Image and
/// video show black, which is what the live path shows before they load.
fn background_state(bg: &Background) -> BackgroundState {
    match bg {
        Background::Solid(c) => BackgroundState::Solid(*c),
        Background::Gradient {
            stops,
            angle,
            drift,
        } => BackgroundState::Gradient {
            stops: stops.clone(),
            angle: *angle,
            drift: *drift,
        },
        Background::Particles {
            base,
            color,
            count,
            speed,
        } => BackgroundState::Particles {
            base: *base,
            color: *color,
            count: *count,
            speed: *speed,
        },
        Background::Image(_) | Background::Video { .. } => BackgroundState::Solid(Rgba::BLACK),
    }
}

/// The background a card thumbnail shows. A video or image already decoded
/// for the projector or the preview is reused, so the card shows the real
/// frame. `None` when the file is not loaded anywhere.
fn thumb_background(app: &App, bg: &Background) -> Option<BackgroundState> {
    let states = [&app.live, &app.themes.preview];
    match bg {
        Background::Video { path } => states
            .iter()
            .find(|l| l.theme.background == *bg)
            .and_then(|l| l.video.clone())
            .map(BackgroundState::Video)
            .or_else(|| {
                states
                    .iter()
                    .find_map(|l| l.bitmaps.get(path))
                    .map(|b| BackgroundState::Image(b.clone()))
            }),
        Background::Image(path) => states
            .iter()
            .find_map(|l| l.bitmaps.get(path))
            .map(|b| BackgroundState::Image(b.clone())),
        other => Some(background_state(other)),
    }
}

fn thumb_scene(
    background: BackgroundState,
    theme: &Theme,
    slide: Option<&Arc<ResolvedSlide>>,
    now: f32,
) -> Arc<Scene> {
    Arc::new(Scene {
        size: (THUMB_W as u32 * 2, THUMB_H as u32 * 2),
        background,
        background_opacity: theme.background_opacity,
        current: slide.map(|s| Layer::Text(s.clone())),
        outgoing: None,
        transition: theme.transition,
        blank: false,
        now_secs: now,
    })
}

fn swatch_color(bg: &Background) -> Color {
    match bg {
        Background::Solid(c) | Background::Particles { base: c, .. } => to_color(*c),
        Background::Gradient { stops, .. } => stops.first().map_or(Color::BLACK, |s| to_color(s.1)),
        Background::Image(_) | Background::Video { .. } => Color::BLACK,
    }
}

fn theme_card<'a>(app: &'a App, i: usize, theme: &'a Theme, now: f32) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let thumb: Element<'a, Msg> = if app.gpu_ok {
        // Before the first Themes message the cache is empty, so resolve here.
        let slide = match app.themes.thumbs.get(&theme.cache_key()) {
            Some(slide) => Some(slide.clone()),
            None => thumb_slide(theme, s).map(Arc::new),
        };
        let background = thumb_background(app, &theme.background);
        let missing = background.is_none();
        let scene = scene_view(
            key(("theme_card", theme.name.as_str())),
            thumb_scene(
                background.unwrap_or(BackgroundState::Solid(Rgba::BLACK)),
                theme,
                slide.as_ref(),
                now,
            ),
        );
        let mut layers = stack![scene];
        if missing {
            // The file is not loaded, so name the kind instead of showing
            // a bare black tile.
            let kind = match &theme.background {
                Background::Video { .. } => s.theme_bg_video.as_str(),
                _ => s.theme_bg_image.as_str(),
            };
            layers = layers.push(
                container(
                    container(text(kind).size(10))
                        .padding([1, 5])
                        .style(ui_theme::tile_label),
                )
                .width(Fill)
                .height(Fill)
                .padding(3)
                .align_x(Alignment::End)
                .align_y(Alignment::End),
            );
        }
        container(layers)
            .width(THUMB_W)
            .height(THUMB_H)
            .style(ui_theme::thumb_frame)
            .into()
    } else {
        let color = swatch_color(&theme.background);
        container(space::horizontal())
            .width(THUMB_W)
            .height(THUMB_H)
            .style(move |_| container::Style {
                background: Some(color.into()),
                border: Border::default().rounded(4.0),
                ..container::Style::default()
            })
            .into()
    };
    let mut info = row![text(theme.name.as_str()).size(14)]
        .spacing(6)
        .align_y(Alignment::Center);
    if theme.favorite {
        info = info.push(text("★").size(13).color(ui_theme::ACCENT));
    }
    if theme.name == app.core.active_theme.name {
        info = info.push(
            container(text(s.active_suffix.as_str()).size(11))
                .padding([2, 8])
                .style(ui_theme::chip),
        );
    }
    let selected = app.themes.draft_index == i;
    button(
        row![thumb, info]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(6)
    .style(ui_theme::list_row(selected))
    .on_press(Msg::Themes(ThemesMsg::Select(i)))
    .into()
}

fn theme_list(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let now = app.themes.clock.elapsed().as_secs_f32();
    let ghost = |label: &str, msg: Option<Msg>| {
        button(text(label.to_string()).size(13))
            .padding([6, 12])
            .style(ui_theme::ghost_button)
            .on_press_maybe(msg)
    };
    let can_delete = !app.themes.draft.is_builtin_default();
    let toolbar = row![
        ghost(&s.theme_new, Some(Msg::Themes(ThemesMsg::New))),
        ghost(&s.theme_duplicate, Some(Msg::Themes(ThemesMsg::Duplicate))),
        ghost(
            &s.theme_delete,
            can_delete.then_some(Msg::Themes(ThemesMsg::Delete))
        ),
    ]
    .spacing(ui_theme::SPACING / 2.0);
    let cards = sorted_themes(&app.core)
        .into_iter()
        .enumerate()
        .fold(Column::new().spacing(4), |col, (i, t)| {
            col.push(theme_card(app, i, t, now))
        });
    card(
        column![
            text(s.themes_heading.as_str())
                .size(16)
                .font(ui_theme::BOLD),
            toolbar,
            scrollable(cards).height(Fill),
        ]
        .spacing(ui_theme::SPACING),
    )
    .width(LIST_WIDTH)
    .height(Fill)
    .into()
}

fn preview(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let frame: Element<'_, Msg> = if app.gpu_ok {
        container(scene_view(
            key("theme_preview"),
            app.themes.preview.scene((960, 540), Instant::now()),
        ))
        .width(Fill)
        .height(300)
        .style(ui_theme::thumb_frame)
        .into()
    } else {
        let color = swatch_color(&app.themes.draft.background);
        container(text(PREVIEW_SAMPLE).size(20).center())
            .width(Fill)
            .height(300)
            .center_x(Fill)
            .center_y(Fill)
            .style(move |_| container::Style {
                background: Some(color.into()),
                border: Border::default().rounded(4.0),
                ..container::Style::default()
            })
            .into()
    };
    let draft = &app.themes.draft;
    let saved = app.core.theme_named(&draft.name).is_some();
    let is_active = app.core.active_theme.name == draft.name;
    let mut toolbar = row![
        button(text(s.theme_save.as_str()).size(13))
            .padding([6, 14])
            .style(ui_theme::accent_button)
            .on_press_maybe((!draft.is_builtin_default()).then_some(Msg::Themes(ThemesMsg::Save))),
        button(text(s.theme_set_active.as_str()).size(13))
            .padding([6, 14])
            .style(ui_theme::ghost_button)
            .on_press_maybe(
                (saved && !is_active)
                    .then_some(Msg::Themes(ThemesMsg::SetActive(draft.name.clone())))
            ),
        checkbox(draft.favorite)
            .label(s.theme_favorite.as_str())
            .size(16)
            .text_size(13)
            .on_toggle_maybe(
                (!draft.is_builtin_default()).then_some(|f| Msg::Themes(ThemesMsg::Favorite(f)))
            ),
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    if let Some(e) = &app.themes.error {
        toolbar = toolbar.push(text(e.as_str()).size(12).color(ui_theme::MUTED));
    }
    card(
        column![
            text(s.preview.as_str()).size(16).font(ui_theme::BOLD),
            frame,
            toolbar,
        ]
        .spacing(ui_theme::SPACING),
    )
    .width(Fill)
    .into()
}

fn editor(app: &App) -> Element<'_, Msg> {
    scrollable(
        column![preview(app), theme_form::show(app)]
            .spacing(ui_theme::SPACING)
            .padding([0, ui_theme::SPACING as u16 + 4]),
    )
    .height(Fill)
    .width(Fill)
    .into()
}
