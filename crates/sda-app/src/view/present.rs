//! Present tab view. Left: search and results. Center: preview, slide
//! thumbnails, Go Live and the Original toggle. Right: theme quick picker
//! and history.

use crate::app::App;
use crate::msg::{Msg, PresentMsg, ThemesMsg};
use crate::state::present::{self as st, MAX_THUMBS, relative_time};
use crate::ui_theme::{self, SPACING};
use crate::view::shell::shorten;
use crate::view::themes::theme_tile;
use crate::widgets::card::card;
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{
    Id, button, column, container, responsive, row, scrollable, space, stack, text, text_input,
    toggler, tooltip,
};
use iced::{Alignment, Element, Fill, Shrink, Size};
use sda_core::model::Hymn;

pub use crate::state::present::SEARCH_ID;

const LEFT_WIDTH: f32 = 320.0;
const RIGHT_WIDTH: f32 = 280.0;
/// Every result row has this height so the list reads as a grid.
const ROW_H: f32 = 36.0;
/// Longest title in a result row before the ellipsis.
const TITLE_CHARS: usize = 28;
const THUMB: (f32, f32) = (128.0, 72.0);
/// Inset between the tile border and the slide, so the border shows.
const THUMB_PAD: f32 = 3.0;
const THEME_THUMB: (f32, f32) = (96.0, 54.0);
/// Theme rows shown before the picker scrolls.
const THEME_ROWS: f32 = 6.0;
const THEME_ROW_H: f32 = THEME_THUMB.1 + 12.0;
/// Scrollbar width plus margin, so the search box ends where rows end.
const SCROLL_GUTTER: f32 = 10.0;

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
    let search = container(
        text_input(&s.search_placeholder, &p.query)
            .id(Id::new(SEARCH_ID))
            .on_input(Msg::Search)
            .on_submit(Msg::GoLive)
            .style(ui_theme::input)
            .padding(10)
            .size(14),
    )
    .padding(iced::Padding::ZERO.right(SCROLL_GUTTER));
    let many_libraries = app.core.cfg.libraries.iter().filter(|l| l.enabled).count() > 1;

    let mut list = column![].spacing(2);
    if p.hits.is_empty() {
        list = list.push(
            container(text(&s.no_results).size(13).color(ui_theme::MUTED))
                .padding(SPACING)
                .width(Fill)
                .center_x(Fill),
        );
    }
    for pos in 0..p.hits.len() {
        if let Some(hymn) = hymn_at(app, pos) {
            list = list.push(hymn_row(hymn, p.selected == Some(pos), many_libraries, pos));
        }
    }
    let results = scrollable(container(list).padding(iced::Padding::ZERO.right(SCROLL_GUTTER)))
        .id(p.scroll.clone())
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(6)
                .margin(2)
                .scroller_width(6),
        ))
        .height(Fill)
        .width(Fill);
    card(column![search, results].spacing(SPACING).height(Fill))
        .height(Fill)
        .into()
}

fn hymn_at(app: &App, pos: usize) -> Option<&Hymn> {
    let idx = *app.present.hits.get(pos)?;
    app.core.searcher.as_ref()?.entry(idx)
}

fn hymn_row(hymn: &Hymn, selected: bool, show_library: bool, pos: usize) -> Element<'_, Msg> {
    let number = text(hymn.number.clone().unwrap_or_default())
        .size(14)
        .font(ui_theme::BOLD)
        .width(44);
    let mut line = row![
        number,
        text(shorten(&hymn.title, TITLE_CHARS))
            .size(14)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(4)
    .height(Fill)
    .align_y(Alignment::Center);
    if show_library {
        line = line.push(space::horizontal()).push(
            text(&hymn.library)
                .size(11)
                .color(ui_theme::MUTED)
                .wrapping(text::Wrapping::None),
        );
    }
    button(line)
        .height(ROW_H)
        .padding([0, 10])
        .width(Fill)
        .style(ui_theme::list_row(selected))
        .on_press(Msg::Select(pos))
        .into()
}

/// Height the preview card needs besides the slide: heading, thumbnail
/// strip with its scrollbar, the action row, card padding, and the gaps.
const CHROME_H: f32 =
    16.0 + THUMB.1 + 2.0 * THUMB_PAD + 10.0 + 36.0 + 3.0 * SPACING + 2.0 * SPACING;

/// Largest 16 by 9 slide that leaves room for the rest of the card.
fn slide_size(avail: Size) -> (f32, f32) {
    let h = ((avail.width - 2.0 * SPACING) * 9.0 / 16.0)
        .min(avail.height - CHROME_H)
        .max(90.0);
    (h * 16.0 / 9.0, h)
}

/// The card ends after the action row. `responsive` only measures the
/// column so the slide can be capped by height on a short window.
fn center_panel(app: &App) -> Element<'_, Msg> {
    responsive(move |avail| {
        let (w, h) = slide_size(avail);
        card(
            column![
                ui_theme::section_label(&app.core.strings.preview),
                container(preview(app, w, h)).width(Fill).center_x(Fill),
                thumbs(app),
                actions(app),
            ]
            .spacing(SPACING),
        )
        .width(Fill)
        .into()
    })
    .into()
}

fn preview(app: &App, w: f32, h: f32) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let p = &app.present;
    let inner: Element<'_, Msg> = match st::selected_hymn(app).map(|(_, h)| h) {
        None => text(&s.nothing_selected)
            .size(14)
            .color(ui_theme::MUTED)
            .into(),
        Some(_) if app.gpu_ok => {
            match st::slide_scene(app, p.preview_slide, (w as u32, h as u32)) {
                Some(scene) => scene_view(key("preview"), scene),
                None => space::horizontal().into(),
            }
        }
        Some(hymn) => {
            let body = hymn
                .slides
                .get(p.preview_slide)
                .map(|sl| sl.plain_text())
                .unwrap_or_default();
            container(text(body).size(14).center())
                .padding(SPACING)
                .into()
        }
    };
    container(inner)
        .width(w)
        .height(h)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(ui_theme::thumb_frame)
        .into()
}

fn thumbs(app: &App) -> Element<'_, Msg> {
    let p = &app.present;
    let count = st::selected_hymn(app).map_or(0, |(_, h)| h.slides.len());
    let mut strip = row![].spacing(SPACING / 2.0).align_y(Alignment::Center);
    for i in 0..count.min(MAX_THUMBS) {
        let inner: Element<'_, Msg> = if app.gpu_ok {
            match st::slide_scene(app, i, (THUMB.0 as u32, THUMB.1 as u32)) {
                Some(scene) => scene_view(key(("thumb", i)), scene),
                None => space::horizontal().into(),
            }
        } else {
            space::horizontal().into()
        };
        let number = container(text((i + 1).to_string()).size(10))
            .padding([0, 4])
            .style(ui_theme::tile_label);
        let tile = stack![
            inner,
            container(number)
                .width(Fill)
                .height(Fill)
                .padding(2)
                .align_x(Alignment::Start)
                .align_y(Alignment::Start),
        ];
        strip = strip.push(
            button(tile)
                .width(THUMB.0 + 2.0 * THUMB_PAD)
                .height(THUMB.1 + 2.0 * THUMB_PAD)
                .padding(THUMB_PAD)
                .style(ui_theme::tile(i == p.preview_slide))
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
    scrollable(container(strip).padding(iced::Padding::ZERO.bottom(10)))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new()
                .width(6)
                .margin(2)
                .scroller_width(6),
        ))
        .width(Fill)
        .into()
}

fn actions(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let p = &app.present;
    let selected = st::selected_hymn(app).map(|(_, h)| h);
    let go_live = button(text(&s.go_live).size(14))
        .padding([9, 18])
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
        let toggle = toggler(on)
            .label(s.original.as_str())
            .text_size(13)
            .on_toggle(|b| Msg::Present(crate::msg::PresentMsg::AsIsToggle(b)));
        actions = actions.push(tooltip(
            toggle,
            container(text(&s.original_hint).size(12))
                .padding(8)
                .max_width(280)
                .style(ui_theme::card),
            tooltip::Position::Top,
        ));
    }
    let mut col = column![actions].spacing(4);
    if let Some(err) = &p.as_is_error {
        let danger = ui_theme::theme().palette().danger;
        col = col.push(
            row![
                text(&s.render_failed).size(11).color(danger),
                text(err).size(11).color(danger),
            ]
            .spacing(6),
        );
    }
    col.into()
}

fn side_panel(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let active = app.core.active_theme.name.as_str();
    let now = app.themes.clock.elapsed().as_secs_f32();
    let mut themes = column![].spacing(2);
    for t in &app.core.themes {
        let frame: Element<'_, Msg> = if app.gpu_ok {
            theme_tile(app, t, st::theme_slide(app, t), THEME_THUMB, "theme", now)
        } else {
            container(space::horizontal())
                .width(THEME_THUMB.0)
                .height(THEME_THUMB.1)
                .style(ui_theme::thumb_frame)
                .into()
        };
        let mut line = row![
            frame,
            text(&t.name)
                .size(13)
                .width(Fill)
                .wrapping(text::Wrapping::None)
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center);
        if t.name == active {
            line = line.push(
                container(text(&s.active_suffix).size(11))
                    .padding([2, 8])
                    .style(ui_theme::chip),
            );
        }
        themes = themes.push(
            button(line)
                .height(THEME_ROW_H)
                .padding([0, 6])
                .width(Fill)
                .style(ui_theme::outlined_row(false))
                .on_press(Msg::Themes(ThemesMsg::SetActive(t.name.clone()))),
        );
    }
    let theme_card = card(
        column![
            ui_theme::section_label(&s.theme_label),
            container(scrollable(themes).height(Shrink))
                .max_height(THEME_ROWS * (THEME_ROW_H + 2.0)),
        ]
        .spacing(SPACING),
    )
    .width(Fill);

    let unix_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut rows = column![].spacing(2);
    if app.core.history.is_empty() {
        rows = rows.push(text(&s.history_empty).size(13).color(ui_theme::MUTED));
    }
    for e in &app.core.history {
        let indexed = app
            .core
            .searcher
            .as_ref()
            .is_some_and(|sr| sr.has_path(&e.path));
        let color = if indexed {
            ui_theme::TEXT
        } else {
            ui_theme::MUTED
        };
        let line = row![
            text(e.number.clone().unwrap_or_default())
                .size(13)
                .font(ui_theme::BOLD)
                .color(color)
                .width(44),
            text(shorten(&e.title, 22))
                .size(13)
                .color(color)
                .width(Fill)
                .wrapping(text::Wrapping::None),
            text(relative_time(s, e.opened_at, unix_now))
                .size(11)
                .color(ui_theme::MUTED),
        ]
        .spacing(4)
        .height(Fill)
        .align_y(Alignment::Center);
        rows = rows.push(
            button(line)
                .height(ROW_H)
                .padding([0, 8])
                .width(Fill)
                .style(ui_theme::list_row(false))
                .on_press_maybe(
                    indexed.then(|| Msg::Present(PresentMsg::OpenHistory(e.path.clone()))),
                ),
        );
    }
    let history_card = card(
        column![
            ui_theme::section_label(&s.history),
            scrollable(rows).height(Fill)
        ]
        .spacing(SPACING),
    )
    .width(Fill)
    .height(Fill);
    column![theme_card, history_card]
        .spacing(SPACING)
        .height(Fill)
        .into()
}
