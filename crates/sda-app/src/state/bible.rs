//! Bible tab state. Owned by the Bible tab task.

use crate::app::App;
use crate::msg::{BibleMsg, Msg};
use iced::Task;

#[derive(Default)]
pub struct BibleState {}

pub fn update(_app: &mut App, msg: BibleMsg) -> Task<Msg> {
    match msg {
        BibleMsg::Noop => Task::none(),
    }
}
