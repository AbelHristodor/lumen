//! Keyboard shortcuts. One table is the whole truth: `filter` maps key
//! presses to messages from it, buttons show its key labels through `hint`,
//! and the shortcut sheet lists it. Runs on every event the widgets did not
//! capture, so nothing fires while a text field has focus. Escape, modifier
//! changes, and left button releases are forwarded before the captured
//! check: Escape so it clears the Bible search while the field has focus,
//! the others as `Msg::Modifiers` and `BibleMsg::Release`.

use crate::msg::{BibleMsg, Msg, Tab, VideosMsg};
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};
use iced::{Event, event, window};
use sda_core::i18n::Strings;

/// What a shortcut does. `filter` turns it into a `Msg` for the current tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Tabs,
    FocusSearch,
    Projector,
    Present,
    Clear,
    Stop,
    Next,
    /// Space. Pauses a live video, else the next slide.
    Space,
    Prev,
    Blank,
    Jump,
    Up,
    Down,
    PreviewPrev,
    PreviewNext,
    Sheet,
}

/// The key half of a shortcut. `Digit` stands for 1 to 9 and `TabDigit`
/// for 1 to 5, both listed once in the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyDef {
    Char(&'static str),
    Named(Named),
    Digit,
    TabDigit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Everywhere,
    Present,
    Bible,
    /// The tabs that present content: Present, Bible, Videos.
    Content,
}

impl Scope {
    fn covers(self, tab: Tab) -> bool {
        match self {
            Scope::Everywhere => true,
            Scope::Present => tab == Tab::Present,
            Scope::Bible => tab == Tab::Bible,
            Scope::Content => matches!(tab, Tab::Present | Tab::Bible | Tab::Videos),
        }
    }
}

pub struct Shortcut {
    pub action: Action,
    key: KeyDef,
    /// Needs Cmd on macOS, Ctrl elsewhere.
    cmd: bool,
    pub scope: Scope,
}

const fn sc(action: Action, key: KeyDef, cmd: bool, scope: Scope) -> Shortcut {
    Shortcut {
        action,
        key,
        cmd,
        scope,
    }
}

/// Every shortcut, in the order the sheet shows them.
pub const TABLE: &[Shortcut] = &[
    sc(Action::Tabs, KeyDef::TabDigit, true, Scope::Everywhere),
    sc(
        Action::FocusSearch,
        KeyDef::Char("k"),
        true,
        Scope::Everywhere,
    ),
    sc(
        Action::FocusSearch,
        KeyDef::Char("f"),
        true,
        Scope::Everywhere,
    ),
    sc(
        Action::Present,
        KeyDef::Named(Named::Enter),
        false,
        Scope::Content,
    ),
    sc(
        Action::Clear,
        KeyDef::Named(Named::Escape),
        false,
        Scope::Everywhere,
    ),
    sc(
        Action::Next,
        KeyDef::Named(Named::ArrowRight),
        false,
        Scope::Everywhere,
    ),
    sc(
        Action::Space,
        KeyDef::Named(Named::Space),
        false,
        Scope::Everywhere,
    ),
    sc(
        Action::Prev,
        KeyDef::Named(Named::ArrowLeft),
        false,
        Scope::Everywhere,
    ),
    sc(Action::Blank, KeyDef::Char("b"), false, Scope::Everywhere),
    sc(Action::Stop, KeyDef::Char("."), true, Scope::Everywhere),
    sc(
        Action::Projector,
        KeyDef::Char("o"),
        true,
        Scope::Everywhere,
    ),
    sc(Action::Jump, KeyDef::Digit, false, Scope::Everywhere),
    sc(
        Action::Up,
        KeyDef::Named(Named::ArrowUp),
        false,
        Scope::Present,
    ),
    sc(
        Action::Down,
        KeyDef::Named(Named::ArrowDown),
        false,
        Scope::Present,
    ),
    sc(Action::PreviewPrev, KeyDef::Char("["), false, Scope::Bible),
    sc(Action::PreviewNext, KeyDef::Char("]"), false, Scope::Bible),
    sc(Action::Sheet, KeyDef::Char("/"), true, Scope::Everywhere),
];

/// Map a key press to a message on `tab`.
pub fn filter(event: Event, status: event::Status, _id: window::Id, tab: Tab) -> Option<Msg> {
    if let Event::Keyboard(keyboard::Event::ModifiersChanged(m)) = event {
        return Some(Msg::Modifiers(m));
    }
    // A release can land outside every verse row. It must still end a drag.
    // `Release` does nothing when no drag runs, so it is safe on every tab.
    if let Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) = event {
        return Some(Msg::Bible(BibleMsg::Release));
    }
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    let escape = matches!(key, Key::Named(Named::Escape));
    if status == event::Status::Captured && !escape {
        return None;
    }
    let cmd = modifiers.command();
    let key = key.as_ref();
    let digit = |max: usize| match key {
        Key::Character(d) => d.parse::<usize>().ok().filter(|n| (1..=max).contains(n)),
        _ => None,
    };
    for s in TABLE {
        if s.cmd != cmd || !s.scope.covers(tab) {
            continue;
        }
        let hit = match s.key {
            KeyDef::Char(c) => matches!(key, Key::Character(k) if k.eq_ignore_ascii_case(c)),
            KeyDef::Named(n) => key == Key::Named(n),
            KeyDef::Digit => digit(9).is_some(),
            KeyDef::TabDigit => digit(5).is_some(),
        };
        if !hit {
            continue;
        }
        return Some(match s.action {
            Action::Tabs => Msg::Tab(Tab::nth(digit(5).unwrap_or(1) - 1)),
            Action::Jump => Msg::LiveJump(digit(9).unwrap_or(1)),
            a => message(a, tab),
        });
    }
    None
}

/// The message an action sends on `tab`. `Tabs` and `Jump` carry a digit
/// and are built in `filter`, here they take the first one.
pub fn message(action: Action, tab: Tab) -> Msg {
    match action {
        Action::FocusSearch => Msg::FocusSearch,
        Action::Projector => Msg::ProjectorToggle,
        Action::Present if tab == Tab::Bible => Msg::Bible(BibleMsg::Present),
        Action::Present if tab == Tab::Videos => Msg::Videos(VideosMsg::Present),
        Action::Present => Msg::GoLive,
        Action::Clear => Msg::Clear,
        Action::Stop => Msg::LiveStop,
        Action::Next => Msg::LiveNext,
        Action::Space => Msg::LiveSpace,
        Action::Prev => Msg::LivePrev,
        Action::Blank => Msg::LiveBlank,
        Action::Up => Msg::MoveSelection(-1),
        Action::Down => Msg::MoveSelection(1),
        Action::PreviewPrev => Msg::Bible(BibleMsg::PreviewStep(-1)),
        Action::PreviewNext => Msg::Bible(BibleMsg::PreviewStep(1)),
        Action::Sheet => Msg::ToggleShortcuts,
        Action::Tabs => Msg::Tab(Tab::Present),
        Action::Jump => Msg::LiveJump(1),
    }
}

/// Short key label for a button, the first binding of `action`: `⌘K`,
/// `⏎`, `Esc`, `→`. Cmd reads `⌘` on macOS and `Ctrl+` elsewhere.
pub fn hint(action: Action) -> String {
    TABLE
        .iter()
        .find(|s| s.action == action)
        .map(key_label)
        .unwrap_or_default()
}

/// Every key label of `action`, for the sheet: `⌘K, ⌘F`.
pub fn hints(action: Action) -> String {
    TABLE
        .iter()
        .filter(|s| s.action == action)
        .map(key_label)
        .collect::<Vec<_>>()
        .join("  ")
}

fn key_label(s: &Shortcut) -> String {
    let key = match s.key {
        KeyDef::Char(c) => c.to_uppercase(),
        KeyDef::Named(Named::Enter) => "⏎".into(),
        KeyDef::Named(Named::Escape) => "Esc".into(),
        KeyDef::Named(Named::Space) => "Space".into(),
        KeyDef::Named(Named::ArrowLeft) => "←".into(),
        KeyDef::Named(Named::ArrowRight) => "→".into(),
        KeyDef::Named(Named::ArrowUp) => "↑".into(),
        KeyDef::Named(Named::ArrowDown) => "↓".into(),
        KeyDef::Named(n) => format!("{n:?}"),
        KeyDef::Digit => "1 … 9".into(),
        KeyDef::TabDigit => "1 … 5".into(),
    };
    if s.cmd {
        format!("{}{key}", cmd_label())
    } else {
        key
    }
}

fn cmd_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl+"
    }
}

/// The actions in sheet order, each once.
pub fn actions() -> Vec<Action> {
    let mut out: Vec<Action> = Vec::new();
    for s in TABLE {
        if !out.contains(&s.action) {
            out.push(s.action);
        }
    }
    out
}

/// Translated description of an action for the sheet.
pub fn label(action: Action, s: &Strings) -> &str {
    match action {
        Action::Tabs => &s.sc_tabs,
        Action::FocusSearch => &s.sc_search,
        Action::Projector => &s.sc_projector,
        Action::Present => &s.sc_present,
        Action::Clear => &s.sc_clear,
        Action::Stop => &s.sc_stop,
        Action::Next => &s.sc_next,
        Action::Space => &s.sc_space,
        Action::Prev => &s.sc_prev,
        Action::Blank => &s.sc_blank,
        Action::Jump => &s.sc_jump,
        Action::Up | Action::Down => &s.sc_move,
        Action::PreviewPrev | Action::PreviewNext => &s.sc_preview_step,
        Action::Sheet => &s.sc_sheet,
    }
}
