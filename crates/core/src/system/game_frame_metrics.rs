use super::game_distribution::Distribution;
use serde::Serialize;

/// How regularly the captured frames arrived.
#[derive(Debug, Clone, Serialize)]
pub struct FrameMetrics {
    pub sampled: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<Distribution>,
    pub dropped: usize,
}
