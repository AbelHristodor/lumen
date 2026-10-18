//! Themes tab state. Owned by the Themes tab task.

use crate::app::App;
use crate::msg::{Msg, ThemesMsg};
use iced::Task;

#[derive(Default)]
pub struct ThemesState {
    /// Font family names found on this machine, for the font picker.
    pub fonts: Vec<String>,
}

pub fn update(app: &mut App, msg: ThemesMsg) -> Task<Msg> {
    match msg {
        ThemesMsg::Noop => {}
        ThemesMsg::SetActive(name) => {
            if let Some(theme) = app.core.theme_named(&name).cloned() {
                app.core.set_active_theme(theme.clone());
                app.live.set_theme(theme);
            }
        }
    }
    Task::none()
}
