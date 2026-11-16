use acdesktop_core::{AdapterError, ErrorCode, Rect};

use super::super::monitors::OverlayMonitor;

pub(super) struct DisplayLayout {
    pub(super) edges: [(i32, i32, i32, i32, u8); 4],
    pub(super) status: (i32, i32, i32, i32),
    pub(super) edge: i32,
    pub(super) scale: f64,
}

pub(super) fn display_layout(monitor: &OverlayMonitor) -> Result<DisplayLayout, AdapterError> {
    let bounds: Rect = monitor.bounds;
    bounds.validate()?;
    let scale = if monitor.scale.is_finite() && monitor.scale > 0.0 {
        monitor.scale.clamp(0.5, 4.0)
    } else {
        1.0
    };
    let x = bounds.x.round() as i32;
    let y = bounds.y.round() as i32;
    let width = bounds.width.round() as i32;
    let height = bounds.height.round() as i32;
    let edge = (24.0 * scale).round() as i32;
    if width <= edge * 2 || height <= edge * 2 {
        return Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "Display is too small for the control indicator",
        ));
    }
    let right = x.checked_add(width).ok_or_else(invalid_bounds)?;
    let bottom = y.checked_add(height).ok_or_else(invalid_bounds)?;
    let status_width = width.min((640.0 * scale).round() as i32);
    let status_height = (90.0 * scale).round().min(f64::from(height - edge * 2)) as i32;
    Ok(DisplayLayout {
        edges: [
            (x, y, width, edge, 0),
            (x, bottom - edge, width, edge, 1),
            (x, y + edge, edge, height - edge * 2, 2),
            (right - edge, y + edge, edge, height - edge * 2, 3),
        ],
        status: (
            x + (width - status_width) / 2,
            y + edge,
            status_width,
            status_height,
        ),
        edge,
        scale,
    })
}

fn invalid_bounds() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "Display bounds exceed the indicator geometry",
    )
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
