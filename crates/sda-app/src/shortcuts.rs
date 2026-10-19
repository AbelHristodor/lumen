//! Keyboard shortcuts. Runs on every event the widgets did not capture, so
//! nothing fires while a text field has focus.

use crate::msg::{Msg, Tab};
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};
use iced::{Event, event, window};

/// Map a key press to a message. Enter goes live only on the Present tab,
/// so a stray Enter on another tab never changes the projector.
pub fn filter(event: Event, status: event::Status, _id: window::Id, tab: Tab) -> Option<Msg> {
    if status == event::Status::Captured {
        return None;
    }
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    let cmd = modifiers.command();
    Some(match key.as_ref() {
        Key::Character("k") if cmd => Msg::FocusSearch,
        Key::Character("o") if cmd => Msg::ProjectorToggle,
        Key::Character(d @ ("1" | "2" | "3" | "4" | "5")) if cmd => {
            Msg::Tab(Tab::nth(d.parse::<usize>().unwrap_or(1) - 1))
        }
        Key::Character(d)
            if !cmd && d.len() == 1 && d.as_bytes()[0].is_ascii_digit() && d != "0" =>
        {
            Msg::LiveJump(d.parse().unwrap_or(1))
        }
        Key::Character("b") | Key::Character("B") if !cmd => Msg::LiveBlank,
        Key::Named(Named::ArrowRight) | Key::Named(Named::Space) => Msg::LiveNext,
        Key::Named(Named::ArrowLeft) => Msg::LivePrev,
        Key::Named(Named::ArrowUp) => Msg::MoveSelection(-1),
        Key::Named(Named::ArrowDown) => Msg::MoveSelection(1),
        Key::Named(Named::Enter) if tab == Tab::Present => Msg::GoLive,
        Key::Named(Named::Escape) => Msg::LiveStop,
        _ => return None,
    })
}
