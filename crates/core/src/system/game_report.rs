use serde::Serialize;

use super::game_distribution::Distribution;
use super::game_flow_metrics::FlowMetrics;
use super::game_frame_metrics::FrameMetrics;
use super::game_input_metrics::InputMetrics;
use super::game_plan::GameSessionPlan;
use super::game_probe_result::ProbeResult;
use super::game_report_summary::GameReportSummary;
use super::game_session_log::GameSessionLog;

const ACTIVITY_THRESHOLD: f64 = 0.002;
const DROP_FACTOR: f64 = 2.0;

/// The structured result a `game` run returns and writes to disk.
#[derive(Debug, Clone, Serialize)]
pub struct GameReport {
    #[serde(flatten)]
    pub summary: GameReportSummary,
    pub cursor_in_saved_frames: bool,
    pub frame_metrics: FrameMetrics,
    pub input_metrics: InputMetrics,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub probes: Vec<ProbeResult>,
    pub flow: FlowMetrics,
}

impl GameReport {
    pub fn from_log(log: &GameSessionLog, plan: &GameSessionPlan) -> Self {
        let observed = observed_ms(log);
        let intervals: Vec<u64> = log
            .frames
            .windows(2)
            .map(|pair| pair[1].at_ms.saturating_sub(pair[0].at_ms))
            .collect();
        let interval_ms = Distribution::from_u64(&intervals);
        let dropped = match &interval_ms {
            Some(distribution) => intervals
                .iter()
                .filter(|&&interval| (interval as f64) > distribution.median * DROP_FACTOR)
                .count(),
            None => 0,
        };
        let fps = match (log.frames.first(), log.frames.last()) {
            (Some(first), Some(last)) if last.at_ms > first.at_ms => Some(
                log.frames.len().saturating_sub(1) as f64 * 1_000.0
                    / (last.at_ms - first.at_ms) as f64,
            ),
            _ => None,
        };
        let frame_metrics = FrameMetrics {
            sampled: log.frames.len(),
            fps,
            interval_ms,
            dropped,
        };

        let jitter: Vec<u64> = log
            .inputs
            .iter()
            .map(|input| input.dispatched_ms.saturating_sub(input.scheduled_ms))
            .collect();
        let input_metrics = InputMetrics {
            dispatched: log.inputs.len(),
            jitter_ms: Distribution::from_u64(&jitter),
        };

        let probes: Vec<ProbeResult> = plan
            .probes
            .iter()
            .map(|probe| probe_result(log, probe))
            .collect();
        let flow = flow_metrics(log, &probes);
        let truncated = observed < plan.duration_ms;

        Self {
            summary: GameReportSummary {
                duration_ms: observed,
                planned_ms: plan.duration_ms,
                truncated,
                width: log.width,
                height: log.height,
                frames: log.frames.len(),
                inputs: log.inputs.len(),
            },
            cursor_in_saved_frames: plan.capture_cursor,
            frame_metrics,
            input_metrics,
            probes,
            flow,
        }
    }
}

fn observed_ms(log: &GameSessionLog) -> u64 {
    log.duration_ms
}

fn probe_result(log: &GameSessionLog, probe: &super::game_probe::GameProbe) -> ProbeResult {
    let dispatched = log
        .inputs
        .iter()
        .find(|input| input.event == probe.event)
        .map(|input| input.dispatched_ms);
    let Some(dispatched_ms) = dispatched else {
        return ProbeResult {
            name: probe.name.clone(),
            event: probe.event,
            timeout_ms: probe.timeout_ms,
            responded: false,
            latency_ms: None,
        };
    };
    let limit = dispatched_ms.saturating_add(probe.timeout_ms);
    let latency = log
        .frames
        .iter()
        .skip(1)
        .filter(|frame| frame.at_ms >= dispatched_ms && frame.at_ms <= limit)
        .find(|frame| frame.changed_ratio >= probe.min_change)
        .map(|frame| frame.at_ms.saturating_sub(dispatched_ms));
    ProbeResult {
        name: probe.name.clone(),
        event: probe.event,
        timeout_ms: probe.timeout_ms,
        responded: latency.is_some(),
        latency_ms: latency,
    }
}

fn flow_metrics(log: &GameSessionLog, probes: &[ProbeResult]) -> FlowMetrics {
    let total = log.frames.len().saturating_sub(1);
    let active = log
        .frames
        .iter()
        .skip(1)
        .filter(|frame| frame.changed_ratio >= ACTIVITY_THRESHOLD)
        .count();
    let active_ratio = (total > 0).then(|| active as f64 / total as f64);

    let mut change_events = 0;
    let mut longest_still_ms = 0;
    let mut still_since = None;
    let mut was_active = None;
    for frame in &log.frames {
        let is_active = frame.changed_ratio >= ACTIVITY_THRESHOLD;
        if was_active == Some(false) && is_active {
            change_events += 1;
        }
        if is_active {
            if let Some(since) = still_since.take() {
                longest_still_ms = longest_still_ms.max(frame.at_ms.saturating_sub(since));
            }
        } else {
            let since = *still_since.get_or_insert(frame.at_ms);
            longest_still_ms = longest_still_ms.max(frame.at_ms.saturating_sub(since));
        }
        was_active = Some(is_active);
    }

    let rhythm_regularity = rhythm_regularity(log);
    let responsiveness = if probes.is_empty() {
        None
    } else {
        let responded = probes.iter().filter(|probe| probe.responded).count();
        Some(responded as f64 / probes.len() as f64)
    };

    FlowMetrics {
        active_ratio,
        change_events,
        longest_still_ms,
        rhythm_regularity,
        responsiveness,
    }
}

fn rhythm_regularity(log: &GameSessionLog) -> Option<f64> {
    let mut beats = Vec::new();
    let mut previous_change: Option<u64> = None;
    let mut was_active = None;
    for frame in &log.frames {
        let is_active = frame.changed_ratio >= ACTIVITY_THRESHOLD;
        if was_active == Some(false) && is_active {
            if let Some(previous) = previous_change {
                beats.push(frame.at_ms.saturating_sub(previous));
            }
            previous_change = Some(frame.at_ms);
        }
        was_active = Some(is_active);
    }
    if beats.len() < 2 {
        return None;
    }
    let distribution = Distribution::from_u64(&beats)?;
    let mean = distribution.median.max(1.0);
    Some((1.0 - distribution.stdev / mean).clamp(0.0, 1.0))
}

#[cfg(test)]
#[path = "game_report_tests.rs"]
mod tests;
