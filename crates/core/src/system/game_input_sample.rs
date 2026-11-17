use serde::{Deserialize, Serialize};

/// One dispatched input: when the plan scheduled it, when it was injected,
/// its plan-event index, and its action label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSample {
    pub event: usize,
    pub scheduled_ms: u64,
    pub dispatched_ms: u64,
    pub kind: String,
}
