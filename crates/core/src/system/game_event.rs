use serde::{Deserialize, Serialize};

use crate::input::gesture::GestureName;
use crate::input::touch::{TouchContact, TouchFrame};

/// One entry of a [`super::game_plan::GameSessionPlan`].
///
/// The tag is `action`, so a plan reads as a list of
/// `{"at_ms": 250, "action": "key", "key": "W", "hold_ms": 400}` records.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum GameEvent {
    Key {
        at_ms: u64,
        key: String,
        #[serde(default)]
        hold_ms: u64,
    },
    KeyDown {
        at_ms: u64,
        key: String,
    },
    KeyUp {
        at_ms: u64,
        key: String,
    },
    MouseMove {
        at_ms: u64,
        x: i32,
        y: i32,
    },
    MouseDown {
        at_ms: u64,
        x: i32,
        y: i32,
        #[serde(default = "left_button")]
        button: String,
    },
    MouseUp {
        at_ms: u64,
        x: i32,
        y: i32,
        #[serde(default = "left_button")]
        button: String,
    },
    MouseClick {
        at_ms: u64,
        x: i32,
        y: i32,
        #[serde(default = "left_button")]
        button: String,
    },
    Gesture {
        at_ms: u64,
        gesture: GestureName,
        x: i32,
        y: i32,
        #[serde(default = "default_duration_ms")]
        duration_ms: u32,
        #[serde(default = "default_distance")]
        distance: u32,
    },
    Touch {
        at_ms: u64,
        contacts: Vec<TouchContact>,
        #[serde(default)]
        hold_ms: u64,
    },
    TouchPath {
        at_ms: u64,
        frames: Vec<TouchFrame>,
    },
}

fn left_button() -> String {
    "left".to_owned()
}

fn default_duration_ms() -> u32 {
    160
}

fn default_distance() -> u32 {
    160
}

impl GameEvent {
    pub fn at_ms(&self) -> u64 {
        match self {
            Self::Key { at_ms, .. }
            | Self::KeyDown { at_ms, .. }
            | Self::KeyUp { at_ms, .. }
            | Self::MouseMove { at_ms, .. }
            | Self::MouseDown { at_ms, .. }
            | Self::MouseUp { at_ms, .. }
            | Self::MouseClick { at_ms, .. }
            | Self::Gesture { at_ms, .. }
            | Self::Touch { at_ms, .. }
            | Self::TouchPath { at_ms, .. } => *at_ms,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Key { .. } => "key",
            Self::KeyDown { .. } => "key-down",
            Self::KeyUp { .. } => "key-up",
            Self::MouseMove { .. } => "mouse-move",
            Self::MouseDown { .. } => "mouse-down",
            Self::MouseUp { .. } => "mouse-up",
            Self::MouseClick { .. } => "mouse-click",
            Self::Gesture { .. } => "gesture",
            Self::Touch { .. } => "touch",
            Self::TouchPath { .. } => "touch-path",
        }
    }
}
