use serde::{Deserialize, Serialize};

/// One observed frame: when it was sampled and how much it changed from the
/// frame before it, as a ratio of sampled pixels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSample {
    pub at_ms: u64,
    pub changed_ratio: f64,
}
