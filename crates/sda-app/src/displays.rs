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
                label: format!("Display {} · {}x{}", i + 1, d.width, d.height),
                rect: logical_rect((d.x, d.y, d.width, d.height), d.scale_factor),
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

/// display-info reports physical pixels on Windows and logical points on
/// the other platforms. Iced wants logical points everywhere.
fn logical_rect(rect: (i32, i32, u32, u32), scale: f32) -> (i32, i32, u32, u32) {
    if cfg!(windows) {
        to_logical(rect, scale)
    } else {
        rect
    }
}

/// Divide a physical pixel rect by `scale`. A scale at or below zero is
/// treated as one.
pub fn to_logical(rect: (i32, i32, u32, u32), scale: f32) -> (i32, i32, u32, u32) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let (x, y, w, h) = rect;
    (
        (x as f32 / scale).round() as i32,
        (y as f32 / scale).round() as i32,
        (w as f32 / scale).round() as u32,
        (h as f32 / scale).round() as u32,
    )
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

#[cfg(test)]
mod tests {
    use super::to_logical;

    #[test]
    fn to_logical_divides_by_scale() {
        assert_eq!(
            to_logical((3840, 0, 3840, 2160), 2.0),
            (1920, 0, 1920, 1080)
        );
        assert_eq!(
            to_logical((-2880, 100, 2880, 1800), 1.5),
            (-1920, 67, 1920, 1200)
        );
        assert_eq!(to_logical((10, 20, 30, 40), 1.0), (10, 20, 30, 40));
        assert_eq!(to_logical((10, 20, 30, 40), 0.0), (10, 20, 30, 40));
    }
}
