//! A capture session that stays open across frames.
//!
//! The one-shot [`super::capture_modern::capture_window`] path builds a device,
//! a frame pool and a capture session per frame, which is right for a single
//! screenshot and wrong for a 30 fps game session. A [`PersistentSession`]
//! builds them once and asks the pool for the next frame on every sample.

use std::time::Duration;

use acdesktop_core::{AdapterError, Deadline, ErrorCode};
use windows::Graphics::Capture::Direct3D11CaptureFrame;
use windows::core::{HRESULT, Interface};

use super::capture_d3d::{self, CaptureDevice, resource_balance};
use super::capture_modern::{self, FrameGuard, PoolGuard, SessionGuard, TextureGuard, wgc_error};
use crate::system::window::window_enum::WindowHandle;

const FRAME_POLL_SLICE: Duration = Duration::from_millis(1);

/// One sampled frame, still in BGRA row order.
#[derive(Clone)]
pub(in crate::system) struct FramePixels {
    pub(in crate::system) pixels: Vec<u8>,
    pub(in crate::system) width: u32,
    pub(in crate::system) height: u32,
}

pub(in crate::system) struct PersistentSession {
    device: CaptureDevice,
    pool: PoolGuard,
    _session: SessionGuard,
    width: u32,
    height: u32,
}

impl PersistentSession {
    pub(in crate::system) fn open(
        handle: WindowHandle,
        capture_cursor: bool,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        deadline.ensure_remaining()?;
        capture_modern::ensure_supported()?;
        let item = capture_modern::item_for_window(handle)?;
        let size = item
            .Size()
            .map_err(|error| wgc_error(error.code().0, "read the capture item size"))?;
        if size.Width <= 0 || size.Height <= 0 {
            return Err(AdapterError::new(
                ErrorCode::InvalidArgs,
                "capture item has a zero-sized frame",
            ));
        }
        let device = CaptureDevice::create()?;
        deadline.ensure_remaining()?;
        let pool = PoolGuard::create(&device.winrt, size)?;
        let session = SessionGuard::create(&pool, &item)?;
        session
            .0
            .SetIsCursorCaptureEnabled(capture_cursor)
            .map_err(|error| {
                AdapterError::not_supported("game capture cursor configuration")
                    .with_platform_detail(format!(
                        "Windows Graphics Capture cursor configuration failed: {error}"
                    ))
            })?;
        let observed_cursor = session.0.IsCursorCaptureEnabled().map_err(|error| {
            AdapterError::not_supported("game capture cursor configuration").with_platform_detail(
                format!("Windows Graphics Capture cursor setting could not be read: {error}"),
            )
        })?;
        if observed_cursor != capture_cursor {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "Windows did not apply the requested cursor capture setting",
            ));
        }
        let _ = session.0.SetIsBorderRequired(false);
        session
            .0
            .StartCapture()
            .map_err(|error| wgc_error(error.code().0, "start the capture session"))?;
        Ok(Self {
            device,
            pool,
            _session: session,
            width: size.Width as u32,
            height: size.Height as u32,
        })
    }

    pub(in crate::system) fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the next frame the pool has ready, or `Ok(None)` when the game
    /// has not produced a new one since the last sample. Never blocks on a
    /// frame: the session loop owns the schedule, so a missing frame is a
    /// skipped sample rather than a stall.
    pub(in crate::system) fn grab(
        &self,
        deadline: Deadline,
    ) -> Result<Option<FramePixels>, AdapterError> {
        deadline.ensure_remaining()?;
        let Some(frame) = try_next_frame(&self.pool)? else {
            return Ok(None);
        };
        let content = frame
            .0
            .ContentSize()
            .map_err(|error| wgc_error(error.code().0, "read the frame content size"))?;
        let width = content.Width.max(0) as u32;
        let height = content.Height.max(0) as u32;
        if width == 0 || height == 0 {
            return Ok(None);
        }
        if width != self.width || height != self.height {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "Capture dimensions changed during the game session",
            ));
        }
        let surface = frame
            .0
            .Surface()
            .map_err(|error| wgc_error(error.code().0, "read the frame surface"))?;
        let texture = capture_d3d::texture_from_surface(&surface)?;
        resource_balance::acquire();
        let _texture_guard = TextureGuard;
        let pixels = capture_d3d::read_texture_bgra(&self.device, &texture, width, height)?;
        Ok(Some(FramePixels {
            pixels,
            width,
            height,
        }))
    }
}

fn try_next_frame(pool: &PoolGuard) -> Result<Option<FrameGuard>, AdapterError> {
    let mut raw = std::ptr::null_mut();
    let status = unsafe {
        (Interface::vtable(&pool.0).TryGetNextFrame)(Interface::as_raw(&pool.0), &mut raw)
    };
    frame_from_abi(status, raw)
}

fn frame_from_abi(
    status: HRESULT,
    raw: *mut std::ffi::c_void,
) -> Result<Option<FrameGuard>, AdapterError> {
    if status.is_err() {
        if !raw.is_null() {
            drop(unsafe { Direct3D11CaptureFrame::from_raw(raw) });
        }
        return Err(wgc_error(status.0, "read the next capture frame"));
    }
    if raw.is_null() {
        return Ok(None);
    }
    Ok(Some(FrameGuard::new(unsafe {
        Direct3D11CaptureFrame::from_raw(raw)
    })))
}

/// The shortest slice the session sleeps instead of spinning. Below this the
/// loop busy-waits so an input lands inside the millisecond of its schedule.
pub(in crate::system) fn wait_until(
    target: std::time::Instant,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    loop {
        let now = std::time::Instant::now();
        if now >= target {
            return Ok(());
        }
        deadline.ensure_remaining()?;
        let remaining = target - now;
        if remaining > FRAME_POLL_SLICE {
            std::thread::sleep(remaining - FRAME_POLL_SLICE);
        } else {
            std::hint::spin_loop();
        }
    }
}

#[cfg(test)]
#[path = "persistent_tests.rs"]
mod tests;
