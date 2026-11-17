use std::time::{Duration, Instant};

use acdesktop_core::{AdapterError, Point};

use super::super::{
    geometry,
    raster::{self, Surface, SurfaceMapping},
    window::OverlayWindow,
};

const DURATION: Duration = Duration::from_millis(600);

pub(super) struct ClickPulse {
    window: OverlayWindow,
    point: Point,
    scale: f64,
    started: Instant,
}

impl ClickPulse {
    pub(super) fn new(point: Point, scale: f64) -> Result<Self, AdapterError> {
        Ok(Self {
            window: OverlayWindow::create()?,
            point,
            scale: scale.clamp(0.5, 4.0),
            started: Instant::now(),
        })
    }

    pub(super) fn draw(&mut self) -> Result<bool, AdapterError> {
        let elapsed = self.started.elapsed();
        if elapsed >= DURATION {
            return Ok(false);
        }
        let size = (geometry::RIPPLE_SIZE * self.scale).ceil() as i32 + 2;
        let origin = Point {
            x: (self.point.x - f64::from(size) / 2.0).floor(),
            y: (self.point.y - f64::from(size) / 2.0).floor(),
        };
        let mut surface = Surface::try_transparent(size, size)
            .ok_or_else(|| AdapterError::internal("Click effect surface is too large"))?;
        let mapping = SurfaceMapping {
            origin: origin.clone(),
            scale: self.scale,
        };
        let phase = (elapsed.as_secs_f64() / DURATION.as_secs_f64()).max(0.05);
        raster::draw_ripple(
            &mut surface,
            &mapping,
            &self.point,
            phase,
            [0.26, 0.60, 1.0],
        );
        self.window.present(
            origin.x as i32,
            origin.y as i32,
            size,
            size,
            &surface.pixels,
        )?;
        self.window.pump();
        self.window.raise();
        Ok(true)
    }
}
