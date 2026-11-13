use super::game_distribution::Distribution;
use serde::Serialize;

/// How closely dispatched input matched its schedule.
#[derive(Debug, Clone, Serialize)]
pub struct InputMetrics {
    pub dispatched: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter_ms: Option<Distribution>,
}
