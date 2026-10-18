//! Videos tab view. Owned by the Videos tab task. One card for the
//! downloader and one for the history.

use crate::app::App;
use crate::msg::{Msg, VideosMsg};
use crate::state::videos::{self, Phase};
use crate::ui_theme;
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, space, text, text_input,
};
use iced::{Alignment, Element, Fill};

pub fn show(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let v = &app.videos;
    let busy = matches!(v.phase, Phase::Probing | Phase::Downloading);

    let url_row = row![
        text_input(&s.downloader_url_placeholder, &v.url)
            .on_input(|u| Msg::Videos(VideosMsg::Url(u)))
            .on_submit(Msg::Videos(VideosMsg::Fetch))
            .padding(10)
            .width(Fill),
        button(text(&s.videos_fetch).size(13))
            .padding([10, 14])
            .style(ui_theme::ghost_button)
            .on_press_maybe(
                (!busy && !v.url.trim().is_empty()).then_some(Msg::Videos(VideosMsg::Fetch))
            ),
    ]
    .spacing(ui_theme::SPACING);

    let mut card = column![
        text(&s.videos).size(18).font(ui_theme::BOLD),
        text(&s.downloader_subtitle).size(13).color(ui_theme::MUTED),
        url_row,
    ]
    .spacing(ui_theme::SPACING);

    if let Some(p) = &v.probe {
        card = card.push(text(&p.title).size(15));
        let mut qualities = row![text(&s.videos_quality).size(13).color(ui_theme::MUTED)]
            .spacing(ui_theme::SPACING / 2.0)
            .align_y(Alignment::Center);
        for q in &p.qualities {
            let active = v.format_id.as_deref() == Some(q.format_id.as_str());
            qualities = qualities.push(
                button(text(&q.label).size(13))
                    .padding([6, 12])
                    .style(ui_theme::segment_button(active))
                    .on_press(Msg::Videos(VideosMsg::Quality(q.format_id.clone()))),
            );
        }
        card = card.push(qualities);
    }

    let dir = videos::dir(app);
    let folder_row = row![
        text(&s.videos_folder).size(13).color(ui_theme::MUTED),
        text(dir.to_string_lossy().into_owned()).size(13),
        space::horizontal(),
        button(text(&s.choose).size(13))
            .padding([6, 12])
            .style(ui_theme::ghost_button)
            .on_press_maybe((!busy).then_some(Msg::Videos(VideosMsg::ChooseFolder))),
        button(text(&s.videos_download).size(13))
            .padding([6, 14])
            .style(ui_theme::accent_button)
            .on_press_maybe(
                (v.probe.is_some() && !busy).then_some(Msg::Videos(VideosMsg::Download))
            ),
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    card = card.push(folder_row);

    match &v.phase {
        Phase::Downloading => {
            let detail = if v.message.is_empty() {
                format!("{:.0}%  {}  {}", v.percent, v.speed, v.eta)
            } else {
                v.message.clone()
            };
            card = card
                .push(progress_bar(0.0..=100.0, v.percent).girth(6))
                .push(text(detail).size(13).color(ui_theme::MUTED));
        }
        Phase::Done(path) => {
            card = card.push(
                row![
                    text(&v.message).size(13),
                    space::horizontal(),
                    button(text(&s.videos_reveal).size(13))
                        .padding([6, 12])
                        .style(ui_theme::ghost_button)
                        .on_press(Msg::Videos(VideosMsg::Reveal(path.clone()))),
                ]
                .spacing(ui_theme::SPACING)
                .align_y(Alignment::Center),
            );
        }
        Phase::Failed => card = card.push(text(&v.message).size(13)),
        Phase::Probing => card = card.push(text(&s.videos_fetch).size(13).color(ui_theme::MUTED)),
        Phase::Idle | Phase::Ready => {}
    }

    let mut history = column![text(&s.videos_history).size(15).font(ui_theme::BOLD)]
        .spacing(ui_theme::SPACING / 2.0);
    if app.core.downloads.is_empty() {
        history = history.push(text(&s.downloader_subtitle).size(13).color(ui_theme::MUTED));
    }
    for rec in &app.core.downloads {
        history = history.push(
            row![
                column![
                    text(&rec.title).size(13),
                    text(rec.path.to_string_lossy().into_owned())
                        .size(11)
                        .color(ui_theme::MUTED),
                ]
                .spacing(2),
                space::horizontal(),
                button(text(&s.videos_reveal).size(12))
                    .padding([4, 10])
                    .style(ui_theme::ghost_button)
                    .on_press(Msg::Videos(VideosMsg::Reveal(rec.path.clone()))),
            ]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
        );
    }

    scrollable(
        column![panel(card), panel(history)]
            .spacing(ui_theme::SPACING)
            .padding(ui_theme::SPACING)
            .max_width(900),
    )
    .width(Fill)
    .into()
}

fn panel<'a>(content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    container(content)
        .padding(ui_theme::SPACING * 1.5)
        .width(Fill)
        .style(ui_theme::card)
        .into()
}
