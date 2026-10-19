//! Bible tab view. Owned by the Bible tab task.

use crate::app::App;
use crate::msg::{BibleMsg, Msg};
use crate::state::bible::BibleState;
use crate::ui_theme;
use crate::widgets::card::card;
use crate::widgets::scene_view::{key, scene_view};
use crate::widgets::segmented::segmented_on_card;
use iced::widget::{
    Column, Row, button, center, column, container, mouse_area, opaque, pick_list, row, scrollable,
    space, stack, text, text_input,
};
use iced::{Alignment, Element, Fill};
use sda_core::bible::{VersionInfo, book_name, format_reference};
use sda_core::i18n::Strings;
use sda_core::resolve::SlideContent;
use std::fmt;

/// Id of the one search bar. Tests type into it.
pub const INPUT_ID: &str = "bible-input";

const CHAPTER_COLUMNS: usize = 6;

/// One installed version in the picker. Shows the upper case abbreviation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionChoice {
    pub code: String,
    pub label: String,
}

impl fmt::Display for VersionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn b(m: BibleMsg) -> Msg {
    Msg::Bible(m)
}

pub fn show(app: &App) -> Element<'_, Msg> {
    let st = &app.bible;
    let body: Element<'_, Msg> = if st.bible.is_some() {
        row![books_pane(app), verses_pane(app), preview_pane(app)]
            .spacing(ui_theme::SPACING)
            .height(Fill)
            .into()
    } else {
        empty_state(app)
    };
    let page = column![top_row(app), body]
        .spacing(ui_theme::SPACING)
        .padding([0, ui_theme::SPACING as u16])
        .width(Fill)
        .height(Fill);
    let page: Element<'_, Msg> = page.into();
    if st.manage_open {
        stack![page, opaque(manage_modal(app))].into()
    } else {
        page
    }
}

fn top_row(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.bible;
    let choices: Vec<VersionChoice> = st
        .installed
        .iter()
        .map(|code| VersionChoice {
            code: code.clone(),
            label: st.abbreviation(code),
        })
        .collect();
    let selected = st
        .version
        .as_deref()
        .and_then(|v| choices.iter().find(|c| c.code == v).cloned());
    // A pick list cannot be disabled, so while a version loads the picker
    // only offers the current choice and a pick of it is a no op.
    let choices = if st.busy.is_some() {
        selected.iter().cloned().collect()
    } else {
        choices
    };
    let picker = pick_list(choices, selected, |c| b(BibleMsg::Version(c.code)))
        .placeholder(s.bible_version.as_str())
        .style(ui_theme::picker)
        .text_size(13)
        .padding([7, 10]);
    let hint = if st.text_mode {
        &s.bible_search_hint
    } else {
        &s.bible_input_hint
    };
    let input = text_input(hint, &st.input)
        .id(INPUT_ID)
        .on_input(|t| b(BibleMsg::Input(t)))
        .on_submit(b(BibleMsg::Submit))
        .style(ui_theme::input)
        .padding([6, 10])
        .size(14)
        .width(Fill);
    let mode = segmented_on_card(
        [
            (s.bible_mode_reference.as_str(), false),
            (s.bible_mode_text.as_str(), true),
        ],
        st.text_mode,
        |on| b(BibleMsg::TextMode(on)),
    );
    let manage = button(text(&s.bible_manage).size(13))
        .padding([6, 12])
        .style(ui_theme::ghost_button)
        .on_press(b(BibleMsg::ManageOpen));
    row![picker, input, mode, manage]
        .spacing(ui_theme::SPACING)
        .align_y(Alignment::Center)
        .into()
}

fn empty_state(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.bible;
    let line = match (&st.error, &st.busy) {
        (Some(e), _) => e.clone(),
        (None, Some(_)) => s.status_loading.clone(),
        (None, None) => s.bible_empty.clone(),
    };
    let open = button(text(&s.bible_manage).size(13))
        .padding([6, 14])
        .style(ui_theme::accent_button)
        .on_press(b(BibleMsg::ManageOpen));
    card(center(
        column![text(line).size(15).color(ui_theme::MUTED), open]
            .spacing(ui_theme::SPACING)
            .align_x(Alignment::Center),
    ))
    .width(Fill)
    .height(Fill)
    .into()
}

fn heading(label: &str) -> Element<'_, Msg> {
    ui_theme::section_label(label).into()
}

/// Room for the scrollbar so rows and the grid never sit under it.
const SCROLL_GUTTER: f32 = 10.0;
/// Room under the last row so it is never cut by the card edge.
const SCROLL_TAIL: f32 = ui_theme::SPACING;

fn thin_scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(6)
            .margin(2)
            .scroller_width(6),
    )
}

fn gutter<'a>(content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    container(content)
        .padding(iced::Padding::ZERO.right(SCROLL_GUTTER).bottom(SCROLL_TAIL))
        .into()
}

fn books_pane(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.bible;
    let lang = app.core.language;
    let count = st
        .bible
        .as_ref()
        .map(|bb| bb.books.len())
        .unwrap_or(0)
        .min(66);
    let books = Column::with_children((0..count).map(|i| {
        button(text(book_name(i, lang)).size(13))
            .padding([4, 8])
            .width(Fill)
            .style(ui_theme::segment_button(i == st.book))
            .on_press(b(BibleMsg::Book(i)))
            .into()
    }))
    .spacing(2);
    card(
        column![
            heading(&s.bible_books),
            scrollable(gutter(books))
                .direction(thin_scrollbar())
                .height(Fill),
            heading(&s.bible_chapters),
            container(scrollable(gutter(chapter_grid(st))).direction(thin_scrollbar()))
                .max_height(190),
        ]
        .spacing(ui_theme::SPACING / 2.0),
    )
    .width(220)
    .height(Fill)
    .into()
}

fn chapter_grid(st: &BibleState) -> Element<'_, Msg> {
    let chapters: Vec<u32> = (1..=st.chapter_count()).collect();
    Column::with_children(chapters.chunks(CHAPTER_COLUMNS).map(|chunk| {
        let mut r = Row::new().spacing(4);
        for c in chunk.iter().copied() {
            r = r.push(
                button(text(c.to_string()).size(12).center())
                    .padding([4, 0])
                    .width(Fill)
                    .style(ui_theme::segment_button(c == st.chapter))
                    .on_press(b(BibleMsg::Chapter(c))),
            );
        }
        for _ in chunk.len()..CHAPTER_COLUMNS {
            r = r.push(space::horizontal());
        }
        r.into()
    }))
    .spacing(4)
    .into()
}

fn verses_pane(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.bible;
    let lang = app.core.language;
    let title = format_reference(&st.reference(), lang);
    let abbr = st
        .version
        .as_deref()
        .map(|v| st.abbreviation(v))
        .unwrap_or_default();
    let header = row![
        text(title).size(16).font(ui_theme::BOLD),
        container(text(abbr).size(12))
            .padding([2, 8])
            .style(ui_theme::chip),
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);

    let rows = Column::with_children(st.chapter_verses().into_iter().map(|(n, verse)| {
        let selected = st.selected.contains(&n);
        let line = row![
            text(n.to_string())
                .size(12)
                .font(ui_theme::BOLD)
                .color(ui_theme::MUTED)
                .width(26),
            text(verse).size(14).width(Fill),
        ]
        .spacing(4)
        .align_y(Alignment::Start);
        mouse_area(
            container(line)
                .padding([4, 8])
                .width(Fill)
                .style(ui_theme::verse_row(selected)),
        )
        .interaction(iced::mouse::Interaction::Pointer)
        .on_press(b(BibleMsg::Press(n)))
        .on_enter(b(BibleMsg::Enter(n)))
        .on_release(b(BibleMsg::Release))
        .into()
    }))
    .spacing(2);
    let list: Element<'_, Msg> = if st.chapter_count() == 0 {
        center(text(&s.bible_pick_chapter).size(13).color(ui_theme::MUTED)).into()
    } else {
        scrollable(gutter(rows))
            .direction(thin_scrollbar())
            .height(Fill)
            .into()
    };
    card(column![header, list].spacing(ui_theme::SPACING))
        .width(Fill)
        .height(Fill)
        .into()
}

const PREVIEW_WIDTH: f32 = 320.0;
const PREVIEW_H: f32 = (PREVIEW_WIDTH - 2.0 * ui_theme::SPACING) * 9.0 / 16.0;

fn preview_pane(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.bible;
    let lang = app.core.language;
    let slides = crate::state::bible::preview_slides(app);
    let count = slides.len();
    let i = st.preview_slide.min(count.saturating_sub(1));
    let w = PREVIEW_WIDTH - 2.0 * ui_theme::SPACING;
    let frame: Element<'_, Msg> = match slides.get(i) {
        None => center(text(&s.bible_preview_empty).size(13).color(ui_theme::MUTED)).into(),
        Some(content) if app.gpu_ok => {
            match crate::state::bible::preview_scene(
                app,
                content,
                (w as u32 * 2, PREVIEW_H as u32 * 2),
            ) {
                Some(scene) => scene_view(key("bible_preview"), scene),
                None => space::horizontal().into(),
            }
        }
        Some(SlideContent::Verses { verses, .. }) => {
            let body: String = verses
                .iter()
                .map(|(n, v)| format!("{n} {v}"))
                .collect::<Vec<_>>()
                .join("\n");
            container(text(body).size(12).center())
                .padding(ui_theme::SPACING)
                .into()
        }
        Some(_) => space::horizontal().into(),
    };
    let frame = container(frame)
        .width(w)
        .height(PREVIEW_H)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(ui_theme::thumb_frame);
    let nav = |label: &'static str, target: Option<usize>| {
        button(text(label).size(13))
            .padding([4, 10])
            .style(ui_theme::ghost_button)
            .on_press_maybe(target.map(|t| b(BibleMsg::PreviewSlide(t))))
    };
    let counter = row![
        nav("‹", (i > 0).then(|| i - 1)),
        container(text(format!("{} / {}", i + 1, count.max(1))).size(12))
            .padding([3, 8])
            .style(ui_theme::chip),
        nav("›", (i + 1 < count).then_some(i + 1)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    let clear = button(text(&s.clear_selection).size(13))
        .padding([6, 12])
        .style(ui_theme::ghost_button)
        .on_press_maybe((!st.selected.is_empty()).then_some(b(BibleMsg::ClearSelection)));
    let present = button(text(&s.bible_present_selected).size(13))
        .padding([6, 14])
        .style(ui_theme::accent_button)
        .on_press_maybe((count > 0).then_some(b(BibleMsg::Present)));
    let actions = row![counter, space::horizontal(), clear, present]
        .spacing(ui_theme::SPACING / 2.0)
        .align_y(Alignment::Center);
    let mut col = column![heading(&s.preview), frame, actions].spacing(ui_theme::SPACING);
    if st.text_mode {
        let results = Column::with_children(st.results.iter().map(|(r, verse)| {
            button(
                column![
                    text(format_reference(r, lang))
                        .size(12)
                        .font(ui_theme::BOLD),
                    text(verse.as_str()).size(12).color(ui_theme::MUTED),
                ]
                .spacing(2),
            )
            .padding([6, 8])
            .width(Fill)
            .style(ui_theme::ghost_button)
            .on_press(b(BibleMsg::Jump(*r)))
            .into()
        }))
        .spacing(4);
        let list: Element<'_, Msg> = if st.results.is_empty() {
            center(
                text(&s.bible_search_empty)
                    .size(13)
                    .color(ui_theme::MUTED)
                    .center(),
            )
            .padding(ui_theme::SPACING)
            .into()
        } else {
            scrollable(gutter(results))
                .direction(thin_scrollbar())
                .height(Fill)
                .into()
        };
        col = col.push(list);
    }
    card(col).width(PREVIEW_WIDTH).height(Fill).into()
}

fn manage_modal(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.bible;
    let close = button(text(&s.close).size(13))
        .padding([4, 10])
        .style(ui_theme::ghost_button)
        .on_press(b(BibleMsg::ManageClose));
    let header = row![
        text(&s.bible_manage).size(16).font(ui_theme::BOLD),
        space::horizontal(),
        close,
    ]
    .align_y(Alignment::Center);
    let mut col = column![header].spacing(ui_theme::SPACING);
    if let Some(e) = &st.error {
        col = col.push(text(e.as_str()).size(12).color(ui_theme::MUTED));
    }
    let list: Element<'_, Msg> = if st.index.is_empty() {
        let line = if st.busy.is_some() {
            &s.status_loading
        } else {
            &s.bible_download
        };
        center(text(line.as_str()).size(13).color(ui_theme::MUTED))
            .height(120)
            .into()
    } else {
        scrollable(gutter(version_list(s, st)))
            .direction(thin_scrollbar())
            .into()
    };
    col = col.push(list);
    let dim = |_: &iced::Theme| container::Style {
        background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
        ..container::Style::default()
    };
    mouse_area(
        center(opaque(card(col).width(560).max_height(620)))
            .width(Fill)
            .height(Fill)
            .style(dim),
    )
    .on_press(b(BibleMsg::ManageClose))
    .into()
}

fn version_list<'a>(s: &'a Strings, st: &'a BibleState) -> Element<'a, Msg> {
    let mut col = Column::new().spacing(6);
    let mut language: Option<&str> = None;
    for v in &st.index {
        if language != Some(v.language.as_str()) {
            language = Some(&v.language);
            col = col.push(
                text(v.language.as_str())
                    .size(12)
                    .color(ui_theme::MUTED)
                    .font(ui_theme::BOLD),
            );
        }
        col = col.push(version_row(s, st, v));
    }
    col.into()
}

fn version_row<'a>(s: &'a Strings, st: &'a BibleState, v: &'a VersionInfo) -> Element<'a, Msg> {
    let installed = st.installed.contains(&v.code);
    let busy = st.busy.as_deref() == Some(v.code.as_str());
    let license = v.public_domain.map(|pd| {
        let label = if pd {
            &s.bible_public_domain
        } else {
            &s.bible_restricted
        };
        container(text(label.as_str()).size(11))
            .padding([2, 8])
            .style(ui_theme::chip)
    });
    let action = if installed {
        button(text(&s.bible_remove).size(12))
            .padding([4, 10])
            .style(ui_theme::ghost_button)
            .on_press_maybe((!busy).then_some(b(BibleMsg::Remove(v.code.clone()))))
    } else {
        button(text(&s.bible_download).size(12))
            .padding([4, 10])
            .style(ui_theme::accent_button)
            .on_press_maybe(
                st.busy
                    .is_none()
                    .then_some(b(BibleMsg::Download(v.code.clone()))),
            )
    };
    let mut r = row![
        text(v.abbreviation.to_uppercase()).size(13).width(70),
        text(v.name.as_str()).size(13).width(Fill),
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    if let Some(chip) = license {
        r = r.push(chip);
    }
    r.push(action).into()
}
