//! A padded raised panel.

use crate::ui_theme;
use iced::Element;
use iced::widget::{Container, container};

pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Container<'a, M> {
    container(content)
        .padding(ui_theme::SPACING)
        .style(ui_theme::card)
}
