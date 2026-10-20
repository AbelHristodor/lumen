//! The strip at the bottom of the operator window. Shows what is on the
//! projector and holds the slide and projector controls.

use crate::app::App;
use crate::displays;
use crate::msg::Msg;
use crate::shortcuts::Action;
use crate::ui_theme;
use crate::widgets::hint::{labeled, labeled_end};
use crate::widgets::icon::Icon;
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{button, column, container, pick_list, row, space, text, tooltip};
use iced::{Alignment, Element, Fill};

pub fn show(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let live = &app.live;
    let loaded = live.present.is_loaded();

    // The frame is always 160 by 90 so the strip keeps one height on every
    // tab, live or not.
    let frame: Element<'_, Msg> = if app.gpu_ok && loaded {
        scene_view(key("live_strip"), live.scene((160, 90), app.now))
    } else if loaded {
        text(live.present.title.as_str()).size(12).center().into()
    } else {
        space::horizontal().into()
    };
    let thumb = column![
        ui_theme::section_label(&s.live),
        container(frame)
            .width(160)
            .height(90)
            .align_x(iced::Center)
            .align_y(iced::Center)
            .style(ui_theme::thumb_frame),
    ]
    .spacing(4);

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
            .width(Fill)
            .into()
    } else {
        text(&s.nothing_live)
            .size(13)
            .color(ui_theme::MUTED)
            .width(Fill)
            .into()
    };

    fn ghost<'a>(body: Element<'a, Msg>, msg: Option<Msg>) -> button::Button<'a, Msg> {
        button(body)
            .padding([6, 12])
            .style(ui_theme::ghost_button)
            .on_press_maybe(msg)
    }
    let when = |m: Msg| loaded.then_some(m);
    // Prev has its arrow on the left, Next on the right, so the pair
    // reads as a mirror.
    let controls = row![
        ghost(
            labeled(
                Some(Icon::ChevronLeft),
                &s.prev_slide,
                13.0,
                Some(Action::Prev)
            ),
            when(Msg::LivePrev)
        ),
        ghost(
            labeled_end(Icon::ChevronRight, &s.next_slide, 13.0, Some(Action::Next)),
            when(Msg::LiveNext)
        ),
        ghost(
            labeled(Some(Icon::EyeOff), &s.blank, 13.0, Some(Action::Blank)),
            when(Msg::LiveBlank)
        ),
        ghost(
            labeled(Some(Icon::Square), &s.stop, 13.0, Some(Action::Stop)),
            when(Msg::LiveStop)
        ),
    ]
    .spacing(ui_theme::SPACING / 2.0);

    let selected = app.projector_display.and_then(|i| app.displays.get(i));
    let picker = pick_list(app.displays.as_slice(), selected, |d| {
        Msg::ProjectorDisplay(d.index)
    })
    .placeholder(s.display.as_str())
    .style(ui_theme::picker)
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
    let toggle_icon = if app.projector.is_some() {
        Icon::MonitorOff
    } else {
        Icon::Monitor
    };
    let toggle = button(labeled(
        Some(toggle_icon),
        toggle_label,
        13.0,
        Some(Action::Projector),
    ))
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
        row![thumb, info, controls, picker, toggle]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
    )
    .padding(ui_theme::SPACING)
    .width(Fill)
    .style(ui_theme::card)
    .into()
}
