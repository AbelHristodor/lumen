//! The projector window: one scene filling the window.

use crate::app::App;
use crate::msg::Msg;
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{center, container, text};
use iced::{Element, Fill};
use std::time::Instant;

pub fn show(app: &App) -> Element<'_, Msg> {
    if !app.gpu_ok {
        return center(text(&app.core.strings.gpu_unavailable)).into();
    }
    container(scene_view(
        key("projector"),
        app.live.scene((1920, 1080), Instant::now()),
    ))
    .width(Fill)
    .height(Fill)
    .into()
}
