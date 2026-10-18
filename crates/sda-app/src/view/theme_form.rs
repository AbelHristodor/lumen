//! The theme form: Text, Background, Transition, Verses, Footer sections.
//! Every text property has an Inherit checkbox that hides its control.

use crate::app::App;
use crate::msg::Msg;
use crate::msg::themes::{BgKind, ColorField, DraftEdit, ThemesMsg};
use crate::state::themes::{COLOR_INHERIT_ID, bg_kind};
use crate::ui_theme;
use crate::view::themes::to_color;
use crate::widgets::card::card;
use crate::widgets::segmented::segmented;
use iced::widget::{
    Column, Row, checkbox, column, container, pick_list, row, slider, space, text, text_input,
};
use iced::{Alignment, Border, Element, Fill};
use sda_core::i18n::Strings;
use sda_core::theme::{
    Background, BundledFont, Direction, Easing, Font, HAlign, Prop, RefPos, Rgba, TextSize, Theme,
    TransitionKind, VAlign,
};
use std::fmt;

const LABEL_W: f32 = 110.0;
const SLIDER_W: f32 = 200.0;
const ROW_H: f32 = 32.0;

fn edit(e: DraftEdit) -> Msg {
    Msg::Themes(ThemesMsg::Draft(e))
}

fn label(t: &str) -> Element<'_, Msg> {
    text(t)
        .size(13)
        .color(ui_theme::MUTED)
        .width(LABEL_W)
        .into()
}

fn section<'a>(title: &'a str, rows: Vec<Element<'a, Msg>>) -> Element<'a, Msg> {
    let body = rows
        .into_iter()
        .fold(Column::new().spacing(ui_theme::SPACING), Column::push);
    card(column![text(title).size(15).font(ui_theme::BOLD), body].spacing(ui_theme::SPACING))
        .width(Fill)
        .into()
}

fn field<'a>(lbl: &'a str, control: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    row![label(lbl), control.into()]
        .spacing(ui_theme::SPACING)
        .align_y(Alignment::Center)
        .height(ROW_H)
        .into()
}

fn check<'a>(lbl: &'a str, on: bool, f: impl Fn(bool) -> DraftEdit + 'a) -> Element<'a, Msg> {
    checkbox(on)
        .label(lbl)
        .size(16)
        .text_size(13)
        .on_toggle(move |b| edit(f(b)))
        .into()
}

/// A labeled row with an Inherit checkbox after the control. The control
/// shows only when the property is set, so every control starts at the same
/// column.
fn inherit_row<'a>(
    lbl: &'a str,
    s: &'a Strings,
    inherit: bool,
    on_inherit: fn(bool) -> DraftEdit,
    control: Element<'a, Msg>,
    id: Option<&'static str>,
) -> Element<'a, Msg> {
    let mut check = container(
        checkbox(inherit)
            .label(s.theme_inherit.as_str())
            .size(16)
            .text_size(13)
            .on_toggle(move |b| edit(on_inherit(b))),
    );
    if let Some(id) = id {
        check = check.id(iced::widget::Id::new(id));
    }
    let mut r = Row::new()
        .spacing(ui_theme::SPACING)
        .align_y(Alignment::Center)
        .height(ROW_H)
        .push(label(lbl));
    if !inherit {
        r = r.push(control);
    }
    r.push(check).into()
}

fn slide<'a>(
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    value: f32,
    shown: String,
    f: fn(f32) -> DraftEdit,
) -> Element<'a, Msg> {
    row![
        slider(range, value, move |v| edit(f(v)))
            .step(step)
            .width(SLIDER_W),
        text(shown).size(13).color(ui_theme::MUTED),
    ]
    .spacing(ui_theme::SPACING / 2.0)
    .align_y(Alignment::Center)
    .into()
}

fn color_field<'a>(app: &'a App, which: ColorField, current: Rgba) -> Element<'a, Msg> {
    let value = match &app.themes.hex_edit {
        Some((f, t)) if *f == which => t.clone(),
        _ => current.to_hex(),
    };
    let color = to_color(current);
    let swatch = container(space::horizontal())
        .width(24)
        .height(24)
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: Border {
                color: ui_theme::MUTED,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..container::Style::default()
        });
    row![
        text_input("#RRGGBB", &value)
            .on_input(move |t| Msg::Themes(ThemesMsg::Hex(which, t)))
            .size(13)
            .padding(6)
            .width(100),
        swatch,
    ]
    .spacing(ui_theme::SPACING / 2.0)
    .align_y(Alignment::Center)
    .into()
}

#[derive(Debug, Clone, PartialEq)]
struct FontChoice(Font);

impl fmt::Display for FontChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.family())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct KindChoice(TransitionKind);

impl KindChoice {
    const ALL: [KindChoice; 8] = [
        KindChoice(TransitionKind::None),
        KindChoice(TransitionKind::Fade),
        KindChoice(TransitionKind::Slide(Direction::Left)),
        KindChoice(TransitionKind::Slide(Direction::Right)),
        KindChoice(TransitionKind::Slide(Direction::Up)),
        KindChoice(TransitionKind::Slide(Direction::Down)),
        KindChoice(TransitionKind::Scale),
        KindChoice(TransitionKind::Blur),
    ];
}

impl fmt::Display for KindChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            TransitionKind::None => "None",
            TransitionKind::Fade => "Fade",
            TransitionKind::Slide(Direction::Left) => "Slide left",
            TransitionKind::Slide(Direction::Right) => "Slide right",
            TransitionKind::Slide(Direction::Up) => "Slide up",
            TransitionKind::Slide(Direction::Down) => "Slide down",
            TransitionKind::Scale => "Scale",
            TransitionKind::Blur => "Blur",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EasingChoice(Easing);

impl EasingChoice {
    const ALL: [EasingChoice; 3] = [
        EasingChoice(Easing::Linear),
        EasingChoice(Easing::EaseInOut),
        EasingChoice(Easing::EaseOut),
    ];
}

impl fmt::Display for EasingChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Easing::Linear => "Linear",
            Easing::EaseInOut => "Ease in out",
            Easing::EaseOut => "Ease out",
        })
    }
}

fn text_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let t = &d.text;

    let fonts: Vec<FontChoice> = BundledFont::all()
        .into_iter()
        .map(|b| FontChoice(Font::Bundled(b)))
        .chain(
            app.themes
                .fonts
                .iter()
                .map(|n| FontChoice(Font::System(n.clone()))),
        )
        .collect();
    let font_sel = match &t.font {
        Prop::Set(f) => Some(FontChoice(f.clone())),
        Prop::Inherit => None,
    };
    let font = pick_list(fonts, font_sel, |f| edit(DraftEdit::Font(f.0)))
        .placeholder(s.theme_font_label.as_str())
        .text_size(13)
        .padding([4, 8])
        .into();

    let size: Element<'a, Msg> = match t.size {
        Prop::Set(TextSize::Pt(pt)) => row![
            check(&s.theme_auto_size, false, DraftEdit::SizeAuto),
            slide(
                12.0..=200.0,
                1.0,
                pt,
                format!("{pt:.0} pt"),
                DraftEdit::SizePt
            ),
        ]
        .spacing(ui_theme::SPACING)
        .align_y(Alignment::Center)
        .into(),
        _ => check(&s.theme_auto_size, true, DraftEdit::SizeAuto),
    };

    let bold = check(
        &s.theme_bold_label,
        matches!(t.bold, Prop::Set(true)),
        DraftEdit::Bold,
    );
    let color = color_field(
        app,
        ColorField::Text,
        match t.color {
            Prop::Set(c) => c,
            Prop::Inherit => Rgba::WHITE,
        },
    );
    let h_align = segmented(
        [
            ("Left", HAlign::Left),
            ("Center", HAlign::Center),
            ("Right", HAlign::Right),
        ],
        match t.h_align {
            Prop::Set(a) => a,
            Prop::Inherit => HAlign::Center,
        },
        |a| edit(DraftEdit::HAlign(a)),
    );
    let v_align = segmented(
        [
            ("Top", VAlign::Top),
            ("Middle", VAlign::Middle),
            ("Bottom", VAlign::Bottom),
        ],
        t.v_align,
        |a| edit(DraftEdit::VAlign(a)),
    );
    let spacing_v = match t.line_spacing {
        Prop::Set(v) => v,
        Prop::Inherit => 1.1,
    };
    let margin_v = match t.margin {
        Prop::Set(v) => v,
        Prop::Inherit => 0.06,
    };

    let mut shadow = row![check(
        &s.theme_shadow,
        t.shadow.is_some(),
        DraftEdit::ShadowOn
    )]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    if let Some(sh) = t.shadow {
        shadow = shadow
            .push(color_field(app, ColorField::Shadow, sh.color))
            .push(slide(
                0.0..=20.0,
                1.0,
                sh.offset.0,
                format!("{:.0} px", sh.offset.0),
                DraftEdit::ShadowOffset,
            ));
    }
    let mut outline = row![check(
        &s.theme_outline,
        t.outline.is_some(),
        DraftEdit::OutlineOn
    )]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    if let Some(o) = t.outline {
        outline = outline
            .push(color_field(app, ColorField::Outline, o.color))
            .push(slide(
                0.0..=10.0,
                0.5,
                o.width,
                format!("{:.1} px", o.width),
                DraftEdit::OutlineWidth,
            ));
    }

    section(
        &s.theme_text,
        vec![
            field(
                &s.theme_name_label,
                text_input(s.theme_name_label.as_str(), &d.name)
                    .on_input(|n| edit(DraftEdit::Name(n)))
                    .size(13)
                    .padding(6)
                    .width(260),
            ),
            inherit_row(
                &s.theme_font_label,
                s,
                t.font == Prop::Inherit,
                DraftEdit::FontInherit,
                font,
                None,
            ),
            inherit_row(
                &s.theme_size_label,
                s,
                t.size == Prop::Inherit,
                DraftEdit::SizeInherit,
                size,
                None,
            ),
            inherit_row(
                &s.theme_bold_label,
                s,
                t.bold == Prop::Inherit,
                DraftEdit::BoldInherit,
                bold,
                None,
            ),
            inherit_row(
                &s.theme_text_color_label,
                s,
                t.color == Prop::Inherit,
                DraftEdit::ColorInherit,
                color,
                Some(COLOR_INHERIT_ID),
            ),
            inherit_row(
                &s.theme_align_label,
                s,
                t.h_align == Prop::Inherit,
                DraftEdit::HAlignInherit,
                h_align,
                None,
            ),
            field(&s.theme_valign_label, v_align),
            inherit_row(
                &s.theme_line_spacing_label,
                s,
                t.line_spacing == Prop::Inherit,
                DraftEdit::SpacingInherit,
                slide(
                    0.8..=2.5,
                    0.05,
                    spacing_v,
                    format!("{spacing_v:.2}"),
                    DraftEdit::Spacing,
                ),
                None,
            ),
            inherit_row(
                &s.theme_margin_label,
                s,
                t.margin == Prop::Inherit,
                DraftEdit::MarginInherit,
                slide(
                    0.0..=0.3,
                    0.01,
                    margin_v,
                    format!("{:.0} %", margin_v * 100.0),
                    DraftEdit::Margin,
                ),
                None,
            ),
            field(
                &s.theme_opacity,
                slide(
                    0.0..=1.0,
                    0.05,
                    t.opacity,
                    format!("{:.0} %", t.opacity * 100.0),
                    DraftEdit::Opacity,
                ),
            ),
            field(&s.theme_shadow, shadow),
            field(&s.theme_outline, outline),
        ],
    )
}

fn background_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let kind = segmented(
        [
            (s.theme_bg_solid.as_str(), BgKind::Solid),
            (s.theme_bg_gradient.as_str(), BgKind::Gradient),
            (s.theme_bg_particles.as_str(), BgKind::Particles),
            (s.theme_bg_image.as_str(), BgKind::Image),
            (s.theme_bg_video.as_str(), BgKind::Video),
        ],
        bg_kind(&d.background),
        |k| edit(DraftEdit::BgKind(k)),
    );
    let mut rows: Vec<Element<'a, Msg>> = vec![kind];
    let choose = |msg: Msg| {
        iced::widget::button(text(s.choose.as_str()).size(13))
            .padding([6, 12])
            .style(ui_theme::ghost_button)
            .on_press(msg)
    };
    match &d.background {
        Background::Solid(c) => rows.push(field(
            &s.theme_bg_color_label,
            color_field(app, ColorField::BgSolid, *c),
        )),
        Background::Gradient {
            stops,
            angle,
            drift,
        } => {
            let mut stop_row = Row::new()
                .spacing(ui_theme::SPACING)
                .align_y(Alignment::Center)
                .push(label(&s.theme_bg_color_label));
            for (i, (_, c)) in stops.iter().enumerate() {
                stop_row = stop_row.push(color_field(app, ColorField::BgStop(i), *c));
            }
            rows.push(stop_row.into());
            rows.push(field(
                "Angle",
                slide(
                    0.0..=360.0,
                    5.0,
                    *angle,
                    format!("{angle:.0} °"),
                    DraftEdit::BgAngle,
                ),
            ));
            rows.push(field(
                "Drift",
                slide(
                    0.0..=1.0,
                    0.05,
                    *drift,
                    format!("{:.0} %", drift * 100.0),
                    DraftEdit::BgDrift,
                ),
            ));
        }
        Background::Particles {
            base,
            color,
            count,
            speed,
        } => {
            rows.push(
                row![
                    label(&s.theme_bg_color_label),
                    color_field(app, ColorField::BgSolid, *base),
                    color_field(app, ColorField::BgParticleColor, *color),
                ]
                .spacing(ui_theme::SPACING)
                .align_y(Alignment::Center)
                .into(),
            );
            rows.push(field(
                &s.theme_bg_particles,
                row![
                    slider(10..=500u32, *count, |n| edit(DraftEdit::BgParticleCount(n)))
                        .step(10u32)
                        .width(SLIDER_W),
                    text(count.to_string()).size(13).color(ui_theme::MUTED),
                ]
                .spacing(ui_theme::SPACING / 2.0)
                .align_y(Alignment::Center),
            ));
            rows.push(field(
                "Speed",
                slide(
                    0.1..=3.0,
                    0.1,
                    *speed,
                    format!("{speed:.1} x"),
                    DraftEdit::BgParticleSpeed,
                ),
            ));
        }
        Background::Image(path) => rows.push(field(
            &s.theme_bg_image,
            row![
                text(path.to_string_lossy().into_owned())
                    .size(13)
                    .color(ui_theme::MUTED),
                choose(Msg::Themes(ThemesMsg::PickImage)),
            ]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
        )),
        Background::Video { path } => rows.push(field(
            &s.theme_bg_video,
            row![
                text(path.to_string_lossy().into_owned())
                    .size(13)
                    .color(ui_theme::MUTED),
                choose(Msg::Themes(ThemesMsg::PickVideo)),
            ]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
        )),
    }
    rows.push(field(
        &s.theme_opacity,
        slide(
            0.0..=1.0,
            0.05,
            d.background_opacity,
            format!("{:.0} %", d.background_opacity * 100.0),
            DraftEdit::BgOpacity,
        ),
    ));
    section(&s.theme_background, rows)
}

fn transition_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let tr = d.transition;
    let kind = pick_list(&KindChoice::ALL[..], Some(KindChoice(tr.kind)), |k| {
        edit(DraftEdit::TransitionKind(k.0))
    })
    .text_size(13)
    .padding([4, 8]);
    let easing = pick_list(&EasingChoice::ALL[..], Some(EasingChoice(tr.easing)), |e| {
        edit(DraftEdit::Easing(e.0))
    })
    .text_size(13)
    .padding([4, 8]);
    let duration = row![
        slider(0..=2000u32, tr.duration_ms, |ms| edit(DraftEdit::Duration(
            ms
        )))
        .step(50u32)
        .width(SLIDER_W),
        text(format!("{} ms", tr.duration_ms))
            .size(13)
            .color(ui_theme::MUTED),
    ]
    .spacing(ui_theme::SPACING / 2.0)
    .align_y(Alignment::Center);
    section(
        &s.theme_transition,
        vec![
            field(&s.theme_transition, kind),
            field(&s.theme_duration, duration),
            field(&s.theme_easing, easing),
        ],
    )
}

fn verses_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let v = d.verses;
    let per_slide = row![
        slider(1..=6u8, v.per_slide, |n| edit(DraftEdit::PerSlide(n))).width(SLIDER_W),
        text(v.per_slide.to_string())
            .size(13)
            .color(ui_theme::MUTED),
    ]
    .spacing(ui_theme::SPACING / 2.0)
    .align_y(Alignment::Center);
    let ref_pos = segmented(
        [("Top", RefPos::Top), ("Bottom", RefPos::Bottom)],
        v.reference_at,
        |p| edit(DraftEdit::RefPos(p)),
    );
    section(
        &s.theme_verses,
        vec![
            field(&s.theme_per_slide, per_slide),
            check(
                &s.theme_show_reference,
                v.show_reference,
                DraftEdit::ShowReference,
            ),
            check(
                &s.theme_show_numbers,
                v.show_numbers,
                DraftEdit::ShowNumbers,
            ),
            field(&s.theme_valign_label, ref_pos),
        ],
    )
}

fn footer_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    section(
        &s.theme_footer,
        vec![check(&s.theme_footer_label, d.footer, DraftEdit::Footer)],
    )
}

pub fn show(app: &App) -> Element<'_, Msg> {
    let d = &app.themes.draft;
    column![
        text_section(app, d),
        background_section(app, d),
        transition_section(app, d),
        verses_section(app, d),
        footer_section(app, d),
    ]
    .spacing(ui_theme::SPACING)
    .into()
}
