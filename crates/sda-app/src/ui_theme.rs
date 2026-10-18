//! Palette and widget styles. Dark neutral surfaces, one blue accent, 8 px
//! corners, a 12 px grid. Every button and card goes through here.

use iced::theme::{Palette, palette};
use iced::widget::{button, container, rule};
use iced::{Border, Color, Shadow, Theme, Vector};
use std::sync::OnceLock;

pub const SPACING: f32 = 12.0;
/// Inter at bold weight for headings.
pub const BOLD: iced::Font = iced::Font {
    weight: iced::font::Weight::Bold,
    ..iced::Font::with_name("Inter")
};
pub const RADIUS: f32 = 8.0;

/// Window background.
pub const BG: Color = Color::from_rgb8(0x14, 0x16, 0x1A);
/// Card surface.
pub const SURFACE: Color = Color::from_rgb8(0x1C, 0x1F, 0x26);
/// Raised surface for hover and chips.
pub const SURFACE_HI: Color = Color::from_rgb8(0x26, 0x2A, 0x33);
pub const TEXT: Color = Color::from_rgb8(0xE6, 0xE8, 0xEC);
pub const MUTED: Color = Color::from_rgb8(0x8A, 0x90, 0x9C);
pub const ACCENT: Color = Color::from_rgb8(0x4F, 0x8C, 0xFF);

/// The one app theme. Built once and cloned, the clone is an `Arc` bump.
pub fn theme() -> Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME
        .get_or_init(|| {
            Theme::custom_with_fn(
                "SDA",
                Palette {
                    background: BG,
                    text: TEXT,
                    primary: ACCENT,
                    success: Color::from_rgb8(0x3D, 0xD6, 0x8C),
                    warning: Color::from_rgb8(0xF5, 0xB8, 0x3D),
                    danger: Color::from_rgb8(0xF0, 0x6A, 0x6A),
                },
                palette::Extended::generate,
            )
        })
        .clone()
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// A raised panel with a soft shadow.
pub fn card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border::default().rounded(RADIUS),
        shadow: Shadow {
            color: with_alpha(Color::BLACK, 0.4),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

/// A small rounded label for status text.
pub fn chip(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE_HI.into()),
        text_color: Some(TEXT),
        border: Border::default().rounded(999.0),
        ..container::Style::default()
    }
}

/// The 160 by 90 live thumbnail frame.
pub fn thumb_frame(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::BLACK.into()),
        border: Border::default().rounded(4.0),
        ..container::Style::default()
    }
}

fn button_base(bg: Color, text: Color) -> button::Style {
    button::Style {
        background: Some(bg.into()),
        text_color: text,
        border: Border::default().rounded(RADIUS),
        ..button::Style::default()
    }
}

fn lighten(c: Color, k: f32) -> Color {
    Color::from_rgb((c.r + k).min(1.0), (c.g + k).min(1.0), (c.b + k).min(1.0))
}

/// Filled accent button for the main action.
pub fn accent_button(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_base(ACCENT, Color::WHITE),
        button::Status::Hovered => button_base(lighten(ACCENT, 0.08), Color::WHITE),
        button::Status::Pressed => button_base(lighten(ACCENT, -0.06), Color::WHITE),
        button::Status::Disabled => {
            button_base(with_alpha(ACCENT, 0.35), with_alpha(Color::WHITE, 0.5))
        }
    }
}

/// Quiet button that only shows a surface on hover.
pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_base(SURFACE_HI, TEXT),
        button::Status::Hovered => button_base(lighten(SURFACE_HI, 0.06), TEXT),
        button::Status::Pressed => button_base(lighten(SURFACE_HI, -0.03), TEXT),
        button::Status::Disabled => button_base(with_alpha(SURFACE_HI, 0.5), MUTED),
    }
}

/// One segment of a segmented control. The active segment is filled.
pub fn segment_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        if active {
            return button_base(ACCENT, Color::WHITE);
        }
        match status {
            button::Status::Hovered => button_base(SURFACE_HI, TEXT),
            _ => button_base(Color::TRANSPARENT, MUTED),
        }
    }
}

/// The track behind a segmented control.
pub fn segment_track(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border::default().rounded(RADIUS + 2.0),
        ..container::Style::default()
    }
}

/// Faint 1 px line between the header and the body.
pub fn hairline(_theme: &Theme) -> rule::Style {
    rule::Style {
        color: SURFACE_HI,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}
