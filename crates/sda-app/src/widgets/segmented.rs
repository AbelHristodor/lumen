//! A row of buttons where one is active. Used for the tab bar.

use crate::ui_theme;
use iced::widget::{Row, button, container, text};
use iced::{Alignment, Element};

/// `items` are label and value pairs. `on_pick` turns a value into a message.
pub fn segmented<'a, T: Copy + PartialEq + 'a, M: Clone + 'a>(
    items: impl IntoIterator<Item = (&'a str, T)>,
    selected: T,
    on_pick: impl Fn(T) -> M + 'a,
) -> Element<'a, M> {
    with_track(items, selected, on_pick, ui_theme::segment_track)
}

/// `segmented` with the darker track for use inside a card.
pub fn segmented_on_card<'a, T: Copy + PartialEq + 'a, M: Clone + 'a>(
    items: impl IntoIterator<Item = (&'a str, T)>,
    selected: T,
    on_pick: impl Fn(T) -> M + 'a,
) -> Element<'a, M> {
    with_track(items, selected, on_pick, ui_theme::segment_track_on_card)
}

fn with_track<'a, T: Copy + PartialEq + 'a, M: Clone + 'a>(
    items: impl IntoIterator<Item = (&'a str, T)>,
    selected: T,
    on_pick: impl Fn(T) -> M + 'a,
    track: fn(&iced::Theme) -> container::Style,
) -> Element<'a, M> {
    let buttons = items.into_iter().fold(Row::new(), |r, (label, value)| {
        r.push(
            button(text(label).size(14))
                .padding([6, 14])
                .style(ui_theme::segment_button(value == selected))
                .on_press(on_pick(value)),
        )
    });
    container(buttons.spacing(2).align_y(Alignment::Center))
        .padding(2)
        .style(track)
        .into()
}
