//! Present tab view. Left: search and results with a Recent section. Center:
//! preview, slide thumbnails, Go Live and the Original toggle. Right: theme
//! quick picker and notes.

use crate::app::App;
use crate::msg::{Msg, ThemesMsg};
use crate::state::present::{self as st, MAX_THUMBS, RECENT_COUNT};
use crate::ui_theme::{self, SPACING};
use crate::widgets::card::card;
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{
    Id, button, column, container, responsive, row, rule, scrollable, space, text, text_input,
    toggler,
};
use iced::{Alignment, Border, Color, Element, Fill, Size, Theme};
use sda_core::history;
use sda_core::model::Hymn;

pub use crate::state::present::SEARCH_ID;

const LEFT_WIDTH: f32 = 320.0;
const RIGHT_WIDTH: f32 = 280.0;
const THUMB: (f32, f32) = (128.0, 72.0);
const THEME_THUMB: (f32, f32) = (96.0, 54.0);

pub fn show(app: &App) -> Element<'_, Msg> {
    row![
        container(search_panel(app)).width(LEFT_WIDTH),
        center_panel(app),
        container(side_panel(app)).width(RIGHT_WIDTH),
    ]
    .spacing(SPACING)
    .padding([0, SPACING as u16])
    .height(Fill)
    .into()
}

fn search_panel(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let p = &app.present;
    let search = text_input(&s.search_placeholder, &p.query)
        .id(Id::new(SEARCH_ID))
        .on_input(Msg::Search)
        .on_submit(Msg::GoLive)
        .padding(10)
        .size(14);

    let mut list = column![].spacing(2);
    if p.query.trim().is_empty() && !app.core.history.is_empty() {
        list = list.push(heading(&s.recent));
        for e in history::recent(&app.core.history, RECENT_COUNT) {
            if let Some(pos) = st::hit_position(app, &e.path)
                && let Some(hymn) = hymn_at(app, pos)
            {
                list = list.push(hymn_row(hymn, p.selected == Some(pos), Msg::Select(pos)));
            }
        }
        list = list.push(
            container(rule::horizontal(1))
                .padding([SPACING as u16 / 2, 0])
                .width(Fill),
        );
    }
    if p.hits.is_empty() {
        list = list.push(
            container(text(&s.search_placeholder).size(13).color(ui_theme::MUTED))
                .padding(SPACING)
                .width(Fill)
                .center_x(Fill),
        );
    }
    for pos in 0..p.hits.len() {
        if let Some(hymn) = hymn_at(app, pos) {
            list = list.push(hymn_row(hymn, p.selected == Some(pos), Msg::Select(pos)));
        }
    }
    let results = scrollable(list)
        .id(p.scroll.clone())
        .height(Fill)
        .width(Fill);
    column![search, results]
        .spacing(SPACING)
        .height(Fill)
        .into()
}

fn hymn_at(app: &App, pos: usize) -> Option<&Hymn> {
    let idx = *app.present.hits.get(pos)?;
    app.core.searcher.as_ref()?.entry(idx)
}

fn heading(label: &str) -> Element<'_, Msg> {
    container(
        text(label)
            .size(12)
            .font(ui_theme::BOLD)
            .color(ui_theme::MUTED),
    )
    .padding([6, 10])
    .into()
}

fn hymn_row<'a>(hymn: &'a Hymn, selected: bool, msg: Msg) -> Element<'a, Msg> {
    let number = text(hymn.number.clone().unwrap_or_default())
        .size(14)
        .font(ui_theme::BOLD)
        .width(52);
    let title = text(&hymn.title).size(14).width(Fill);
    let library = container(text(&hymn.library).size(11))
        .padding([2, 8])
        .style(ui_theme::chip);
    button(
        row![number, title, library]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .padding([8, 10])
    .width(Fill)
    .style(row_style(selected))
    .on_press(msg)
    .into()
}

/// A list row. The selected row carries a tinted accent surface.
fn row_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if selected {
            Color {
                a: 0.28,
                ..ui_theme::ACCENT
            }
        } else if status == button::Status::Hovered {
            ui_theme::SURFACE_HI
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(bg.into()),
            text_color: ui_theme::TEXT,
            border: Border::default().rounded(ui_theme::RADIUS - 2.0),
            ..button::Style::default()
        }
    }
}

/// A thumbnail button. The active one gets an accent border.
fn thumb_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: Some(Color::BLACK.into()),
        text_color: ui_theme::TEXT,
        border: Border {
            color: if active {
                ui_theme::ACCENT
            } else {
                ui_theme::SURFACE_HI
            },
            width: 2.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

/// Largest 16 by 9 box inside `avail`.
fn fit_16_9(avail: Size) -> (f32, f32) {
    let w = avail.width.min(avail.height * 16.0 / 9.0).max(16.0);
    (w, w * 9.0 / 16.0)
}

fn center_panel(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let p = &app.present;
    let selected = st::selected_hymn(app).map(|(_, h)| h);

    let preview: Element<'_, Msg> = match selected {
        None => container(text(&s.search_placeholder).size(14).color(ui_theme::MUTED))
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .into(),
        Some(hymn) => responsive(move |avail| {
            let (w, h) = fit_16_9(avail);
            let inner: Element<'_, Msg> = if app.gpu_ok {
                match st::slide_scene(app, p.preview_slide, (w as u32, h as u32)) {
                    Some(scene) => scene_view(key("preview"), scene),
                    None => space::horizontal().into(),
                }
            } else {
                let body = hymn
                    .slides
                    .get(p.preview_slide)
                    .map(|sl| sl.plain_text())
                    .unwrap_or_default();
                container(text(body).size(14).center())
                    .padding(SPACING)
                    .center(Fill)
                    .into()
            };
            container(
                container(inner)
                    .width(w)
                    .height(h)
                    .style(ui_theme::thumb_frame),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .into()
        })
        .into(),
    };

    let mut strip = row![].spacing(SPACING / 2.0).align_y(Alignment::Center);
    let count = selected.map_or(0, |h| h.slides.len());
    for i in 0..count.min(MAX_THUMBS) {
        let inner: Element<'_, Msg> = if app.gpu_ok {
            match st::slide_scene(app, i, (THUMB.0 as u32, THUMB.1 as u32)) {
                Some(scene) => scene_view(key(("thumb", i)), scene),
                None => space::horizontal().into(),
            }
        } else {
            container(text((i + 1).to_string()).size(12))
                .center(Fill)
                .into()
        };
        strip = strip.push(
            button(inner)
                .width(THUMB.0)
                .height(THUMB.1)
                .padding(0)
                .style(thumb_style(i == p.preview_slide))
                .on_press(Msg::PreviewSlide(i)),
        );
    }
    if count > MAX_THUMBS {
        strip = strip.push(
            text(format!("+{}", count - MAX_THUMBS))
                .size(13)
                .color(ui_theme::MUTED),
        );
    }
    let thumbs = scrollable(container(strip).padding(iced::Padding::ZERO.bottom(8)))
        .horizontal()
        .width(Fill);

    let go_live = button(text(&s.go_live).size(14))
        .padding([8, 18])
        .style(ui_theme::accent_button)
        .on_press_maybe(selected.map(|_| Msg::GoLiveAt(p.preview_slide)));
    let mut actions = row![go_live].spacing(SPACING).align_y(Alignment::Center);
    if p.as_is_busy {
        actions = actions.push(
            container(text(&s.status_rendering).size(12))
                .padding([4, 10])
                .style(ui_theme::chip),
        );
    }
    actions = actions.push(space::horizontal());
    if let (Some(hymn), Some(_)) = (selected, &app.core.converter) {
        let on = st::as_is_on(app, hymn);
        let mut col = column![
            toggler(on)
                .label(s.original.as_str())
                .text_size(13)
                .on_toggle(|b| Msg::Present(crate::msg::PresentMsg::AsIsToggle(b))),
            text(&s.original_hint).size(11).color(ui_theme::MUTED),
        ]
        .spacing(2)
        .align_x(Alignment::End);
        if let Some(err) = &p.as_is_error {
            col = col.push(text(err).size(11).color(ui_theme::theme().palette().danger));
        }
        actions = actions.push(col);
    }

    let body = column![
        text(&s.preview)
            .size(12)
            .font(ui_theme::BOLD)
            .color(ui_theme::MUTED),
        preview,
        thumbs,
        actions,
    ]
    .spacing(SPACING)
    .height(Fill);
    card(body).width(Fill).height(Fill).into()
}

fn side_panel(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let active = app.core.active_theme.name.as_str();
    let mut themes = column![].spacing(2);
    for t in &app.core.themes {
        let thumb: Element<'_, Msg> = if app.gpu_ok {
            match st::sample_scene(
                app,
                t,
                &s.theme_preview_sample,
                (THEME_THUMB.0 as u32, THEME_THUMB.1 as u32),
            ) {
                Some(scene) => scene_view(key(("theme", t.name.as_str())), scene),
                None => space::horizontal().into(),
            }
        } else {
            space::horizontal().into()
        };
        let frame = container(thumb)
            .width(THEME_THUMB.0)
            .height(THEME_THUMB.1)
            .style(ui_theme::thumb_frame);
        themes = themes.push(
            button(
                row![frame, text(&t.name).size(13).width(Fill)]
                    .spacing(SPACING)
                    .align_y(Alignment::Center),
            )
            .padding(6)
            .width(Fill)
            .style(row_style(t.name == active))
            .on_press(Msg::Themes(ThemesMsg::SetActive(t.name.clone()))),
        );
    }
    let theme_card = card(
        column![
            text(&s.theme_label)
                .size(12)
                .font(ui_theme::BOLD)
                .color(ui_theme::MUTED),
            scrollable(themes).height(Fill),
        ]
        .spacing(SPACING),
    )
    .height(Fill);

    let notes = st::selected_hymn(app)
        .and_then(|(_, h)| h.slides.get(app.present.preview_slide))
        .and_then(|sl| sl.notes.as_deref())
        .filter(|n| !n.trim().is_empty());
    let mut col = column![theme_card].spacing(SPACING).height(Fill);
    if let Some(notes) = notes {
        col = col.push(
            card(
                column![
                    text(&s.notes)
                        .size(12)
                        .font(ui_theme::BOLD)
                        .color(ui_theme::MUTED),
                    scrollable(text(notes).size(13)).height(Fill),
                ]
                .spacing(SPACING),
            )
            .height(Fill),
        );
    }
    col.into()
}
