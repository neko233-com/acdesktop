use acdesktop_core::{AdapterError, ErrorCode};
use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, IsChild, WindowFromPoint};

use super::steps::StepAction;
use crate::system::window::window_enum::WindowHandle;

pub(in crate::system) fn verify_step(
    handle: WindowHandle,
    origin: (i32, i32),
    action: &StepAction,
) -> Result<(), AdapterError> {
    match action {
        StepAction::MouseMove { x, y } | StepAction::MouseButton { x, y, .. } => {
            verify_point(handle, origin, *x, *y)
        }
        StepAction::Touch(contacts) => {
            for contact in contacts {
                verify_point(handle, origin, contact.x, contact.y)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn verify_point(
    handle: WindowHandle,
    origin: (i32, i32),
    x: i32,
    y: i32,
) -> Result<(), AdapterError> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetClientRect(handle, &mut rect) } == 0 || !inside(rect.right, rect.bottom, x, y) {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            "Game input point is outside the target client area",
        ));
    }
    let absolute = match (origin.0.checked_add(x), origin.1.checked_add(y)) {
        (Some(x), Some(y)) => POINT { x, y },
        _ => {
            return Err(AdapterError::new(
                ErrorCode::InvalidArgs,
                "Game input point overflows screen coordinates",
            ));
        }
    };
    let hit = unsafe { WindowFromPoint(absolute) };
    if hit != handle && unsafe { IsChild(handle, hit) } == 0 {
        return Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "Game input point is obscured or no longer targets the selected window",
        ).with_details(serde_json::json!({"client_x": x, "client_y": y, "target": format!("{:?}", handle), "hit": format!("{:?}", hit)})));
    }
    Ok(())
}

fn inside(width: i32, height: i32, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < width && y < height
}

#[cfg(test)]
#[path = "target_tests.rs"]
mod tests;
