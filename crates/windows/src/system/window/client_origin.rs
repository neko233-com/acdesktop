//! The virtual-desktop point a window's client area starts at.

use super::window_enum::WindowHandle;
use acdesktop_core::{AdapterError, ErrorCode};

/// A game plan addresses points inside the window, so the session converts them
/// to absolute coordinates once, here, instead of asking every plan to know the
/// window's screen position and frame thickness.
#[cfg(target_os = "windows")]
pub(crate) fn client_origin(handle: WindowHandle) -> Result<(i32, i32), AdapterError> {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

    let mut rect = RECT {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    };
    if unsafe { GetClientRect(handle, &mut rect) } == 0 {
        return Err(AdapterError::new(
            ErrorCode::WindowNotFound,
            "the window's client area could not be read",
        ));
    }
    let mut point = POINT { x: 0, y: 0 };
    if unsafe { ClientToScreen(handle, &mut point) } == 0 {
        return Err(AdapterError::new(
            ErrorCode::WindowNotFound,
            "the window's client origin could not be resolved",
        ));
    }
    Ok((point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn client_origin(_handle: WindowHandle) -> Result<(i32, i32), AdapterError> {
    Err(AdapterError::not_supported("client_origin"))
}
