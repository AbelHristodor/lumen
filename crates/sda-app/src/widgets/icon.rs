//! Icons from a Lucide font subset, see `fonts/LICENSE-lucide.txt`. One
//! glyph is one `text` in the icon font, so it sizes and colors like text.

use iced::Font;
use iced::widget::{Text, text};

pub const FONT: Font = Font::with_name("lucide");
pub const BYTES: &[u8] = include_bytes!("../../fonts/lucide-subset.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Settings,
    Search,
    Play,
    Square,
    ChevronLeft,
    ChevronRight,
    SkipBack,
    SkipForward,
    Keyboard,
    X,
    CircleX,
    CircleHelp,
    Monitor,
    MonitorOff,
    EyeOff,
    History,
    BookOpen,
    CornerDownLeft,
}

impl Icon {
    pub fn codepoint(self) -> char {
        let c = match self {
            Icon::Settings => 0xe154,
            Icon::Search => 0xe151,
            Icon::Play => 0xe13c,
            Icon::Square => 0xe167,
            Icon::ChevronLeft => 0xe06e,
            Icon::ChevronRight => 0xe06f,
            Icon::SkipBack => 0xe15f,
            Icon::SkipForward => 0xe160,
            Icon::Keyboard => 0xe284,
            Icon::X => 0xe1b2,
            Icon::CircleX => 0xe084,
            Icon::CircleHelp => 0xe082,
            Icon::Monitor => 0xe11d,
            Icon::MonitorOff => 0xe1dc,
            Icon::EyeOff => 0xe0bb,
            Icon::History => 0xe1f5,
            Icon::BookOpen => 0xe05f,
            Icon::CornerDownLeft => 0xe0a1,
        };
        char::from_u32(c).expect("private use codepoint")
    }
}

/// A glyph on the left of a `text_input`.
pub fn input_icon(i: Icon, size: f32) -> iced::widget::text_input::Icon<Font> {
    iced::widget::text_input::Icon {
        font: FONT,
        code_point: i.codepoint(),
        size: Some(iced::Pixels(size)),
        spacing: 8.0,
        side: iced::widget::text_input::Side::Left,
    }
}

/// The icon as text at `size` points, centered in its own box so it sits
/// level with text and buttons. Chain `.color(..)` like any text.
pub fn icon<'a>(i: Icon, size: f32) -> Text<'a> {
    text(i.codepoint().to_string())
        .font(FONT)
        .size(size)
        .center()
}
