//! Turns a planned gesture into raw multi-touch frames.
//!
//! A frame never moves and lifts a contact at once: the OS rejects a lift that
//! changes the contact position, so every moving gesture lands its last
//! position on a `move` frame and lifts on a separate `up` frame.

use serde::{Deserialize, Serialize};

use super::touch::{TouchContact, TouchFrame, TouchPhase};

/// The gestures a timeline can plan without a platform-specific recognizer.
///
/// Every gesture is expressed as a sequence of touch frames, so a platform
/// only has to inject raw multi-touch contacts; it never needs to recognize
/// the gesture itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GestureName {
    Tap,
    DoubleTap,
    LongPress,
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
    PinchIn,
    PinchOut,
    RotateCw,
    RotateCcw,
    TwoFingerTap,
    DrawS,
    CircleCw,
    CircleCcw,
    ThreeFingerSwipeLeft,
    ThreeFingerSwipeRight,
    ThreeFingerSwipeUp,
    ThreeFingerSwipeDown,
}

impl GestureName {
    /// Whether the gesture needs more than one contact at once.
    pub fn needs_two_fingers(self) -> bool {
        matches!(
            self,
            Self::PinchIn | Self::PinchOut | Self::RotateCw | Self::RotateCcw | Self::TwoFingerTap
        )
    }

    /// Whether the gesture needs three contacts at once.
    pub fn needs_three_fingers(self) -> bool {
        matches!(
            self,
            Self::ThreeFingerSwipeLeft
                | Self::ThreeFingerSwipeRight
                | Self::ThreeFingerSwipeUp
                | Self::ThreeFingerSwipeDown
        )
    }
}

/// A gesture anchored at a virtual-screen point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GestureSpec {
    pub name: GestureName,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_duration_ms")]
    pub duration_ms: u32,
    #[serde(default = "default_distance")]
    pub distance: u32,
}

fn default_duration_ms() -> u32 {
    160
}

fn default_distance() -> u32 {
    160
}

const STEP_MS: u64 = 16;
const TAP_PRESS_MS: u64 = 60;
const DOUBLE_TAP_GAP_MS: u64 = 80;
const LONG_PRESS_MS: u64 = 600;
const MIN_SWIPE_MS: u64 = 160;
const MAX_SWIPE_MS: u64 = 1_200;

/// Expands one gesture into the touch frames a platform injects, with the
/// first frame at `start_ms`.
pub fn gesture_frames(spec: &GestureSpec, start_ms: u64) -> Vec<TouchFrame> {
    match spec.name {
        GestureName::Tap => tap(spec, start_ms, 1),
        GestureName::DoubleTap => tap(spec, start_ms, 2),
        GestureName::LongPress => long_press(spec, start_ms),
        GestureName::SwipeLeft => swipe(spec, start_ms, -1, 0),
        GestureName::SwipeRight => swipe(spec, start_ms, 1, 0),
        GestureName::SwipeUp => swipe(spec, start_ms, 0, -1),
        GestureName::SwipeDown => swipe(spec, start_ms, 0, 1),
        GestureName::PinchIn => pinch(spec, start_ms, false),
        GestureName::PinchOut => pinch(spec, start_ms, true),
        GestureName::RotateCw => rotate(spec, start_ms, true),
        GestureName::RotateCcw => rotate(spec, start_ms, false),
        GestureName::TwoFingerTap => two_finger_tap(spec, start_ms),
        GestureName::DrawS => draw_curve(spec, start_ms, 0),
        GestureName::CircleCw => draw_curve(spec, start_ms, 1),
        GestureName::CircleCcw => draw_curve(spec, start_ms, -1),
        GestureName::ThreeFingerSwipeLeft => three_finger_swipe(spec, start_ms, -1, 0),
        GestureName::ThreeFingerSwipeRight => three_finger_swipe(spec, start_ms, 1, 0),
        GestureName::ThreeFingerSwipeUp => three_finger_swipe(spec, start_ms, 0, -1),
        GestureName::ThreeFingerSwipeDown => three_finger_swipe(spec, start_ms, 0, 1),
    }
}

fn draw_curve(spec: &GestureSpec, start_ms: u64, direction: i32) -> Vec<TouchFrame> {
    let steps = (clamped_ms(spec.duration_ms) / STEP_MS).max(1);
    let radius = f64::from(spec.distance.max(20)) / 2.0;
    let mut frames = Vec::new();
    let mut landed = (spec.x, spec.y);
    for step in 0..=steps {
        let fraction = step as f64 / steps as f64;
        let angle = std::f64::consts::TAU * fraction;
        let (dx, dy) = if direction == 0 {
            (-radius * angle.sin(), radius * (2.0 * fraction - 1.0))
        } else {
            (
                radius * angle.cos(),
                radius * angle.sin() * f64::from(direction),
            )
        };
        landed = (
            spec.x.saturating_add(dx.round() as i32),
            spec.y.saturating_add(dy.round() as i32),
        );
        let phase = if step == 0 {
            TouchPhase::Down
        } else {
            TouchPhase::Move
        };
        frames.push(frame(
            start_ms.saturating_add(step * STEP_MS),
            &[(0, landed.0, landed.1, phase)],
        ));
    }
    frames.push(frame(
        start_ms.saturating_add((steps + 1) * STEP_MS),
        &[(0, landed.0, landed.1, TouchPhase::Up)],
    ));
    frames
}

fn frame(at_ms: u64, contacts: &[(u32, i32, i32, TouchPhase)]) -> TouchFrame {
    TouchFrame {
        at_ms,
        contacts: contacts
            .iter()
            .map(|&(id, x, y, phase)| TouchContact { id, x, y, phase })
            .collect(),
    }
}

fn clamped_ms(duration_ms: u32) -> u64 {
    u64::from(duration_ms).clamp(MIN_SWIPE_MS, MAX_SWIPE_MS)
}

fn tap(spec: &GestureSpec, start_ms: u64, count: u32) -> Vec<TouchFrame> {
    let mut frames = Vec::new();
    let mut at = start_ms;
    for _ in 0..count {
        frames.push(frame(at, &[(0, spec.x, spec.y, TouchPhase::Down)]));
        at += TAP_PRESS_MS;
        frames.push(frame(at, &[(0, spec.x, spec.y, TouchPhase::Up)]));
        at += DOUBLE_TAP_GAP_MS;
    }
    frames
}

fn long_press(spec: &GestureSpec, start_ms: u64) -> Vec<TouchFrame> {
    let hold = u64::from(spec.duration_ms).max(LONG_PRESS_MS);
    let mut frames = vec![frame(start_ms, &[(0, spec.x, spec.y, TouchPhase::Down)])];
    let steps = (hold / STEP_MS).max(1);
    for step in 1..steps {
        frames.push(frame(
            start_ms + step * STEP_MS,
            &[(0, spec.x, spec.y, TouchPhase::Move)],
        ));
    }
    frames.push(frame(
        start_ms + hold,
        &[(0, spec.x, spec.y, TouchPhase::Up)],
    ));
    frames
}

fn swipe(spec: &GestureSpec, start_ms: u64, dx: i32, dy: i32) -> Vec<TouchFrame> {
    let duration = clamped_ms(spec.duration_ms);
    let distance = f64::from(spec.distance.max(20));
    let steps = (duration / STEP_MS).max(1);
    let mut frames = vec![frame(start_ms, &[(0, spec.x, spec.y, TouchPhase::Down)])];
    let mut landed = (spec.x, spec.y);
    for step in 1..=steps {
        let fraction = step as f64 / steps as f64;
        let x = spec.x + (f64::from(dx) * distance * fraction).round() as i32;
        let y = spec.y + (f64::from(dy) * distance * fraction).round() as i32;
        landed = (x, y);
        frames.push(frame(
            start_ms + step * STEP_MS,
            &[(0, x, y, TouchPhase::Move)],
        ));
    }
    frames.push(frame(
        start_ms + (steps + 1) * STEP_MS,
        &[(0, landed.0, landed.1, TouchPhase::Up)],
    ));
    frames
}

fn pinch(spec: &GestureSpec, start_ms: u64, outward: bool) -> Vec<TouchFrame> {
    let duration = clamped_ms(spec.duration_ms);
    let far = f64::from(spec.distance.max(40)) / 2.0;
    let (start_offset, end_offset) = if outward { (16.0, far) } else { (far, 16.0) };
    let steps = (duration / STEP_MS).max(1);
    let mut frames = Vec::new();
    let mut landed = (spec.x, spec.x);
    for step in 0..=steps {
        let fraction = step as f64 / steps as f64;
        let offset = start_offset + (end_offset - start_offset) * fraction;
        let phase = if step == 0 {
            TouchPhase::Down
        } else {
            TouchPhase::Move
        };
        landed = (
            spec.x - offset.round() as i32,
            spec.x + offset.round() as i32,
        );
        frames.push(frame(
            start_ms + step * STEP_MS,
            &[(0, landed.0, spec.y, phase), (1, landed.1, spec.y, phase)],
        ));
    }
    frames.push(frame(
        start_ms + (steps + 1) * STEP_MS,
        &[
            (0, landed.0, spec.y, TouchPhase::Up),
            (1, landed.1, spec.y, TouchPhase::Up),
        ],
    ));
    frames
}

fn rotate(spec: &GestureSpec, start_ms: u64, clockwise: bool) -> Vec<TouchFrame> {
    let duration = clamped_ms(spec.duration_ms);
    let radius = f64::from(spec.distance.max(40)) / 2.0;
    let total = if clockwise {
        std::f64::consts::FRAC_PI_2
    } else {
        -std::f64::consts::FRAC_PI_2
    };
    let steps = (duration / STEP_MS).max(1);
    let mut frames = Vec::new();
    let mut landed = ((spec.x, spec.y), (spec.x, spec.y));
    for step in 0..=steps {
        let angle = total * (step as f64 / steps as f64);
        let phase = if step == 0 {
            TouchPhase::Down
        } else {
            TouchPhase::Move
        };
        let (sin, cos) = (angle.sin(), angle.cos());
        let first = (
            spec.x + (radius * cos).round() as i32,
            spec.y + (radius * sin).round() as i32,
        );
        let second = (
            spec.x - (radius * cos).round() as i32,
            spec.y - (radius * sin).round() as i32,
        );
        landed = (first, second);
        frames.push(frame(
            start_ms + step * STEP_MS,
            &[(0, first.0, first.1, phase), (1, second.0, second.1, phase)],
        ));
    }
    frames.push(frame(
        start_ms + (steps + 1) * STEP_MS,
        &[
            (0, landed.0.0, landed.0.1, TouchPhase::Up),
            (1, landed.1.0, landed.1.1, TouchPhase::Up),
        ],
    ));
    frames
}

fn two_finger_tap(spec: &GestureSpec, start_ms: u64) -> Vec<TouchFrame> {
    let contacts = [
        (0, spec.x - 40, spec.y, TouchPhase::Down),
        (1, spec.x + 40, spec.y, TouchPhase::Down),
    ];
    let lifted = [
        (0, spec.x - 40, spec.y, TouchPhase::Up),
        (1, spec.x + 40, spec.y, TouchPhase::Up),
    ];
    vec![
        frame(start_ms, &contacts),
        frame(start_ms + TAP_PRESS_MS, &lifted),
    ]
}

fn three_finger_swipe(spec: &GestureSpec, start_ms: u64, dx: i32, dy: i32) -> Vec<TouchFrame> {
    let duration = clamped_ms(spec.duration_ms);
    let distance = f64::from(spec.distance.max(20));
    let steps = (duration / STEP_MS).max(1);
    let offsets = [-60, 0, 60];
    let mut frames = Vec::new();
    let mut landed = (spec.x, spec.y);
    for step in 0..=steps {
        let fraction = step as f64 / steps as f64;
        let x = spec.x + (f64::from(dx) * distance * fraction).round() as i32;
        let y = spec.y + (f64::from(dy) * distance * fraction).round() as i32;
        landed = (x, y);
        let phase = if step == 0 {
            TouchPhase::Down
        } else {
            TouchPhase::Move
        };
        let mut contacts = Vec::with_capacity(3);
        for (id, offset) in offsets.iter().enumerate() {
            contacts.push((id as u32, x + offset, y, phase));
        }
        frames.push(frame(start_ms + step * STEP_MS, &contacts));
    }
    let lifted: Vec<(u32, i32, i32, TouchPhase)> = offsets
        .iter()
        .enumerate()
        .map(|(id, offset)| (id as u32, landed.0 + offset, landed.1, TouchPhase::Up))
        .collect();
    frames.push(frame(start_ms + (steps + 1) * STEP_MS, &lifted));
    frames
}

#[cfg(test)]
#[path = "gesture_tests.rs"]
mod tests;
