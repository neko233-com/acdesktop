use serde::{Deserialize, Serialize};

use super::game_frame_sample::FrameSample;
use super::game_input_sample::InputSample;

/// What a platform observed while playing a game session: the sampled frames
/// and the dispatched inputs, each with its own timestamp. Metrics are
/// computed from this log, never inside the adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSessionLog {
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub frames: Vec<FrameSample>,
    pub inputs: Vec<InputSample>,
}
