//! Bible tab view. Owned by the Bible tab task.

use crate::app::App;
use crate::msg::Msg;
use crate::view::placeholder;
use iced::Element;

pub fn show(app: &App) -> Element<'_, Msg> {
    placeholder::show(&app.core.strings.tab_bible)
}
