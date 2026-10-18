//! Settings tab view. Owned by the Settings tab task.

use crate::app::App;
use crate::msg::settings::Section;
use crate::msg::{Msg, SettingsMsg};
use crate::state::settings::{data_dir, installed_bibles};
use crate::ui_theme;
use crate::widgets::card::card;
use crate::widgets::segmented::segmented;
use iced::widget::{Column, button, column, container, pick_list, row, scrollable, text, toggler};
use iced::{Alignment, Element, Fill};
use sda_core::asis::Converter;
use sda_core::i18n::Language;
use sda_core::library;

#[path = "screen_picker.rs"]
pub mod screen_picker;

fn m(msg: SettingsMsg) -> Msg {
    Msg::Settings(msg)
}

/// Items under one heading sit close. Groups sit far apart.
fn group<'a>(items: impl IntoIterator<Item = Element<'a, Msg>>) -> Element<'a, Msg> {
    items
        .into_iter()
        .fold(
            Column::new().spacing(ui_theme::SPACING * 0.75),
            Column::push,
        )
        .into()
}

fn heading(label: &str) -> Element<'_, Msg> {
    text(label).size(16).font(ui_theme::BOLD).into()
}

fn hint(label: &str) -> Element<'_, Msg> {
    text(label).size(13).color(ui_theme::MUTED).into()
}

fn ghost<'a>(label: &'a str, msg: Option<Msg>) -> Element<'a, Msg> {
    button(text(label).size(13))
        .padding([6, 12])
        .style(ui_theme::ghost_button)
        .on_press_maybe(msg)
        .into()
}

fn accent<'a>(label: &'a str, msg: Option<Msg>) -> Element<'a, Msg> {
    button(text(label).size(13))
        .padding([6, 14])
        .style(ui_theme::accent_button)
        .on_press_maybe(msg)
        .into()
}

fn chip(label: String) -> Element<'static, Msg> {
    container(text(label).size(12))
        .padding([3, 8])
        .style(ui_theme::chip)
        .into()
}

/// Keep the start and the end of a long path.
fn truncate_middle(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let head = max / 3;
    let tail = max - head - 1;
    let start: String = s.chars().take(head).collect();
    let end: String = s.chars().skip(n - tail).collect();
    format!("{start}…{end}")
}

pub fn show(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let section = app.settings.section;
    let nav = Section::ALL
        .iter()
        .fold(Column::new().spacing(4), |col, sec| {
            col.push(
                button(text(sec.label(s)).size(14))
                    .padding([8, 12])
                    .width(Fill)
                    .style(ui_theme::segment_button(*sec == section))
                    .on_press(m(SettingsMsg::Section(*sec))),
            )
        });
    let body = match section {
        Section::General => general(app),
        Section::Library => library_section(app),
        Section::Display => display(app),
        Section::Presenting => presenting(app),
        Section::About => about(app),
    };
    row![
        container(nav).width(180),
        card(scrollable(body.spacing(ui_theme::SPACING * 2.0).width(Fill)).height(Fill))
            .width(Fill)
            .height(Fill),
    ]
    .spacing(ui_theme::SPACING)
    .padding(
        iced::Padding::ZERO
            .left(ui_theme::SPACING)
            .right(ui_theme::SPACING)
            .bottom(ui_theme::SPACING),
    )
    .height(Fill)
    .into()
}

fn general(app: &App) -> Column<'_, Msg> {
    let s = &app.core.strings;
    let langs = segmented(
        Language::all().into_iter().map(|l| (l.label(), l)),
        app.core.language,
        |l| m(SettingsMsg::Language(l)),
    );
    let checking = app.status.update.as_deref() == Some(s.update_checking.as_str());
    let mut updates = row![accent(
        &s.check_updates,
        (!checking).then_some(m(SettingsMsg::CheckUpdate)),
    )]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    if let Some(u) = app.status.update.as_deref().filter(|u| !u.is_empty()) {
        updates = updates.push(hint(u));
    }
    let version = format!("{}{}", s.version_prefix, env!("CARGO_PKG_VERSION"));
    column![
        group([heading(&s.language_heading), langs]),
        group([
            text(version).size(16).font(ui_theme::BOLD).into(),
            updates.into(),
        ]),
    ]
}

fn library_section(app: &App) -> Column<'_, Msg> {
    let s = &app.core.strings;
    let busy = app.status.sync_busy;
    let sync_label = if busy {
        &s.syncing_button
    } else {
        &s.force_sync_button
    };
    let sync_row = row![
        ghost(sync_label, (!busy).then_some(m(SettingsMsg::ForceSync))),
        hint(&app.status.library),
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);

    let folders: Vec<&library::Library> = app
        .core
        .cfg
        .libraries
        .iter()
        .filter(|l| !l.managed_by_git)
        .collect();
    let mut table = Column::new().spacing(6);
    if folders.is_empty() {
        table = table.push(hint(&s.user_libraries_description));
    }
    for lib in folders {
        let path = lib.path.clone();
        let mut r = row![
            container(text(lib.name.as_str()).size(13)).width(160),
            container(
                text(truncate_middle(&lib.path, 48))
                    .size(12)
                    .color(ui_theme::MUTED)
            )
            .width(Fill),
            toggler(lib.enabled).size(18).on_toggle({
                let path = path.clone();
                move |on| m(SettingsMsg::EnableFolder(path.clone(), on))
            }),
        ]
        .spacing(ui_theme::SPACING)
        .align_y(Alignment::Center);
        let state = if library::library_available(&lib.path) {
            &s.library_available
        } else {
            &s.library_missing
        };
        r = r.push(chip(state.clone()));
        r = r.push(ghost(&s.remove, Some(m(SettingsMsg::RemoveFolder(path)))));
        table = table.push(
            container(r)
                .padding([6, 10])
                .width(Fill)
                .style(|_| container::Style {
                    background: Some(ui_theme::SURFACE_HI.into()),
                    border: iced::Border::default().rounded(ui_theme::RADIUS),
                    ..container::Style::default()
                }),
        );
    }
    let mut folders_group = vec![
        heading(&s.user_libraries_heading),
        table.into(),
        accent(&s.add_folder_button, Some(m(SettingsMsg::AddFolder))),
    ];
    if let Some(e) = &app.settings.folder_error {
        folders_group.push(
            text(e.as_str())
                .size(13)
                .color(ui_theme::theme().palette().danger)
                .into(),
        );
    }
    column![
        group([
            heading(&s.library_heading),
            hint(&s.force_sync_description),
            sync_row.into(),
        ]),
        group(folders_group),
    ]
}

fn display(app: &App) -> Column<'_, Msg> {
    let s = &app.core.strings;
    let picker = screen_picker::show(&app.displays, app.projector_display, |i| {
        m(SettingsMsg::DefaultDisplay(i))
    });
    let selected = app.projector_display.and_then(|i| app.displays.get(i));
    let default = pick_list(app.displays.as_slice(), selected, |d| {
        m(SettingsMsg::DefaultDisplay(d.index))
    })
    .placeholder(s.display.as_str())
    .text_size(13)
    .padding([6, 10]);
    let controls = row![
        ghost(
            &s.identify,
            (!app.displays.is_empty()).then_some(m(SettingsMsg::Identify))
        ),
        hint(&s.default_display),
        default,
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    let mut items = vec![
        heading(&s.settings_display),
        container(picker)
            .padding(4)
            .style(|_| container::Style {
                background: Some(ui_theme::BG.into()),
                border: iced::Border::default().rounded(ui_theme::RADIUS),
                ..container::Style::default()
            })
            .into(),
        controls.into(),
        hint(&s.identify_hint),
    ];
    if app.displays.len() < 2 {
        items.push(hint(&s.single_display_hint));
    }
    column![group(items)]
}

fn presenting(app: &App) -> Column<'_, Msg> {
    let s = &app.core.strings;
    let mut conv = row![].spacing(ui_theme::SPACING).align_y(Alignment::Center);
    match &app.core.converter {
        Some(Converter::LibreOffice(p) | Converter::PowerPoint(p)) => {
            conv = conv
                .push(text(s.converter_found.as_str()).size(13))
                .push(chip(truncate_middle(&p.to_string_lossy(), 48)));
        }
        None => conv = conv.push(hint(&s.converter_missing)),
    }
    conv = conv.push(ghost(
        &s.detect_again,
        Some(m(SettingsMsg::DetectConverter)),
    ));

    let codes = installed_bibles(&app.core.paths.bibles_dir);
    let chosen = app
        .core
        .cfg
        .bible_version
        .clone()
        .filter(|c| codes.contains(c));
    let bible = row![
        hint(&s.bible_version),
        pick_list(codes, chosen, |c| m(SettingsMsg::BibleDefault(c)))
            .placeholder(s.choose.as_str())
            .text_size(13)
            .padding([6, 10]),
    ]
    .spacing(ui_theme::SPACING)
    .align_y(Alignment::Center);
    column![
        group([heading(&s.original), conv.into()]),
        group([heading(&s.tab_bible), bible.into()]),
    ]
}

fn about(app: &App) -> Column<'_, Msg> {
    let s = &app.core.strings;
    let version = format!("{}{}", s.version_prefix, env!("CARGO_PKG_VERSION"));
    let dir = data_dir(app);
    column![
        group([
            heading(&s.app_title),
            text(version).size(13).into(),
            hint(&s.font_licenses),
        ]),
        group([
            heading(&s.data_folder),
            row![
                text(truncate_middle(&dir.to_string_lossy(), 80)).size(13),
                ghost(&s.reveal_in_folder, Some(m(SettingsMsg::RevealData))),
            ]
            .spacing(ui_theme::SPACING)
            .align_y(Alignment::Center)
            .into(),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::truncate_middle;

    #[test]
    fn truncate_keeps_both_ends() {
        assert_eq!(truncate_middle("short", 10), "short");
        let t = truncate_middle("/Users/someone/Documents/Hymns/Folder", 20);
        assert_eq!(t.chars().count(), 20);
        assert!(t.starts_with("/Users"));
        assert!(t.ends_with("Folder"));
        assert!(t.contains('…'));
    }
}
