//! Themes tab view. Owned by the Themes tab task.

use crate::app::App;
use crate::msg::Msg;
use crate::view::placeholder;
use iced::Element;

pub fn show(app: &App) -> Element<'_, Msg> {
    placeholder::show(&app.core.strings.tab_themes)
}
