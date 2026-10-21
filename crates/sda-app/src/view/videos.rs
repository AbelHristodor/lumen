//! Videos tab view. Owned by the Videos tab task. The picked file on the
//! left, the preview and its transport in the middle, the downloader and
//! its history on the right.

use crate::app::App;
use crate::msg::{Msg, VideosMsg};
use crate::shortcuts::Action;
use crate::state::videos::{self, Phase};
use crate::ui_theme;
use crate::widgets::card::card;
use crate::widgets::hint::labeled;
use crate::widgets::icon::{Icon, icon};
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::{
    button, column, container, image, progress_bar, row, scrollable, slider, space, text,
    text_input,
};
use iced::{Alignment, Element, Fill, Shrink};
use sda_core::media::format_time;
use sda_media::State;

const PICKER_W: f32 = 260.0;
const SIDE_W: f32 = 340.0;

pub fn show(app: &App) -> Element<'_, Msg> {
    let side = column![downloader(app), history(app)]
        .spacing(ui_theme::SPACING)
        .width(SIDE_W)
        .height(Fill);
    row![picker(app), preview(app), side]
        .spacing(ui_theme::SPACING)
        .padding([0, ui_theme::SPACING as u16])
        .height(Fill)
        .into()
}

/// The picked file and the Open button. Phase 2 grows this into a folder
/// list with thumbnails.
fn picker(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let v = &app.videos;
    let open = button(labeled(Some(Icon::Search), &s.videos_open_file, 13.0, None))
        .padding([6, 12])
        .style(ui_theme::accent_button)
        .on_press(Msg::Videos(VideosMsg::OpenFile));
    let mut col = column![ui_theme::section_label(&s.tab_videos), open].spacing(ui_theme::SPACING);
    match &v.picked {
        Some(p) => {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            col = col.push(text(name).size(13));
            if v.opening {
                col = col.push(text(&s.videos_opening).size(12).color(ui_theme::MUTED));
            }
        }
        None => col = col.push(text(&s.videos_pick_hint).size(13).color(ui_theme::MUTED)),
    }
    if let Some(e) = &v.error {
        let danger = ui_theme::theme().palette().danger;
        col = col.push(text(e.as_str()).size(12).color(danger));
    }
    card(col).width(PICKER_W).height(Fill).into()
}

/// The preview frame with the transport under it.
fn preview(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let v = &app.videos;
    let frame: Element<'_, Msg> = match (&v.player, app.gpu_ok) {
        (Some(p), true) => {
            let scene = std::sync::Arc::new(sda_render::Scene {
                size: (1280, 720),
                background: sda_render::BackgroundState::Solid(sda_core::theme::Rgba::BLACK),
                background_opacity: 1.0,
                current: Some(sda_render::Layer::Video(p.frames())),
                outgoing: None,
                transition: sda_core::theme::Transition::default(),
                blank: false,
                now_secs: 0.0,
            });
            scene_view(key("video_preview"), scene)
        }
        (Some(_), false) => text(&s.gpu_unavailable).size(13).center().into(),
        (None, _) => text(&s.videos_pick_hint)
            .size(13)
            .color(ui_theme::MUTED)
            .center()
            .into(),
    };
    let frame = container(frame)
        .width(Fill)
        .height(Fill)
        .style(ui_theme::thumb_frame);
    let mut col = column![ui_theme::section_label(&s.preview), frame].spacing(ui_theme::SPACING);
    if let Some(p) = &v.player {
        col = col.push(transport(app, p));
    }
    card(col).width(Fill).height(Fill).into()
}

fn transport<'a>(app: &'a App, p: &'a sda_media::Player) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let pos = p.position();
    let dur = p.duration();
    let playing = p.state() == State::Playing;
    let seek = slider(0.0..=dur.as_secs_f32().max(0.01), pos.as_secs_f32(), |x| {
        Msg::Videos(VideosMsg::Seek(x))
    })
    .step(0.1_f32);
    let time = text(format!("{} / {}", format_time(pos), format_time(dur)))
        .size(12)
        .color(ui_theme::MUTED)
        .wrapping(text::Wrapping::None);
    let ghost = |body: Element<'a, Msg>, msg: Msg| {
        button(body)
            .padding([6, 10])
            .style(ui_theme::ghost_button)
            .on_press(msg)
    };
    let back = ghost(
        icon(Icon::SkipBack, 14.0).into(),
        Msg::Videos(VideosMsg::Step(-10)),
    );
    let fwd = ghost(
        icon(Icon::SkipForward, 14.0).into(),
        Msg::Videos(VideosMsg::Step(10)),
    );
    let (play_icon, play_label) = if playing {
        (Icon::Square, s.pause.as_str())
    } else {
        (Icon::Play, s.play.as_str())
    };
    let toggle = ghost(
        labeled(Some(play_icon), play_label, 13.0, None),
        Msg::Videos(VideosMsg::Toggle),
    );
    let volume = slider(0.0..=1.0, app.videos.volume, |x| {
        Msg::Videos(VideosMsg::Volume(x))
    })
    .step(0.05_f32)
    .width(100);
    let mut vol_row = row![text(&s.volume).size(12).color(ui_theme::MUTED), volume]
        .spacing(6)
        .align_y(Alignment::Center);
    if p.muted() {
        vol_row = vol_row.push(iced::widget::tooltip(
            icon(Icon::EyeOff, 13.0).color(ui_theme::MUTED),
            container(text(&s.videos_muted_hint).size(12))
                .padding(6)
                .style(ui_theme::card),
            iced::widget::tooltip::Position::Top,
        ));
    }
    let state_hint: Element<'a, Msg> = if p.state() == State::Ended {
        text(&s.videos_ended).size(12).color(ui_theme::MUTED).into()
    } else {
        space::horizontal().into()
    };
    let present = button(labeled(
        Some(Icon::Play),
        &s.bible_present_selected,
        13.0,
        Some(Action::Present),
    ))
    .padding([6, 14])
    .style(ui_theme::accent_button)
    .on_press(Msg::Videos(VideosMsg::Present));
    column![
        row![time, seek]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center),
        row![back, toggle, fwd, state_hint, vol_row, present]
            .spacing(ui_theme::SPACING / 2.0)
            .align_y(Alignment::Center),
    ]
    .spacing(ui_theme::SPACING / 2.0)
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
        ui_theme::section_label(&s.videos_download),
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
    card(col).width(Fill).height(Shrink).into()
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
            ui_theme::section_label(&s.videos_history),
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
