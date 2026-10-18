//! Settings tab state. Owned by the Settings tab task.

use crate::app::App;
use crate::msg::{Msg, SettingsMsg};
use iced::Task;

#[derive(Default)]
pub struct SettingsState {}

pub fn update(app: &mut App, msg: SettingsMsg) -> Task<Msg> {
    match msg {
        SettingsMsg::Noop => {}
        SettingsMsg::Detected(converter) => app.core.converter = converter,
    }
    Task::none()
}
