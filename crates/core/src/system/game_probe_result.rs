use serde::Serialize;

/// Whether one planned input was followed by a visible change in time.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
    pub name: String,
    pub event: usize,
    pub timeout_ms: u64,
    pub responded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}
