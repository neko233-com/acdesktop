use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use acdesktop_core::{AdapterError, Point, Rect};
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};

use super::{
    display_probe, monitors, notice_language, raster::Surface, text, window::OverlayWindow,
};

pub struct ControlNotice {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl ControlNotice {
    pub fn start() -> Result<Self, AdapterError> {
        let (stop, stopped) = mpsc::channel();
        let (ready, started) = mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            let result = show();
            match result {
                Ok(window) => {
                    if ready.send(Ok(())).is_err() {
                        return;
                    }
                    while let Err(RecvTimeoutError::Timeout) =
                        stopped.recv_timeout(Duration::from_millis(50))
                    {
                        window.pump();
                    }
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            }
        });
        match started.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => Ok(Self {
                stop,
                thread: Some(thread),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(AdapterError::internal(
                "The control notice could not be displayed in time",
            )),
        }
    }
}

impl Drop for ControlNotice {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn show() -> Result<OverlayWindow, AdapterError> {
    let label = notice_language::text(unsafe { GetUserDefaultUILanguage() });
    let monitors = display_probe::monitors();
    let mut bounds = unsafe { std::mem::zeroed() };
    let foreground = unsafe { GetForegroundWindow() };
    let point = if !foreground.is_null() && unsafe { GetWindowRect(foreground, &mut bounds) } != 0 {
        Point {
            x: f64::from(bounds.left + (bounds.right - bounds.left) / 2),
            y: f64::from(bounds.top + (bounds.bottom - bounds.top) / 2),
        }
    } else {
        monitors::resting_point(&monitors).ok_or_else(|| {
            AdapterError::internal("No display is available for the control notice")
        })?
    };
    let monitor = monitors::monitor_for_point(&monitors, &point)
        .ok_or_else(|| AdapterError::internal("No display is available for the control notice"))?;
    let scale = monitor.scale.clamp(1.0, 3.0);
    let font = 15.0 * scale;
    let width = (text::measure_label(label, font)
        .ok_or_else(|| AdapterError::internal("The control notice could not be measured"))?
        + 32.0 * scale)
        .ceil() as i32;
    let height = (34.0 * scale).ceil() as i32;
    let mut surface = Surface::try_transparent(width, height)
        .ok_or_else(|| AdapterError::internal("The control notice could not be allocated"))?;
    surface.pixels.fill(0xff263949);
    let rect = Rect {
        x: 8.0 * scale,
        y: 0.0,
        width: f64::from(width) - 16.0 * scale,
        height: f64::from(height),
    };
    text::draw_label(&mut surface, &rect, label, [1.0, 1.0, 1.0], font);
    let window = OverlayWindow::create()?;
    let x =
        (monitor.work_area.x + (monitor.work_area.width - f64::from(width)) / 2.0).round() as i32;
    let y = (monitor.work_area.y + 12.0 * scale).round() as i32;
    window.present(x, y, width, height, &surface.pixels)?;
    Ok(window)
}
