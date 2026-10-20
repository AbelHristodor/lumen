//! Button labels with an icon on the left and a key hint on the right, so
//! every shortcut is visible where its action lives.

use crate::shortcuts::{self, Action};
use crate::ui_theme;
use crate::widgets::icon::{Icon, icon};
use iced::widget::{Row, container, row, text};
use iced::{Alignment, Element};

/// A key label as a small pill, `⌘K` or `Esc`.
pub fn key<'a, M: 'a>(label: impl ToString) -> Element<'a, M> {
    container(text(label.to_string()).size(10).font(ui_theme::BOLD))
        .padding([1, 5])
        .style(ui_theme::key_cap)
        .into()
}

/// `label` with the first key of `action` as a pill after it, and an icon
/// before it when given. Goes inside a `button`.
pub fn labeled<'a, M: 'a>(
    icon_: Option<Icon>,
    label: &str,
    size: f32,
    action: Option<Action>,
) -> Element<'a, M> {
    build(icon_, false, label, size, action)
}

/// `labeled` with the icon after the label, for "next" style buttons so
/// the arrow points the way the button goes.
pub fn labeled_end<'a, M: 'a>(
    icon_: Icon,
    label: &str,
    size: f32,
    action: Option<Action>,
) -> Element<'a, M> {
    build(Some(icon_), true, label, size, action)
}

fn build<'a, M: 'a>(
    icon_: Option<Icon>,
    at_end: bool,
    label: &str,
    size: f32,
    action: Option<Action>,
) -> Element<'a, M> {
    let mut r: Row<'a, M> = row![].spacing(6).align_y(Alignment::Center);
    if let (Some(i), false) = (icon_, at_end) {
        r = r.push(icon(i, size));
    }
    r = r.push(
        text(label.to_string())
            .size(size)
            .wrapping(text::Wrapping::None),
    );
    if let (Some(i), true) = (icon_, at_end) {
        r = r.push(icon(i, size));
    }
    if let Some(a) = action {
        let k = shortcuts::hint(a);
        if !k.is_empty() {
            r = r.push(key(k));
        }
    }
    r.into()
}
