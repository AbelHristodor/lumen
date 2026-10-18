//! Settings tab messages. Owned by the Settings tab task.

use sda_core::asis::Converter;

#[derive(Debug, Clone)]
pub enum SettingsMsg {
    Noop,
    /// Result of the pptx converter detection.
    Detected(Option<Converter>),
}
