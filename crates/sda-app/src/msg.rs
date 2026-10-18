//! Every message the app handles. Workers send one of these when they
//! finish. Tab specific messages live in sub enums owned by the tab files.

pub mod bible;
pub mod present;
pub mod settings;
pub mod themes;
pub mod videos;

pub use bible::BibleMsg;
pub use present::PresentMsg;
pub use settings::SettingsMsg;
pub use themes::ThemesMsg;
pub use videos::VideosMsg;

use crate::displays::Display;
use iced::{Point, Size, window};
use sda_core::i18n::Strings;
use sda_core::search::Searcher;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Carries a value that is not `Clone` through a `Clone` message. Iced
/// buttons require `Message: Clone`, so a worker puts its result in here and
/// `update` takes it out once.
pub struct Handoff<T>(Arc<Mutex<Option<T>>>);

impl<T> Handoff<T> {
    pub fn new(value: T) -> Self {
        Handoff(Arc::new(Mutex::new(Some(value))))
    }

    /// The value, or `None` if it was already taken.
    pub fn take(&self) -> Option<T> {
        self.0.lock().ok().and_then(|mut v| v.take())
    }
}

impl<T> Clone for Handoff<T> {
    fn clone(&self) -> Self {
        Handoff(self.0.clone())
    }
}

impl<T> fmt::Debug for Handoff<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Handoff")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Present,
    Bible,
    Videos,
    Themes,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Present,
        Tab::Bible,
        Tab::Videos,
        Tab::Themes,
        Tab::Settings,
    ];

    /// The tab at position `i`, clamped to the last one.
    pub fn nth(i: usize) -> Tab {
        Tab::ALL[i.min(Tab::ALL.len() - 1)]
    }

    pub fn label(self, s: &Strings) -> &str {
        match self {
            Tab::Present => &s.tab_present,
            Tab::Bible => &s.tab_bible,
            Tab::Videos => &s.tab_videos,
            Tab::Themes => &s.tab_themes,
            Tab::Settings => &s.tab_settings,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Tab(Tab),
    Search(String),
    Select(usize),
    MoveSelection(i32),
    PreviewSlide(usize),
    GoLive,
    GoLiveAt(usize),
    FocusSearch,
    LiveNext,
    LivePrev,
    LiveBlank,
    LiveJump(usize),
    LiveStop,
    ProjectorToggle,
    ProjectorDisplay(usize),
    ProjectorClosed(window::Id),
    Tick(Instant),
    Indexed(Result<Handoff<Searcher>, String>),
    /// `Ok(None)` means the hymns zip was already current.
    Synced(Result<Option<Handoff<Searcher>>, String>),
    Reindexed(Result<Handoff<Searcher>, String>),
    UpdateChecked(String),
    ForceSync,
    Present(PresentMsg),
    Bible(BibleMsg),
    Videos(VideosMsg),
    Themes(ThemesMsg),
    Settings(SettingsMsg),
    AsIsToggle(bool),
    AsIsRendered(Result<(PathBuf, Vec<PathBuf>), String>),
    DisplaysListed(Vec<Display>),
    FontsListed(Vec<String>),
    WindowMoved(Point),
    WindowResized(Size),
    SaveWindow,
    Identify,
    IdentifyDone,
    Noop,
}
