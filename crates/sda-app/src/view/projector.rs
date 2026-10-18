//! The projector window and the small Identify windows.

use crate::app::App;
use crate::msg::Msg;
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{center, container, mouse_area, text};
use iced::{Element, Fill, mouse};

pub fn show(app: &App) -> Element<'_, Msg> {
    if !app.gpu_ok {
        return center(text(&app.core.strings.gpu_unavailable)).into();
    }
    let size = app
        .projector_display
        .and_then(|i| app.displays.get(i))
        .map(|d| {
            let (_, _, w, h) = d.rect;
            ((w as f32 * d.scale) as u32, (h as f32 * d.scale) as u32)
        })
        .unwrap_or((1920, 1080));
    let scene = container(scene_view(key("projector"), app.live.scene(size, app.now)))
        .width(Fill)
        .height(Fill);
    mouse_area(scene)
        .interaction(mouse::Interaction::None)
        .into()
}

/// The display number shown by an Identify window.
pub fn identify<'a>(number: usize) -> Element<'a, Msg> {
    center(text(number.to_string()).size(96)).into()
}
