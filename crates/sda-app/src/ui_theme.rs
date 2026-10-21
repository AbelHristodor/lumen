//! Palettes and widget styles. Dark or light surfaces, one accent, 8 px
//! corners, a 12 px grid. Every button and card goes through here and reads
//! the active [`AppTheme`] through [`palette`].

use iced::theme::{Palette as IcedPalette, palette as iced_palette};
use iced::widget::{button, container, pick_list, rule, text_input};
use iced::{Border, Color, Shadow, Theme, Vector};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const SPACING: f32 = 12.0;
/// Inter at bold weight for headings.
pub const BOLD: iced::Font = iced::Font {
    weight: iced::font::Weight::Bold,
    ..iced::Font::with_name("Inter")
};
/// Lora at bold weight, the wordmark only.
pub const SERIF_BOLD: iced::Font = iced::Font {
    weight: iced::font::Weight::Bold,
    ..iced::Font::with_name("Lora")
};
pub const RADIUS: f32 = 8.0;

/// The colors one app theme is made of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Window background.
    pub bg: Color,
    /// Card surface.
    pub surface: Color,
    /// Raised surface for hover and chips.
    pub surface_hi: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    /// Text on an accent fill.
    pub on_accent: Color,
    /// Selected row tint.
    pub selection: Color,
    /// The wordmark color.
    pub brand: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    /// False for a light theme. Hover then darkens instead of lightening.
    pub dark: bool,
}

const fn rgb(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

const fn rgba(hex: u32, a: f32) -> Color {
    Color::from_rgba8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, a)
}

/// The neutral dark palette Lumen shipped with up to 2.1.
const GRAPHITE: Palette = Palette {
    bg: rgb(0x14161A),
    surface: rgb(0x1C1F26),
    surface_hi: rgb(0x262A33),
    text: rgb(0xE6E8EC),
    muted: rgb(0x8A909C),
    accent: rgb(0x4F8CFF),
    on_accent: Color::WHITE,
    selection: rgba(0x4F8CFF, 0.22),
    brand: rgb(0xE6E8EC),
    success: rgb(0x3DD68C),
    warning: rgb(0xF5B83D),
    danger: rgb(0xF06A6A),
    dark: true,
};

/// Navy, gold, and lavender, the colors of the app mark.
const LUMEN: Palette = Palette {
    bg: rgb(0x0E1220),
    surface: rgb(0x161C2E),
    surface_hi: rgb(0x252F4A),
    text: rgb(0xECE8DF),
    muted: rgb(0x98A1BF),
    accent: rgb(0xF0C96E),
    on_accent: rgb(0x1A160C),
    selection: rgba(0x8090C8, 0.30),
    brand: rgb(0xF4E7C6),
    success: rgb(0x3DD68C),
    warning: rgb(0xF59E4B),
    danger: rgb(0xF06A6A),
    dark: true,
};

/// Warm charcoal with an amber accent.
const EMBER: Palette = Palette {
    bg: rgb(0x16120F),
    surface: rgb(0x201A16),
    surface_hi: rgb(0x2E2620),
    text: rgb(0xEFE6DC),
    muted: rgb(0xA3948A),
    accent: rgb(0xE8884A),
    on_accent: rgb(0x1B100A),
    selection: rgba(0xE8884A, 0.22),
    brand: rgb(0xF6D9B8),
    success: rgb(0x5BC48C),
    warning: rgb(0xF2C14E),
    danger: rgb(0xEF6B6B),
    dark: true,
};

/// Light. Cream paper, ink text, a navy accent.
const DAWN: Palette = Palette {
    bg: rgb(0xF4F1EA),
    surface: rgb(0xFFFFFF),
    surface_hi: rgb(0xE6E1D6),
    text: rgb(0x1F2430),
    muted: rgb(0x6B7280),
    accent: rgb(0x2E4A8C),
    on_accent: Color::WHITE,
    selection: rgba(0x2E4A8C, 0.16),
    brand: rgb(0x1E2A4A),
    success: rgb(0x1F9D5A),
    warning: rgb(0xC77A12),
    danger: rgb(0xC43E3E),
    dark: false,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTheme {
    Graphite,
    #[default]
    Lumen,
    Ember,
    Dawn,
}

impl AppTheme {
    pub const ALL: [AppTheme; 4] = [
        AppTheme::Lumen,
        AppTheme::Graphite,
        AppTheme::Ember,
        AppTheme::Dawn,
    ];

    /// Shown in Settings. Proper nouns, the same in every language.
    pub fn label(self) -> &'static str {
        match self {
            AppTheme::Graphite => "Graphite",
            AppTheme::Lumen => "Lumen",
            AppTheme::Ember => "Ember",
            AppTheme::Dawn => "Dawn",
        }
    }

    /// The value stored in the config file.
    pub fn code(self) -> &'static str {
        match self {
            AppTheme::Graphite => "graphite",
            AppTheme::Lumen => "lumen",
            AppTheme::Ember => "ember",
            AppTheme::Dawn => "dawn",
        }
    }

    /// Unknown codes fall back to the default.
    pub fn from_code(code: Option<&str>) -> AppTheme {
        AppTheme::ALL
            .into_iter()
            .find(|t| Some(t.code()) == code)
            .unwrap_or_default()
    }

    pub fn palette(self) -> &'static Palette {
        match self {
            AppTheme::Graphite => &GRAPHITE,
            AppTheme::Lumen => &LUMEN,
            AppTheme::Ember => &EMBER,
            AppTheme::Dawn => &DAWN,
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

// ponytail: one process wide theme. Style fns get an iced Theme, not the App,
// so a global is the shortest path. Thread it through App if a second window
// ever needs its own palette.
static CURRENT: AtomicUsize = AtomicUsize::new(AppTheme::Lumen as usize);

/// Make `theme` the active app theme. The next frame draws with it.
pub fn set_app_theme(theme: AppTheme) {
    CURRENT.store(theme.index(), Ordering::Relaxed);
}

pub fn app_theme() -> AppTheme {
    let i = CURRENT.load(Ordering::Relaxed);
    AppTheme::ALL
        .into_iter()
        .find(|t| t.index() == i)
        .unwrap_or_default()
}

/// The active palette.
pub fn palette() -> &'static Palette {
    app_theme().palette()
}

/// The iced theme for the active palette. Built once per theme and cloned,
/// the clone is an `Arc` bump.
pub fn theme() -> Theme {
    static THEMES: [OnceLock<Theme>; 4] = [const { OnceLock::new() }; 4];
    let t = app_theme();
    THEMES[t.index()]
        .get_or_init(|| {
            let p = t.palette();
            Theme::custom_with_fn(
                t.label().to_string(),
                IcedPalette {
                    background: p.bg,
                    text: p.text,
                    primary: p.accent,
                    success: p.success,
                    warning: p.warning,
                    danger: p.danger,
                },
                iced_palette::Extended::generate,
            )
        })
        .clone()
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// A raised panel with a soft shadow.
pub fn card(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        background: Some(p.surface.into()),
        border: Border::default().rounded(RADIUS),
        shadow: Shadow {
            color: with_alpha(Color::BLACK, if p.dark { 0.4 } else { 0.12 }),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

/// A key cap pill inside a button or the shortcut sheet. Translucent so it
/// reads on the accent and the ghost button alike.
pub fn key_cap(_theme: &Theme) -> container::Style {
    let p = palette();
    let tint = if p.dark { Color::WHITE } else { Color::BLACK };
    container::Style {
        background: Some(with_alpha(tint, 0.14).into()),
        border: Border::default().rounded(4.0),
        ..container::Style::default()
    }
}

pub fn chip(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        background: Some(p.surface_hi.into()),
        text_color: Some(p.text),
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

/// Nudge a color toward the hover shade: lighter on a dark theme, darker on
/// a light one.
fn raise(c: Color, k: f32) -> Color {
    let k = if palette().dark { k } else { -k };
    Color::from_rgb(
        (c.r + k).clamp(0.0, 1.0),
        (c.g + k).clamp(0.0, 1.0),
        (c.b + k).clamp(0.0, 1.0),
    )
}

/// Filled accent button for the main action.
pub fn accent_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    match status {
        button::Status::Active => button_base(p.accent, p.on_accent),
        button::Status::Hovered => button_base(raise(p.accent, 0.08), p.on_accent),
        button::Status::Pressed => button_base(raise(p.accent, -0.06), p.on_accent),
        button::Status::Disabled => button_base(p.surface_hi, with_alpha(p.muted, 0.7)),
    }
}

/// Quiet button that only shows a surface on hover. Disabled drops to a
/// faint outline with dim text so it cannot pass for an active one.
pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let p = palette();
    match status {
        button::Status::Active => button_base(p.surface_hi, p.text),
        button::Status::Hovered => button_base(raise(p.surface_hi, 0.06), p.text),
        button::Status::Pressed => button_base(raise(p.surface_hi, -0.03), p.text),
        button::Status::Disabled => button::Style {
            border: Border {
                color: with_alpha(p.surface_hi, 0.8),
                width: 1.0,
                radius: RADIUS.into(),
            },
            ..button_base(Color::TRANSPARENT, with_alpha(p.muted, 0.55))
        },
    }
}

/// A verse row. Selected rows get the accent tint, others are flat.
pub fn verse_row(selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: selected.then(|| palette().selection.into()),
        border: Border::default().rounded(RADIUS),
        ..container::Style::default()
    }
}

/// A row in a list or a card in a picker. The selected one carries a
/// tinted accent surface, hover a raised surface.
pub fn list_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let p = palette();
        let bg = if selected {
            p.selection
        } else if status == button::Status::Hovered {
            p.surface_hi
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(bg.into()),
            text_color: p.text,
            border: Border::default().rounded(RADIUS - 2.0),
            ..button::Style::default()
        }
    }
}

/// A card in a picker. The selected one gets an accent outline and no fill,
/// so a second marker such as an Active chip stays readable.
pub fn outlined_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let p = palette();
        let bg = if status == button::Status::Hovered {
            p.surface_hi
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(bg.into()),
            text_color: p.text,
            border: Border {
                color: if selected {
                    p.accent
                } else {
                    Color::TRANSPARENT
                },
                width: 1.5,
                radius: (RADIUS - 2.0).into(),
            },
            ..button::Style::default()
        }
    }
}

/// A small 16 by 9 tile. The active one gets an accent border.
pub fn tile(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| {
        let p = palette();
        button::Style {
            background: Some(Color::BLACK.into()),
            text_color: p.text,
            border: Border {
                color: if active { p.accent } else { p.surface_hi },
                width: 2.0,
                radius: 4.0.into(),
            },
            ..button::Style::default()
        }
    }
}

/// Small label drawn over a tile corner.
pub fn tile_label(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        background: Some(with_alpha(Color::BLACK, 0.6).into()),
        text_color: Some(p.text),
        border: Border::default().rounded(3.0),
        ..container::Style::default()
    }
}

/// Section label above a group of controls. Small, muted, upper case.
pub fn section_label<'a>(label: &str) -> iced::widget::Text<'a> {
    iced::widget::text(label.to_uppercase())
        .size(11)
        .font(BOLD)
        .color(palette().muted)
}

/// Text fields share the card corner radius.
pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let p = palette();
    let base = text_input::default(theme, status);
    text_input::Style {
        background: p.bg.into(),
        border: Border {
            radius: (RADIUS - 2.0).into(),
            ..base.border
        },
        ..base
    }
}

/// Pick lists match `input`.
pub fn picker(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let p = palette();
    let base = pick_list::default(theme, status);
    pick_list::Style {
        background: p.bg.into(),
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
        let p = palette();
        if active {
            return button_base(p.accent, p.on_accent);
        }
        match status {
            button::Status::Hovered => button_base(p.surface_hi, p.text),
            _ => button_base(Color::TRANSPARENT, p.muted),
        }
    }
}

/// The track behind a segmented control that sits on the window.
pub fn segment_track(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        background: Some(p.surface.into()),
        border: Border::default().rounded(RADIUS + 2.0),
        ..container::Style::default()
    }
}

/// The track behind a segmented control that sits on a card.
pub fn segment_track_on_card(_theme: &Theme) -> container::Style {
    let p = palette();
    container::Style {
        background: Some(p.bg.into()),
        border: Border::default().rounded(RADIUS + 2.0),
        ..container::Style::default()
    }
}

/// Faint 1 px line between the header and the body.
pub fn hairline(_theme: &Theme) -> rule::Style {
    let p = palette();
    rule::Style {
        color: p.surface_hi,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}
