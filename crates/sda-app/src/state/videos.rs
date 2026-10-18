//! Videos tab state. Owned by the Videos tab task.

use crate::app::App;
use crate::msg::{Msg, VideosMsg};
use iced::Task;

#[derive(Default)]
pub struct VideosState {}

pub fn update(_app: &mut App, msg: VideosMsg) -> Task<Msg> {
    match msg {
        VideosMsg::Noop => Task::none(),
    }
}
