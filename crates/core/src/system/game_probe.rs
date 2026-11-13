use serde::{Deserialize, Serialize};

/// An input-to-picture reaction the session measures.
///
/// `event` is the index of the plan event whose dispatch starts the clock;
/// the probe responds when a sampled frame within `timeout_ms` changes by at
/// least `min_change` of its sampled pixels.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameProbe {
    pub name: String,
    pub event: usize,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_min_change")]
    pub min_change: f64,
}

fn default_timeout_ms() -> u64 {
    1_000
}

fn default_min_change() -> f64 {
    0.002
}
