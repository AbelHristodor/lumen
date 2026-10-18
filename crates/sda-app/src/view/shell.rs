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
    let chip = |line: &str| {
        container(text(line.to_string()).size(12))
            .padding([4, 10])
            .style(ui_theme::chip)
    };
    let mut chips = row![chip(&app.status.library)].spacing(ui_theme::SPACING / 2.0);
    if let Some(update) = app.status.update.as_deref().filter(|u| !u.is_empty()) {
        chips = chips.push(chip(update));
    }
    chips.into()
}

fn banner(message: &str) -> Element<'_, Msg> {
    container(text(message).size(13))
        .padding([6, ui_theme::SPACING as u16])
        .width(Fill)
        .style(|_| container::Style {
            background: Some(ui_theme::SURFACE_HI.into()),
            ..container::Style::default()
        })
        .into()
}
