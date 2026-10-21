//! Stand in body for a tab that is not built yet.

use crate::msg::Msg;
use crate::ui_theme;
use iced::widget::{center, text};
use iced::{Element, Fill};

pub fn show<'a>(title: &'a str) -> Element<'a, Msg> {
    center(text(title).size(28).color(ui_theme::palette().muted))
        .width(Fill)
        .height(Fill)
        .into()
}
