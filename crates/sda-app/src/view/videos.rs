//! Videos tab view. Owned by the Videos tab task.

use crate::app::App;
use crate::msg::Msg;
use crate::view::placeholder;
use iced::Element;

pub fn show(app: &App) -> Element<'_, Msg> {
    placeholder::show(&app.core.strings.tab_videos)
}
