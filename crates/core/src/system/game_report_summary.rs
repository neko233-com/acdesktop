use serde::Serialize;

/// Timing and dimensions of a game capture.
#[derive(Debug, Clone, Serialize)]
pub struct GameReportSummary {
    pub duration_ms: u64,
    pub planned_ms: u64,
    pub truncated: bool,
    pub width: u32,
    pub height: u32,
    pub frames: usize,
    pub inputs: usize,
}
