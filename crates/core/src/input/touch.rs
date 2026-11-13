use serde::{Deserialize, Serialize};

/// One phase of a synthesized touch contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TouchPhase {
    Down,
    Move,
    Up,
}

/// One finger contact in virtual-screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchContact {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    #[serde(default = "resting_phase")]
    pub phase: TouchPhase,
}

fn resting_phase() -> TouchPhase {
    TouchPhase::Move
}

/// The set of contacts injected at one instant of a touch sequence.
///
/// A frame carries every contact currently in contact, plus any contact that
/// lifts in this frame with [`TouchPhase::Up`] exactly once. The injection
/// seam sends the frame as one batch, so a lifted contact must not appear in
/// a later frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchFrame {
    pub at_ms: u64,
    pub contacts: Vec<TouchContact>,
}

impl TouchFrame {
    pub fn is_release_only(&self) -> bool {
        !self.contacts.is_empty()
            && self
                .contacts
                .iter()
                .all(|contact| contact.phase == TouchPhase::Up)
    }
}
