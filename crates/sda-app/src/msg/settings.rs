//! Settings tab messages. Owned by the Settings tab task.

use sda_core::asis::Converter;
use sda_core::i18n::{Language, Strings};
use std::path::PathBuf;

/// One group of settings shown at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    General,
    Library,
    Display,
    Presenting,
    About,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::General,
        Section::Library,
        Section::Display,
        Section::Presenting,
        Section::About,
    ];

    pub fn label(self, s: &Strings) -> &str {
        match self {
            Section::General => &s.settings_general,
            Section::Library => &s.settings_library,
            Section::Display => &s.settings_display,
            Section::Presenting => &s.settings_presenting,
            Section::About => &s.settings_about,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SettingsMsg {
    Noop,
    Section(Section),
    Language(Language),
    CheckUpdate,
    ForceSync,
    /// Open the folder picker.
    AddFolder,
    /// The picker closed. `None` means it was cancelled.
    FolderPicked(Option<PathBuf>),
    /// Library path to drop.
    RemoveFolder(String),
    /// Library path and its new enabled flag.
    EnableFolder(String, bool),
    DefaultDisplay(usize),
    Identify,
    DetectConverter,
    /// Result of the pptx converter detection.
    Detected(Option<Converter>),
    BibleDefault(String),
    RevealData,
}
