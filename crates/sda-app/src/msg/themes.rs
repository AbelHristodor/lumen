//! Themes tab messages. Owned by the Themes tab task.

#[derive(Debug, Clone)]
pub enum ThemesMsg {
    Noop,
    /// Make the named theme the one used for projection.
    SetActive(String),
}
