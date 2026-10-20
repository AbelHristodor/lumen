//! Palette and widget styles. Dark neutral surfaces, one blue accent, 8 px
//! corners, a 12 px grid. Every button and card goes through here.

use iced::theme::{Palette, palette};
use iced::widget::{button, container, pick_list, rule, text_input};
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
/// A key cap pill inside a button or the shortcut sheet. Translucent so it
/// reads on the accent and the ghost button alike.
pub fn key_cap(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(with_alpha(Color::WHITE, 0.14).into()),
        border: Border::default().rounded(4.0),
        ..container::Style::default()
    }
}

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
        button::Status::Disabled => button_base(SURFACE_HI, with_alpha(MUTED, 0.7)),
    }
}

/// Quiet button that only shows a surface on hover. Disabled drops to a
/// faint outline with dim text so it cannot pass for an active one.
pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active => button_base(SURFACE_HI, TEXT),
        button::Status::Hovered => button_base(lighten(SURFACE_HI, 0.06), TEXT),
        button::Status::Pressed => button_base(lighten(SURFACE_HI, -0.03), TEXT),
        button::Status::Disabled => button::Style {
            border: Border {
                color: with_alpha(SURFACE_HI, 0.8),
                width: 1.0,
                radius: RADIUS.into(),
            },
            ..button_base(Color::TRANSPARENT, with_alpha(MUTED, 0.55))
        },
    }
}

/// A verse row. Selected rows get the accent tint, others are flat.
pub fn verse_row(selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: selected.then(|| with_alpha(ACCENT, 0.22).into()),
        border: Border::default().rounded(RADIUS),
        ..container::Style::default()
    }
}

/// A row in a list or a card in a picker. The selected one carries a
/// tinted accent surface, hover a raised surface.
pub fn list_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if selected {
            with_alpha(ACCENT, 0.22)
        } else if status == button::Status::Hovered {
            SURFACE_HI
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(bg.into()),
            text_color: TEXT,
            border: Border::default().rounded(RADIUS - 2.0),
            ..button::Style::default()
        }
    }
}

/// A card in a picker. The selected one gets an accent outline and no fill,
/// so a second marker such as an Active chip stays readable.
pub fn outlined_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if status == button::Status::Hovered {
            SURFACE_HI
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(bg.into()),
            text_color: TEXT,
            border: Border {
                color: if selected { ACCENT } else { Color::TRANSPARENT },
                width: 1.5,
                radius: (RADIUS - 2.0).into(),
            },
            ..button::Style::default()
        }
    }
}

/// A small 16 by 9 tile. The active one gets an accent border.
pub fn tile(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: Some(Color::BLACK.into()),
        text_color: TEXT,
        border: Border {
            color: if active { ACCENT } else { SURFACE_HI },
            width: 2.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

/// Small label drawn over a tile corner.
pub fn tile_label(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(with_alpha(Color::BLACK, 0.6).into()),
        text_color: Some(TEXT),
        border: Border::default().rounded(3.0),
        ..container::Style::default()
    }
}

/// Section label above a group of controls. Small, muted, upper case.
pub fn section_label<'a>(label: &str) -> iced::widget::Text<'a> {
    iced::widget::text(label.to_uppercase())
        .size(11)
        .font(BOLD)
        .color(MUTED)
}

/// Text fields share the card corner radius.
pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = text_input::default(theme, status);
    text_input::Style {
        background: BG.into(),
        border: Border {
            radius: (RADIUS - 2.0).into(),
            ..base.border
        },
        ..base
    }
}

/// Pick lists match `input`.
pub fn picker(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::default(theme, status);
    pick_list::Style {
        background: BG.into(),
        border: Border {
            radius: (RADIUS - 2.0).into(),
            ..base.border
        },
        ..base
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

/// The track behind a segmented control that sits on the window.
pub fn segment_track(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border::default().rounded(RADIUS + 2.0),
        ..container::Style::default()
    }
}

/// The track behind a segmented control that sits on a card.
pub fn segment_track_on_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG.into()),
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
