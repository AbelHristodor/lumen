//! The theme form: Text, Reference, Background, Transition, Verses, Footer
//! sections.
//! Every text property has an Inherit checkbox that hides its control.

use crate::app::App;
use crate::msg::Msg;
use crate::msg::themes::{BgKind, ColorField, DraftEdit, ThemesMsg};
use crate::state::present::selected_hymn;
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
    Background, BundledFont, Direction, Easing, Font, HAlign, Prop, Rgba, TextSize, TextStyle,
    Theme, TransitionKind, VAlign,
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
        .color(ui_theme::palette().muted)
        .width(LABEL_W)
        .into()
}

fn section<'a>(title: &'a str, rows: Vec<Element<'a, Msg>>) -> Element<'a, Msg> {
    let body = rows
        .into_iter()
        .fold(Column::new().spacing(ui_theme::SPACING), Column::push);
    card(column![ui_theme::section_label(title), body].spacing(ui_theme::SPACING))
        .width(Fill)
        .into()
}

/// What the selected hymn's first run carries for each inheritable
/// property, as display text. Shown next to a checked Inherit box.
#[derive(Default)]
struct Inherited {
    font: Option<String>,
    size: Option<String>,
    bold: Option<String>,
    color: Option<String>,
    h_align: Option<String>,
    spacing: Option<String>,
}

fn align_label(s: &Strings, a: HAlign) -> String {
    match a {
        HAlign::Left => s.align_left.clone(),
        HAlign::Center => s.align_center.clone(),
        HAlign::Right => s.align_right.clone(),
    }
}

fn inherited(app: &App) -> Inherited {
    let s = &app.core.strings;
    let Some(paragraph) = selected_hymn(app)
        .and_then(|(_, h)| h.slides.first())
        .and_then(|sl| sl.boxes.first())
        .and_then(|b| b.paragraphs.first())
    else {
        return Inherited::default();
    };
    let run = paragraph.runs.first();
    let align = |a| align_label(s, a);
    Inherited {
        font: run.and_then(|r| r.font.clone()),
        size: run.and_then(|r| r.size_pt).map(|pt| format!("{pt:.0}pt")),
        bold: run
            .and_then(|r| r.bold)
            .map(|b| if b { "✓" } else { "✗" }.to_string()),
        color: run.and_then(|r| r.color).map(|c| c.to_hex()),
        h_align: paragraph.align.map(align),
        spacing: paragraph.line_spacing.map(|v| format!("{v:.2}")),
    }
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

/// A bare checkbox for rows whose label already names the property.
fn toggle_box<'a>(on: bool, f: impl Fn(bool) -> DraftEdit + 'a) -> Element<'a, Msg> {
    checkbox(on).size(16).on_toggle(move |b| edit(f(b))).into()
}

/// A labeled row with an Inherit checkbox after the control. The control
/// shows only when the property is set, so every control starts at the same
/// column. A checked box shows the value the file carries, when known.
fn inherit_row<'a>(
    lbl: &'a str,
    s: &'a Strings,
    inherit: bool,
    on_inherit: impl Fn(bool) -> DraftEdit + 'a,
    control: Element<'a, Msg>,
    id: Option<&'static str>,
    file_value: Option<String>,
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
    r = r.push(check);
    if inherit && let Some(v) = file_value {
        r = r.push(
            text(format!("·  {v}"))
                .size(13)
                .color(ui_theme::palette().muted)
                .wrapping(text::Wrapping::None),
        );
    }
    r.into()
}

fn slide<'a>(
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    value: f32,
    shown: String,
    f: impl Fn(f32) -> DraftEdit + 'a,
) -> Element<'a, Msg> {
    row![
        slider(range, value, move |v| edit(f(v)))
            .step(step)
            .width(SLIDER_W),
        text(shown).size(13).color(ui_theme::palette().muted),
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
                color: ui_theme::palette().muted,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..container::Style::default()
        });
    row![
        text_input("#RRGGBB", &value)
            .on_input(move |t| Msg::Themes(ThemesMsg::Hex(which, t)))
            .style(ui_theme::input)
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

/// A pick list entry carrying its translated label. Equality is on the
/// value so the picker finds the selected one whatever the language.
#[derive(Debug, Clone)]
struct Choice<T> {
    value: T,
    label: String,
}

impl<T: PartialEq> PartialEq for Choice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn choice<T>(value: T, label: impl Into<String>) -> Choice<T> {
    Choice {
        value,
        label: label.into(),
    }
}

fn kind_label(s: &Strings, kind: TransitionKind) -> String {
    let slide = |dir: &str| format!("{} {}", s.transition_slide, dir.to_lowercase());
    match kind {
        TransitionKind::None => s.transition_none.clone(),
        TransitionKind::Fade => s.transition_fade.clone(),
        TransitionKind::Slide(Direction::Left) => slide(&s.align_left),
        TransitionKind::Slide(Direction::Right) => slide(&s.align_right),
        TransitionKind::Slide(Direction::Up) => slide(&s.align_top),
        TransitionKind::Slide(Direction::Down) => slide(&s.align_bottom),
        TransitionKind::Scale => s.transition_scale.clone(),
        TransitionKind::Blur => s.transition_blur.clone(),
    }
}

const KINDS: [TransitionKind; 8] = [
    TransitionKind::None,
    TransitionKind::Fade,
    TransitionKind::Slide(Direction::Left),
    TransitionKind::Slide(Direction::Right),
    TransitionKind::Slide(Direction::Up),
    TransitionKind::Slide(Direction::Down),
    TransitionKind::Scale,
    TransitionKind::Blur,
];

fn easing_label(s: &Strings, e: Easing) -> &str {
    match e {
        Easing::Linear => &s.easing_linear,
        Easing::EaseInOut => &s.easing_in_out,
        Easing::EaseOut => &s.easing_out,
    }
}

const EASINGS: [Easing; 3] = [Easing::Linear, Easing::EaseInOut, Easing::EaseOut];

/// The controls for one `TextStyle`. `wrap` turns a plain text edit into
/// the edit for this style, `colors` names the text, shadow and outline
/// color fields, and `color_id` tags the color Inherit checkbox.
#[allow(clippy::too_many_arguments)]
fn text_style_section<'a>(
    app: &'a App,
    title: &'a str,
    t: &'a TextStyle,
    file: Inherited,
    colors: (ColorField, ColorField, ColorField),
    wrap: fn(DraftEdit) -> DraftEdit,
    name_row: Option<Element<'a, Msg>>,
    color_id: Option<&'static str>,
) -> Element<'a, Msg> {
    let s = &app.core.strings;

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
    let font = pick_list(fonts, font_sel, move |f| edit(wrap(DraftEdit::Font(f.0))))
        .placeholder(s.theme_font_label.as_str())
        .style(ui_theme::picker)
        .text_size(13)
        .padding([4, 8])
        .into();

    let size: Element<'a, Msg> = match t.size {
        Prop::Set(TextSize::Pt(pt)) => row![
            check(&s.theme_auto_size, false, move |v| wrap(
                DraftEdit::SizeAuto(v)
            )),
            slide(12.0..=200.0, 1.0, pt, format!("{pt:.0} pt"), move |v| wrap(
                DraftEdit::SizePt(v)
            )),
        ]
        .spacing(ui_theme::SPACING)
        .align_y(Alignment::Center)
        .into(),
        _ => check(&s.theme_auto_size, true, move |v| {
            wrap(DraftEdit::SizeAuto(v))
        }),
    };

    let bold = toggle_box(matches!(t.bold, Prop::Set(true)), move |v| {
        wrap(DraftEdit::Bold(v))
    });
    let color = color_field(
        app,
        colors.0,
        match t.color {
            Prop::Set(c) => c,
            Prop::Inherit => Rgba::WHITE,
        },
    );
    let h_align = segmented(
        [
            (s.align_left.as_str(), HAlign::Left),
            (s.align_center.as_str(), HAlign::Center),
            (s.align_right.as_str(), HAlign::Right),
        ],
        match t.h_align {
            Prop::Set(a) => a,
            Prop::Inherit => HAlign::Center,
        },
        move |a| edit(wrap(DraftEdit::HAlign(a))),
    );
    let v_align = segmented(
        [
            (s.align_top.as_str(), VAlign::Top),
            (s.align_middle.as_str(), VAlign::Middle),
            (s.align_bottom.as_str(), VAlign::Bottom),
        ],
        t.v_align,
        move |a| edit(wrap(DraftEdit::VAlign(a))),
    );
    let spacing_v = match t.line_spacing {
        Prop::Set(v) => v,
        Prop::Inherit => 1.1,
    };
    let margin_v = match t.margin {
        Prop::Set(v) => v,
        Prop::Inherit => 0.06,
    };

    let mut shadow = row![toggle_box(t.shadow.is_some(), move |v| wrap(
        DraftEdit::ShadowOn(v)
    ))]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    if let Some(sh) = t.shadow {
        shadow = shadow
            .push(color_field(app, colors.1, sh.color))
            .push(slide(
                0.0..=20.0,
                1.0,
                sh.offset.0,
                format!("{:.0} px", sh.offset.0),
                move |v| wrap(DraftEdit::ShadowOffset(v)),
            ));
    }
    let mut outline = row![toggle_box(t.outline.is_some(), move |v| wrap(
        DraftEdit::OutlineOn(v)
    ))]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    if let Some(o) = t.outline {
        outline = outline
            .push(color_field(app, colors.2, o.color))
            .push(slide(
                0.0..=10.0,
                0.5,
                o.width,
                format!("{:.1} px", o.width),
                move |v| wrap(DraftEdit::OutlineWidth(v)),
            ));
    }

    let mut rows: Vec<Element<'a, Msg>> = name_row.into_iter().collect();
    rows.extend([
        inherit_row(
            &s.theme_font_label,
            s,
            t.font == Prop::Inherit,
            move |v| wrap(DraftEdit::FontInherit(v)),
            font,
            None,
            file.font,
        ),
        inherit_row(
            &s.theme_size_label,
            s,
            t.size == Prop::Inherit,
            move |v| wrap(DraftEdit::SizeInherit(v)),
            size,
            None,
            file.size,
        ),
        inherit_row(
            &s.theme_bold_label,
            s,
            t.bold == Prop::Inherit,
            move |v| wrap(DraftEdit::BoldInherit(v)),
            bold,
            None,
            file.bold,
        ),
        inherit_row(
            &s.theme_text_color_label,
            s,
            t.color == Prop::Inherit,
            move |v| wrap(DraftEdit::ColorInherit(v)),
            color,
            color_id,
            file.color,
        ),
        inherit_row(
            &s.theme_align_label,
            s,
            t.h_align == Prop::Inherit,
            move |v| wrap(DraftEdit::HAlignInherit(v)),
            h_align,
            None,
            file.h_align,
        ),
        field(&s.theme_valign_label, v_align),
        inherit_row(
            &s.theme_line_spacing_label,
            s,
            t.line_spacing == Prop::Inherit,
            move |v| wrap(DraftEdit::SpacingInherit(v)),
            slide(
                0.8..=2.5,
                0.05,
                spacing_v,
                format!("{spacing_v:.2}"),
                move |v| wrap(DraftEdit::Spacing(v)),
            ),
            None,
            file.spacing,
        ),
        inherit_row(
            &s.theme_margin_label,
            s,
            t.margin == Prop::Inherit,
            move |v| wrap(DraftEdit::MarginInherit(v)),
            slide(
                0.0..=0.3,
                0.01,
                margin_v,
                format!("{:.0} %", margin_v * 100.0),
                move |v| wrap(DraftEdit::Margin(v)),
            ),
            None,
            None,
        ),
        field(
            &s.theme_opacity,
            slide(
                0.0..=1.0,
                0.05,
                t.opacity,
                format!("{:.0} %", t.opacity * 100.0),
                move |v| wrap(DraftEdit::Opacity(v)),
            ),
        ),
        field(&s.theme_shadow, shadow),
        field(&s.theme_outline, outline),
    ]);
    section(title, rows)
}

fn text_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let name = field(
        &s.theme_name_label,
        text_input(s.theme_name_label.as_str(), &d.name)
            .on_input(|n| edit(DraftEdit::Name(n)))
            .style(ui_theme::input)
            .size(13)
            .padding(6)
            .width(260),
    );
    text_style_section(
        app,
        &s.theme_text,
        &d.text,
        inherited(app),
        (ColorField::Text, ColorField::Shadow, ColorField::Outline),
        |e| e,
        Some(name),
        Some(COLOR_INHERIT_ID),
    )
}

/// The reference line's own style. A checked Inherit box shows the value
/// derived from the verse text.
fn reference_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let r = sda_core::resolve::reference_style(d);
    let file = Inherited {
        font: Some(
            r.font
                .resolve(None, Font::Bundled(BundledFont::Inter))
                .family()
                .to_string(),
        ),
        size: Some(match r.size.resolve(None, TextSize::Auto) {
            TextSize::Pt(p) => format!("{p:.0}pt"),
            TextSize::Auto => s.theme_auto_size.clone(),
        }),
        bold: Some(
            if r.bold.resolve(None, false) {
                "✓"
            } else {
                "✗"
            }
            .to_string(),
        ),
        color: Some(r.color.resolve(None, Rgba::WHITE).to_hex()),
        h_align: Some(align_label(s, r.h_align.resolve(None, HAlign::Center))),
        spacing: Some(format!("{:.2}", r.line_spacing.resolve(None, 1.1))),
    };
    text_style_section(
        app,
        &s.theme_reference,
        &d.reference,
        file,
        (
            ColorField::RefText,
            ColorField::RefShadow,
            ColorField::RefOutline,
        ),
        |e| DraftEdit::Reference(Box::new(e)),
        None,
        None,
    )
}

fn bg_kind_label(s: &Strings, kind: BgKind) -> &str {
    match kind {
        BgKind::Solid => &s.theme_bg_solid,
        BgKind::Gradient => &s.theme_bg_gradient,
        BgKind::Particles => &s.theme_bg_particles,
        BgKind::Image => &s.theme_bg_image,
        BgKind::Video => &s.theme_bg_video,
    }
}

fn background_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    // A pick list like the transition kind below it. Five labels in a
    // segmented row overflow the form column.
    let kinds: Vec<Choice<BgKind>> = [
        BgKind::Solid,
        BgKind::Gradient,
        BgKind::Particles,
        BgKind::Image,
        BgKind::Video,
    ]
    .into_iter()
    .map(|k| choice(k, bg_kind_label(s, k)))
    .collect();
    let current = bg_kind(&d.background);
    let kind = pick_list(
        kinds,
        Some(choice(current, bg_kind_label(s, current))),
        |k| edit(DraftEdit::BgKind(k.value)),
    )
    .style(ui_theme::picker)
    .text_size(13)
    .padding([4, 8]);
    let mut rows: Vec<Element<'a, Msg>> = vec![field(&s.theme_kind, kind)];
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
                &s.angle,
                slide(
                    0.0..=360.0,
                    5.0,
                    *angle,
                    format!("{angle:.0} °"),
                    DraftEdit::BgAngle,
                ),
            ));
            rows.push(field(
                &s.drift,
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
                    text(count.to_string())
                        .size(13)
                        .color(ui_theme::palette().muted),
                ]
                .spacing(ui_theme::SPACING / 2.0)
                .align_y(Alignment::Center),
            ));
            rows.push(field(
                &s.speed,
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
                    .color(ui_theme::palette().muted),
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
                    .color(ui_theme::palette().muted),
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
    let kinds: Vec<Choice<TransitionKind>> = KINDS
        .into_iter()
        .map(|k| choice(k, kind_label(s, k)))
        .collect();
    let kind = pick_list(kinds, Some(choice(tr.kind, kind_label(s, tr.kind))), |k| {
        edit(DraftEdit::TransitionKind(k.value))
    })
    .style(ui_theme::picker)
    .text_size(13)
    .padding([4, 8]);
    let easings: Vec<Choice<Easing>> = EASINGS
        .into_iter()
        .map(|e| choice(e, easing_label(s, e)))
        .collect();
    let easing = pick_list(
        easings,
        Some(choice(tr.easing, easing_label(s, tr.easing))),
        |e| edit(DraftEdit::Easing(e.value)),
    )
    .style(ui_theme::picker)
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
            .color(ui_theme::palette().muted),
    ]
    .spacing(ui_theme::SPACING / 2.0)
    .align_y(Alignment::Center);
    section(
        &s.theme_transition,
        vec![
            field(&s.theme_kind, kind),
            field(&s.theme_duration, duration),
            field(&s.theme_easing, easing),
        ],
    )
}

fn verses_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let v = d.verses;
    section(
        &s.theme_verses,
        vec![
            field(
                &s.theme_min_size,
                slide(
                    16.0..=60.0,
                    1.0,
                    v.min_size,
                    format!("{:.0} pt", v.min_size),
                    DraftEdit::MinSize,
                ),
            ),
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

/// The left form column: Text and Reference. The preview sits above it.
pub fn left_column(app: &App) -> Element<'_, Msg> {
    let d = &app.themes.draft;
    column![text_section(app, d), reference_section(app, d)]
        .spacing(ui_theme::SPACING)
        .into()
}

/// The right form column: Background, Transition, Verses, Footer.
pub fn right_column(app: &App) -> Element<'_, Msg> {
    let d = &app.themes.draft;
    column![
        background_section(app, d),
        transition_section(app, d),
        verses_section(app, d),
        footer_section(app, d),
    ]
    .spacing(ui_theme::SPACING)
    .into()
}
