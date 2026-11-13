use serde::{Deserialize, Serialize};

use super::game_event::GameEvent;
use super::game_probe::GameProbe;

/// A timed input timeline observed against a window's pixels.
///
/// The plan is the whole session: which input to inject at which millisecond,
/// which frames to sample, and which input-to-picture reactions to probe. A
/// platform plays the timeline and returns timestamps; core turns the
/// timestamps into metrics, so the measurement stays identical across
/// platforms.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameSessionPlan {
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_duration_ms")]
    pub duration_ms: u64,
    #[serde(default)]
    pub events: Vec<GameEvent>,
    #[serde(default)]
    pub probes: Vec<GameProbe>,
    #[serde(default)]
    pub capture_cursor: bool,
    #[serde(default)]
    pub acknowledge_system_gesture_conflict: bool,
    #[serde(default)]
    pub brief_response: bool,
}

fn default_fps() -> u32 {
    30
}

fn default_duration_ms() -> u64 {
    5_000
}

impl GameSessionPlan {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !(1..=240).contains(&self.fps) || self.duration_ms == 0 || self.duration_ms > 3_600_000 {
            return Err("fps must be 1-240 and duration_ms must be 1-3600000");
        }
        if self.events.len() > 10_000 || self.probes.len() > 10_000 {
            return Err("a plan may contain at most 10000 events and probes");
        }
        let mut steps = 0usize;
        for event in &self.events {
            if !self.acknowledge_system_gesture_conflict && may_trigger_system_gestures(event) {
                return Err(
                    "three or more touch contacts may trigger system gestures; disable the Windows three- and four-finger touch setting and set acknowledge_system_gesture_conflict to true after reviewing that risk",
                );
            }
            if event.at_ms() >= self.duration_ms {
                return Err("event time exceeds the session duration");
            }
            let additional = match event {
                GameEvent::Gesture { duration_ms, .. } => {
                    (*duration_ms as usize / 16).max(40).saturating_add(4)
                }
                GameEvent::TouchPath { frames, .. } => frames.len(),
                _ => 2,
            };
            steps = steps.saturating_add(additional);
            if steps > 100_000 {
                return Err("plan expands to too many input steps");
            }
            if !valid_coordinates(event) {
                return Err("input coordinates exceed the supported range");
            }
            match event {
                GameEvent::Gesture {
                    duration_ms,
                    distance,
                    ..
                } if *duration_ms > 60_000 || *distance > 100_000 => {
                    return Err("gesture duration or distance is too large");
                }
                GameEvent::Key { hold_ms, .. } | GameEvent::Touch { hold_ms, .. }
                    if *hold_ms > 60_000 =>
                {
                    return Err("input hold exceeds 60000 ms");
                }
                GameEvent::TouchPath { at_ms, frames } => {
                    if frames.is_empty() || frames.len() > 10_000 {
                        return Err("touch-path requires 1-10000 frames");
                    }
                    let mut previous = None;
                    for frame in frames {
                        if frame.contacts.is_empty()
                            || frame.contacts.len() > 10
                            || previous.is_some_and(|time| frame.at_ms <= time)
                            || at_ms
                                .checked_add(frame.at_ms)
                                .is_none_or(|time| time >= self.duration_ms)
                        {
                            return Err(
                                "touch-path frames must have 1-10 contacts and strictly increasing times within the session",
                            );
                        }
                        previous = Some(frame.at_ms);
                    }
                }
                _ => {}
            }
        }
        if self.probes.iter().any(|probe| {
            probe.event >= self.events.len()
                || !probe.min_change.is_finite()
                || !(0.0..=1.0).contains(&probe.min_change)
        }) {
            return Err("probe event index or change threshold is invalid");
        }
        Ok(())
    }
}

fn may_trigger_system_gestures(event: &GameEvent) -> bool {
    match event {
        GameEvent::Gesture { gesture, .. } => gesture.needs_three_fingers(),
        GameEvent::Touch { contacts, .. } => contacts.len() >= 3,
        GameEvent::TouchPath { frames, .. } => frames.iter().any(|frame| frame.contacts.len() >= 3),
        _ => false,
    }
}

fn valid_coordinates(event: &GameEvent) -> bool {
    let valid = |x: i32, y: i32| x.abs_diff(0) <= 1_000_000 && y.abs_diff(0) <= 1_000_000;
    match event {
        GameEvent::MouseMove { x, y, .. }
        | GameEvent::MouseDown { x, y, .. }
        | GameEvent::MouseUp { x, y, .. }
        | GameEvent::MouseClick { x, y, .. }
        | GameEvent::Gesture { x, y, .. } => valid(*x, *y),
        GameEvent::Touch { contacts, .. } => {
            contacts.iter().all(|contact| valid(contact.x, contact.y))
        }
        GameEvent::TouchPath { frames, .. } => frames
            .iter()
            .flat_map(|frame| &frame.contacts)
            .all(|contact| valid(contact.x, contact.y)),
        _ => true,
    }
}

#[cfg(test)]
#[path = "game_plan_tests.rs"]
mod tests;
