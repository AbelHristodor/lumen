//! The operator window frame: top bar, the active tab, the Live strip.

use crate::app::App;
use crate::msg::{Msg, Tab};
use crate::ui_theme;
use crate::view::{self, live_strip};
use crate::widgets::segmented::segmented;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Fill};

pub fn show(app: &App) -> Element<'_, Msg> {
    let content = match app.tab {
        Tab::Present => view::present::show(app),
        Tab::Bible => view::bible::show(app),
        Tab::Videos => view::videos::show(app),
        Tab::Themes => view::themes::show(app),
        Tab::Settings => view::settings::show(app),
    };
    let mut body = column![top_bar(app)];
    if !app.gpu_ok {
        body = body.push(banner(&app.core.strings.gpu_unavailable));
    }
    body.push(container(content).height(Fill).width(Fill))
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

fn status_chips(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.status;
    let chip = |line: &str, dot: Option<iced::Color>| {
        let mut body = row![].spacing(6).align_y(Alignment::Center);
        if let Some(color) = dot {
            body = body.push(text("●").size(10).color(color));
        }
        container(body.push(text(shorten(line, CHIP_CHARS)).size(12)))
            .padding([4, 10])
            .style(ui_theme::chip)
    };
    let danger = ui_theme::theme().palette().danger;
    let library = chip(&st.library, st.library_error.then_some(danger));
    let mut chips = row![library].spacing(ui_theme::SPACING / 2.0);
    // An update chip next to a failed library chip reads as a contradiction,
    // so it only shows when the library is fine.
    if !st.library_error
        && let Some(update) = st.update.as_deref().filter(|u| !u.is_empty())
    {
        let label = if st.update_staged {
            s.update_available.as_str()
        } else {
            update
        };
        chips = chips.push(chip(label, None));
    }
    chips.into()
}

/// Longest chip text. The Settings Library section shows the full line.
const CHIP_CHARS: usize = 72;

fn shorten(line: &str, max: usize) -> String {
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
