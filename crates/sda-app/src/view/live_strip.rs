//! The strip at the bottom of the operator window. Shows what is on the
//! projector and holds the slide and projector controls.

use crate::app::App;
use crate::displays;
use crate::msg::Msg;
use crate::ui_theme;
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{button, column, container, pick_list, row, space, text, tooltip};
use iced::{Alignment, Element, Fill};

pub fn show(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let live = &app.live;
    let loaded = live.present.is_loaded();

    let thumb: Element<'_, Msg> = if app.gpu_ok && loaded {
        container(scene_view(
            key("live_strip"),
            live.scene((160, 90), app.now),
        ))
        .width(160)
        .height(90)
        .style(ui_theme::thumb_frame)
        .into()
    } else if loaded {
        container(text(live.present.title.as_str()).size(12).center())
            .width(160)
            .height(90)
            .align_x(iced::Center)
            .align_y(iced::Center)
            .style(ui_theme::thumb_frame)
            .into()
    } else {
        text(&s.nothing_live).size(13).color(ui_theme::MUTED).into()
    };

    let info: Element<'_, Msg> = if loaded {
        let counter = format!(
            "{} / {}",
            live.present.index + 1,
            live.present.slide_count()
        );
        let mut col = column![text(live.present.title.as_str()).size(15)].spacing(2);
        if let Some(sub) = &live.present.subtitle {
            col = col.push(text(sub.as_str()).size(12).color(ui_theme::MUTED));
        }
        col.push(text(counter).size(12).color(ui_theme::MUTED))
            .into()
    } else {
        space::horizontal().into()
    };

    let ghost = |label: &str, msg: Option<Msg>| {
        button(text(label.to_string()).size(13))
            .padding([6, 12])
            .style(ui_theme::ghost_button)
            .on_press_maybe(msg)
    };
    let when = |m: Msg| loaded.then_some(m);
    let controls = row![
        ghost(&s.prev_slide, when(Msg::LivePrev)),
        ghost(&s.next_slide, when(Msg::LiveNext)),
        ghost(&s.blank, when(Msg::LiveBlank)),
        ghost(&s.stop, when(Msg::LiveStop)),
    ]
    .spacing(ui_theme::SPACING / 2.0);

    let selected = app.projector_display.and_then(|i| app.displays.get(i));
    let picker = pick_list(app.displays.as_slice(), selected, |d| {
        Msg::ProjectorDisplay(d.index)
    })
    .placeholder(s.display.as_str())
    .text_size(13)
    .padding([6, 10]);

    let toggle_label = if app.projector.is_some() {
        &s.projector_off
    } else {
        &s.projector_on
    };
    let enough =
        app.displays.len() >= 2 || (displays::single_display_allowed() && !app.displays.is_empty());
    let can_project = app.gpu_ok && (enough || app.projector.is_some());
    let toggle = button(text(toggle_label.as_str()).size(13))
        .padding([6, 14])
        .style(ui_theme::accent_button)
        .on_press_maybe(can_project.then_some(Msg::ProjectorToggle));
    let hint = if !app.gpu_ok {
        Some(&s.gpu_unavailable)
    } else if !can_project {
        Some(&s.single_display_hint)
    } else {
        None
    };
    let toggle: Element<'_, Msg> = match hint {
        Some(h) => tooltip(
            toggle,
            container(text(h.as_str()).size(12))
                .padding(8)
                .max_width(280)
                .style(ui_theme::card),
            tooltip::Position::Top,
        )
        .into(),
        None => toggle.into(),
    };

    container(
        row![thumb, info, space::horizontal(), controls, picker, toggle]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
    )
    .padding(ui_theme::SPACING)
    .width(Fill)
    .style(ui_theme::card)
    .into()
}
