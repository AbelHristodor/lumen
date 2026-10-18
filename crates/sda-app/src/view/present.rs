//! Present tab view. Owned by the Present tab task.

use crate::app::App;
use crate::msg::Msg;
use crate::view::placeholder;
use iced::Element;

/// Id of the search box, so Cmd+K can focus it.
pub const SEARCH_ID: &str = "search";

pub fn show(app: &App) -> Element<'_, Msg> {
    placeholder::show(&app.core.strings.tab_present)
}
