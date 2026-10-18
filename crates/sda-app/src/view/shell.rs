//! The operator window frame: top bar, the active tab, the Live strip.

use crate::app::App;
use crate::msg::{Msg, Tab};
use crate::ui_theme;
use crate::view::{self, live_strip};
use crate::widgets::segmented::segmented;
use iced::widget::{button, column, container, row, rule, text, tooltip};
use iced::{Alignment, Element, Fill};

/// Longest chip text. The tooltip and the Settings Library section show
/// the full line.
pub const CHIP_CHARS: usize = 48;
/// Chip height, the same as the tab pill so the top bar reads as one row.
const CHIP_HEIGHT: f32 = 34.0;

pub fn show(app: &App) -> Element<'_, Msg> {
    let content = match app.tab {
        Tab::Present => view::present::show(app),
        Tab::Bible => view::bible::show(app),
        Tab::Videos => view::videos::show(app),
        Tab::Themes => view::themes::show(app),
        Tab::Settings => view::settings::show(app),
    };
    let mut body = column![top_bar(app), rule::horizontal(1).style(ui_theme::hairline)];
    if !app.gpu_ok {
        body = body.push(banner(&app.core.strings.gpu_unavailable));
    }
    body.push(
        container(content)
            .height(Fill)
            .width(Fill)
            .padding(iced::Padding::ZERO.top(ui_theme::SPACING)),
    )
    .push(live_strip::show(app))
    .into()
}

fn top_bar(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let tabs = segmented(Tab::ALL.iter().map(|t| (t.label(s), *t)), app.tab, Msg::Tab);
    // Equal side columns keep the tabs centered whatever the chips say.
    row![
        container(text(&s.app_title).size(18).font(ui_theme::BOLD)).width(Fill),
        tabs,
        container(status_chips(app))
            .width(Fill)
            .align_x(iced::Right),
    ]
    .align_y(Alignment::Center)
    .padding(ui_theme::SPACING)
    .spacing(ui_theme::SPACING)
    .into()
}

fn chip<'a>(body: impl Into<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    container(body)
        .height(CHIP_HEIGHT)
        .align_y(Alignment::Center)
        .padding([0, 12])
        .style(ui_theme::chip)
}

fn status_chips(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.status;
    let palette = ui_theme::theme().palette();
    let dot = if st.library_error {
        palette.danger
    } else if st.sync_busy {
        ui_theme::ACCENT
    } else {
        palette.success
    };
    let short = shorten(&st.library, CHIP_CHARS);
    let truncated = short != st.library;
    let library = chip(
        row![
            text("●").size(10).color(dot),
            text(short).size(13).wrapping(text::Wrapping::None),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    let mut chips =
        row![with_tooltip(library, &st.library, truncated)].spacing(ui_theme::SPACING / 2.0);
    if let Some(notice) = &st.notice {
        let short = shorten(notice, CHIP_CHARS);
        let cut = short != *notice;
        let item = chip(
            row![
                text("●").size(10).color(palette.warning),
                text(short).size(13).wrapping(text::Wrapping::None),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        chips = chips.push(with_tooltip(item, notice, cut));
    }
    // An update chip next to a failed library chip reads as a contradiction,
    // so it only shows when the library is fine.
    if !st.library_error
        && let Some(update) = st.update.as_deref().filter(|u| !u.is_empty())
    {
        let item: Element<'_, Msg> = if st.update_staged {
            button(text(&s.update_available).size(13))
                .height(CHIP_HEIGHT)
                .padding([0, 12])
                .style(ui_theme::accent_button)
                .on_press(Msg::OpenReleases)
                .into()
        } else {
            chip(text(shorten(update, CHIP_CHARS)).size(13)).into()
        };
        chips = chips.push(item);
    }
    chips.align_y(Alignment::Center).into()
}

/// Wrap `chip` in a tooltip with `full` when its text was cut.
fn with_tooltip<'a>(
    chip: container::Container<'a, Msg>,
    full: &'a str,
    cut: bool,
) -> Element<'a, Msg> {
    if !cut {
        return chip.into();
    }
    tooltip(
        chip,
        container(text(full).size(12))
            .padding(8)
            .max_width(420)
            .style(ui_theme::card),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Cut `line` to `max` chars with an ellipsis.
pub fn shorten(line: &str, max: usize) -> String {
    if line.chars().count() <= max {
        return line.to_string();
    }
    let head: String = line.chars().take(max - 1).collect();
    format!("{}…", head.trim_end())
}

fn banner(message: &str) -> Element<'_, Msg> {
    let warning = ui_theme::theme().palette().warning;
    container(text(message).size(13).color(warning))
        .padding([6, ui_theme::SPACING as u16])
        .width(Fill)
        .style(|_| container::Style {
            background: Some(ui_theme::SURFACE_HI.into()),
            ..container::Style::default()
        })
        .into()
}
