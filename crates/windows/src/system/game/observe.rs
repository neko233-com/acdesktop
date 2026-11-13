//! Frame sampling: a cheap region diff between consecutive frames.
//!
//! Comparing every pixel of a 1080p frame at 30 fps would spend more time on
//! the diff than on the game, so a fixed grid of about 65k samples stands in
//! for the full frame. That is dense enough to notice a HUD flash or a sprite
//! jump and cheap enough to stay inside the frame budget.

const SAMPLE_TARGET: u64 = 65_536;
const CHANNEL_DELTA: u8 = 8;

pub(in crate::system) fn changed_ratio(
    previous: Option<&[u8]>,
    current: &[u8],
    width: u32,
    height: u32,
) -> f64 {
    let Some(previous) = previous else {
        return 0.0;
    };
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|size| size.checked_mul(4));
    if width == 0
        || height == 0
        || expected != Some(current.len())
        || previous.len() != current.len()
    {
        return 0.0;
    }
    let step = grid_step(width, height);
    let mut sampled = 0u64;
    let mut changed = 0u64;
    let mut y = 0u32;
    while y < height {
        let row = y as usize * width as usize * 4;
        let mut x = 0u32;
        while x < width {
            let offset = row + x as usize * 4;
            sampled += 1;
            if !same_pixel(&previous[offset..offset + 4], &current[offset..offset + 4]) {
                changed += 1;
            }
            x = x.saturating_add(step);
        }
        y = y.saturating_add(step);
    }
    changed as f64 / sampled.max(1) as f64
}

fn grid_step(width: u32, height: u32) -> u32 {
    let pixels = u64::from(width) * u64::from(height);
    let cells = pixels.div_ceil(SAMPLE_TARGET).max(1) as f64;
    cells.sqrt().ceil().max(1.0) as u32
}

fn same_pixel(a: &[u8], b: &[u8]) -> bool {
    a[0].abs_diff(b[0]) <= CHANNEL_DELTA
        && a[1].abs_diff(b[1]) <= CHANNEL_DELTA
        && a[2].abs_diff(b[2]) <= CHANNEL_DELTA
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
