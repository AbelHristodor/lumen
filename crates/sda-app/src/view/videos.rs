//! Videos tab view. Owned by the Videos tab task. The downloader form on
//! the left, the history on the right.

use crate::app::App;
use crate::msg::{Msg, VideosMsg};
use crate::state::videos::{self, Phase};
use crate::ui_theme;
use crate::widgets::card::card;
use iced::widget::{
    button, column, container, image, progress_bar, row, scrollable, space, text, text_input,
};
use iced::{Alignment, Element, Fill};

pub fn show(app: &App) -> Element<'_, Msg> {
    row![downloader(app), history(app)]
        .spacing(ui_theme::SPACING)
        .padding([0, ui_theme::SPACING as u16])
        .height(Fill)
        .into()
}

fn ghost<'a>(label: &'a str, msg: Option<Msg>) -> Element<'a, Msg> {
    button(text(label).size(13))
        .padding([6, 12])
        .style(ui_theme::ghost_button)
        .on_press_maybe(msg)
        .into()
}

fn downloader(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let v = &app.videos;
    let busy = matches!(v.phase, Phase::Probing | Phase::Downloading);

    let url_row = row![
        text_input(&s.downloader_url_placeholder, &v.url)
            .on_input(|u| Msg::Videos(VideosMsg::Url(u)))
            .on_submit_maybe((!busy).then_some(Msg::Videos(VideosMsg::Fetch)))
            .style(ui_theme::input)
            .padding(10)
            .size(14)
            .width(Fill),
        button(text(&s.videos_fetch).size(13))
            .padding([10, 14])
            .style(ui_theme::ghost_button)
            .on_press_maybe(
                (!busy && !v.url.trim().is_empty()).then_some(Msg::Videos(VideosMsg::Fetch))
            ),
    ]
    .spacing(ui_theme::SPACING);

    let mut col = column![
        ui_theme::section_label::<Msg>(&s.videos),
        text(&s.downloader_subtitle).size(13).color(ui_theme::MUTED),
        url_row,
    ]
    .spacing(ui_theme::SPACING);

    if let Some(p) = &v.probe {
        let mut head = row![].spacing(ui_theme::SPACING).align_y(Alignment::Center);
        if let Some(h) = &v.thumbnail {
            head = head.push(image(h.clone()).width(160));
        }
        col = col.push(head.push(text(&p.title).size(15)));
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
        col = col.push(qualities);
    }

    let dir = videos::dir(app).to_string_lossy().into_owned();
    let folder_row = row![
        text(&s.videos_folder).size(13).color(ui_theme::MUTED),
        text_input("", &dir)
            .style(ui_theme::input)
            .padding([6, 10])
            .size(13)
            .width(Fill),
        ghost(
            &s.choose,
            (!busy).then_some(Msg::Videos(VideosMsg::ChooseFolder))
        ),
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    col = col.push(folder_row);

    match &v.phase {
        Phase::Downloading => {
            let detail = if v.message.is_empty() {
                format!("{:.0}%  {}  {}", v.percent, v.speed, v.eta)
            } else {
                v.message.clone()
            };
            col = col
                .push(progress_bar(0.0..=100.0, v.percent).girth(6))
                .push(text(detail).size(13).color(ui_theme::MUTED));
        }
        Phase::Done(path) => {
            col = col.push(
                row![
                    text(&v.message).size(13),
                    space::horizontal(),
                    ghost(
                        &s.videos_reveal,
                        Some(Msg::Videos(VideosMsg::Reveal(path.clone())))
                    ),
                ]
                .spacing(ui_theme::SPACING)
                .align_y(Alignment::Center),
            );
        }
        Phase::Failed => col = col.push(text(&v.message).size(13)),
        Phase::Probing => col = col.push(text(&s.probing).size(13).color(ui_theme::MUTED)),
        Phase::Idle | Phase::Ready => {}
    }

    col = col.push(
        row![
            space::horizontal(),
            button(text(&s.videos_download).size(13))
                .padding([6, 14])
                .style(ui_theme::accent_button)
                .on_press_maybe(
                    (v.probe.is_some() && !busy).then_some(Msg::Videos(VideosMsg::Download))
                ),
        ]
        .align_y(Alignment::Center),
    );
    card(col).width(Fill).height(Fill).into()
}

fn history(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let mut rows = column![].spacing(ui_theme::SPACING / 2.0);
    if app.core.downloads.is_empty() {
        rows = rows.push(text(&s.downloads_empty).size(13).color(ui_theme::MUTED));
    }
    for rec in &app.core.downloads {
        rows = rows.push(
            row![
                column![
                    text(&rec.title).size(13),
                    text(format!(
                        "{}  ·  {}",
                        ymd(rec.at),
                        rec.path.to_string_lossy()
                    ))
                    .size(11)
                    .color(ui_theme::MUTED),
                ]
                .spacing(2)
                .width(Fill),
                ghost(
                    &s.videos_reveal,
                    Some(Msg::Videos(VideosMsg::Reveal(rec.path.clone())))
                ),
            ]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
        );
    }
    card(
        column![
            ui_theme::section_label::<Msg>(&s.videos_history),
            scrollable(container(rows).padding(iced::Padding::ZERO.right(10))).height(Fill),
        ]
        .spacing(ui_theme::SPACING),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

/// `YYYY-MM-DD` in UTC for a unix time. Civil from days, Howard Hinnant.
pub fn ymd(unix: i64) -> String {
    let z = unix.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::ymd;

    #[test]
    fn ymd_matches_known_dates() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(951_782_400), "2000-02-29");
        assert_eq!(ymd(1_760_745_600), "2025-10-18");
    }
}
