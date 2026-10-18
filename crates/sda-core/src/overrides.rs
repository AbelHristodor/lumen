//! Per hymn user choices that live outside the hymn files.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Overrides {
    /// Hymn path to whether it is shown as is, with the original slide look.
    #[serde(default)]
    pub as_is: BTreeMap<PathBuf, bool>,
}
