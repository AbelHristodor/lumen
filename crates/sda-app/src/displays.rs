//! Connected displays, for the projector picker.

#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    pub index: usize,
    pub label: String,
    /// x, y, width, height in logical points.
    pub rect: (i32, i32, u32, u32),
    pub scale: f32,
    pub is_primary: bool,
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Every connected display in OS order. Empty when the query fails.
pub fn list() -> Vec<Display> {
    match display_info::DisplayInfo::all() {
        Ok(all) => all
            .into_iter()
            .enumerate()
            .map(|(i, d)| Display {
                index: i,
                label: format!("Display {} {}x{}", i + 1, d.width, d.height),
                rect: (d.x, d.y, d.width, d.height),
                scale: d.scale_factor,
                is_primary: d.is_primary,
            })
            .collect(),
        Err(e) => {
            log::warn!("display query failed: {e}");
            Vec::new()
        }
    }
}

/// The first display that is not the primary one, where the projector goes.
pub fn default_display(displays: &[Display]) -> Option<usize> {
    displays.iter().position(|d| !d.is_primary)
}

/// Debug switch. `SDA_ALLOW_SINGLE_DISPLAY=1` lets the projector open
/// fullscreen on the only screen so it can be tried on a laptop.
pub fn single_display_allowed() -> bool {
    std::env::var_os("SDA_ALLOW_SINGLE_DISPLAY").is_some_and(|v| v == "1")
}
