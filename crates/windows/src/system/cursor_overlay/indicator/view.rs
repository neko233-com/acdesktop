use std::time::{Duration, Instant};

use acdesktop_core::{AdapterError, ErrorCode, Rect};

use super::super::{
    display_probe, monitors::OverlayMonitor, raster::Surface, text, window::OverlayWindow,
};
use super::{layout, pulse::ClickPulse};

pub(super) struct IndicatorView {
    windows: Vec<OverlayWindow>,
    monitors: Vec<OverlayMonitor>,
    description: String,
    keys: Option<String>,
    pulse: Option<ClickPulse>,
    checked: Instant,
}

impl IndicatorView {
    pub(super) fn new() -> Self {
        Self {
            windows: Vec::new(),
            monitors: Vec::new(),
            description: String::new(),
            keys: None,
            pulse: None,
            checked: Instant::now(),
        }
    }

    pub(super) fn present(&mut self, description: &str) -> Result<(), AdapterError> {
        let monitors = display_probe::monitors();
        if !self.windows.is_empty()
            && description == self.description
            && monitors == self.monitors
            && self.keys.is_none()
        {
            return Ok(());
        }
        self.description = description.to_owned();
        self.keys = None;
        if !monitors.is_empty() {
            self.monitors = monitors;
        }
        self.draw()
    }

    pub(super) fn set_keys(&mut self, keys: &str) -> Result<(), AdapterError> {
        self.keys = Some(keys.to_owned());
        self.draw()
    }

    pub(super) fn click(&mut self, point: &acdesktop_core::Point) -> Result<(), AdapterError> {
        point.validate()?;
        let monitor = self.monitors.iter().find(|monitor| {
            let bounds = &monitor.bounds;
            point.x >= bounds.x
                && point.x < bounds.x + bounds.width
                && point.y >= bounds.y
                && point.y < bounds.y + bounds.height
        });
        if let Some(monitor) = monitor {
            let mut pulse = ClickPulse::new(point.clone(), monitor.scale)?;
            pulse.draw()?;
            self.pulse = Some(pulse);
        }
        Ok(())
    }

    pub(super) fn idle_tick(&self) -> Duration {
        if self.pulse.is_some() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(250)
        }
    }

    pub(super) fn refresh(&mut self) -> Result<(), AdapterError> {
        for window in &self.windows {
            window.pump();
            window.raise();
        }
        if let Some(pulse) = &mut self.pulse {
            let finished = match pulse.draw() {
                Ok(drawing) => !drawing,
                Err(error) => {
                    tracing::warn!(code = %error.code.as_str(), "control click effect was stopped");
                    true
                }
            };
            if finished {
                self.pulse = None;
            }
        }
        if self.checked.elapsed() >= Duration::from_secs(1) {
            self.checked = Instant::now();
            let monitors = display_probe::monitors();
            if !monitors.is_empty() && monitors != self.monitors {
                self.monitors = monitors;
                self.draw()?;
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<(), AdapterError> {
        if self.monitors.is_empty() {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "No display is available for the control indicator",
            ));
        }
        let mut windows = Vec::new();
        for monitor in &self.monitors {
            let layout = layout::display_layout(monitor)?;
            for (x, y, w, h, edge) in layout.edges {
                let surface = edge_surface(w, h, edge, layout.edge)?;
                let window = OverlayWindow::create()?;
                window.present(x, y, w, h, &surface.pixels)?;
                windows.push(window);
            }
            let (x, y, width, height) = layout.status;
            let surface = status_surface(
                width,
                height,
                layout.scale,
                &self.description,
                self.keys.as_deref(),
            )?;
            let window = OverlayWindow::create()?;
            window.present(x, y, width, height, &surface.pixels)?;
            windows.push(window);
        }
        self.windows = windows;
        Ok(())
    }
}

fn edge_surface(
    width: i32,
    height: i32,
    edge: u8,
    thickness: i32,
) -> Result<Surface, AdapterError> {
    let mut surface = Surface::try_transparent(width, height)
        .ok_or_else(|| AdapterError::internal("Control indicator surface is too large"))?;
    for y in 0..height {
        for x in 0..width {
            let distance = match edge {
                0 => y,
                1 => height - y - 1,
                2 => x,
                _ => width - x - 1,
            };
            let alpha = ((thickness - distance).max(0) * 42 / thickness) as u32;
            let red = alpha * 66 / 255;
            let green = alpha * 153 / 255;
            surface.pixels[(y * width + x) as usize] =
                (alpha << 24) | (red << 16) | (green << 8) | alpha;
        }
    }
    Ok(surface)
}

fn status_surface(
    width: i32,
    height: i32,
    scale: f64,
    description: &str,
    keys: Option<&str>,
) -> Result<Surface, AdapterError> {
    let mut surface = Surface::try_transparent(width, height)
        .ok_or_else(|| AdapterError::internal("Control indicator label is too large"))?;
    surface.pixels.fill(0xff171c29);
    let rect = |y| Rect {
        x: 8.0 * scale,
        y: y * scale,
        width: (f64::from(width) - 16.0 * scale).max(0.0),
        height: 27.0 * scale,
    };
    text::draw_label(
        &mut surface,
        &rect(4.0),
        "acdesktop is controlling this computer",
        [0.85, 0.91, 1.0],
        15.0 * scale,
    );
    if let Some(keys) = keys {
        text::draw_label(
            &mut surface,
            &rect(60.0),
            keys,
            [0.72, 0.83, 1.0],
            14.0 * scale,
        );
    }
    text::draw_label(
        &mut surface,
        &rect(32.0),
        description,
        [1.0, 1.0, 1.0],
        14.0 * scale,
    );
    Ok(surface)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
