use serde::Serialize;

/// Visual activity and response markers from the captured frames.
#[derive(Debug, Clone, Serialize)]
pub struct FlowMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_ratio: Option<f64>,
    pub change_events: usize,
    pub longest_still_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rhythm_regularity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsiveness: Option<f64>,
}
