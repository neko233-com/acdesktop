use super::*;
use crate::system::game_frame_sample::FrameSample;
use crate::system::game_input_sample::InputSample;

fn log(frames: Vec<(u64, f64)>, inputs: Vec<(usize, u64, u64)>) -> GameSessionLog {
    GameSessionLog {
        duration_ms: frames
            .last()
            .map(|frame| frame.0)
            .unwrap_or(0)
            .max(inputs.last().map(|input| input.2).unwrap_or(0)),
        width: 100,
        height: 100,
        frames: frames
            .into_iter()
            .map(|(at_ms, changed_ratio)| FrameSample {
                at_ms,
                changed_ratio,
            })
            .collect(),
        inputs: inputs
            .into_iter()
            .map(|(event, scheduled_ms, dispatched_ms)| InputSample {
                event,
                scheduled_ms,
                dispatched_ms,
                kind: "key".to_owned(),
            })
            .collect(),
    }
}

fn plan(duration_ms: u64) -> GameSessionPlan {
    GameSessionPlan {
        fps: 30,
        duration_ms,
        events: Vec::new(),
        capture_cursor: false,
        acknowledge_system_gesture_conflict: false,
        brief_response: false,
        probes: vec![crate::system::game_probe::GameProbe {
            name: "jump".to_owned(),
            event: 0,
            timeout_ms: 200,
            min_change: 0.01,
        }],
    }
}

#[test]
fn a_probe_reports_the_first_reacting_frame() {
    let log = log(
        vec![(0, 0.0), (0, 0.0), (33, 0.0), (66, 0.5)],
        vec![(0, 0, 0)],
    );
    let report = GameReport::from_log(&log, &plan(100));
    assert_eq!(report.probes.len(), 1);
    assert!(report.probes[0].responded);
    assert_eq!(report.probes[0].latency_ms, Some(66));
}

#[test]
fn a_silent_session_is_idle() {
    let log = log(vec![(0, 0.0), (33, 0.0), (66, 0.0)], Vec::new());
    let report = GameReport::from_log(&log, &plan(100));
    assert_eq!(report.flow.change_events, 0);
    assert_eq!(report.flow.active_ratio, Some(0.0));
}

#[test]
fn a_regular_session_scores_high() {
    let frames: Vec<(u64, f64)> = (0..30)
        .map(|index| (index * 33, if index % 3 == 0 { 0.2 } else { 0.0 }))
        .collect();
    let inputs: Vec<(usize, u64, u64)> = (0..10usize)
        .map(|index| (index, index as u64 * 100, index as u64 * 100))
        .collect();
    let report = GameReport::from_log(&log(frames, inputs), &plan(1000));
    assert!(report.flow.active_ratio.expect("activity was sampled") > 0.2);
    assert!(report.flow.change_events >= 2);
    assert!(report.flow.rhythm_regularity.is_some());
}

#[test]
fn a_stalled_frame_stream_marks_dropped_frames() {
    let frames = vec![(0, 0.0), (33, 0.0), (66, 0.0), (500, 0.0), (533, 0.0)];
    let report = GameReport::from_log(&log(frames, Vec::new()), &plan(1000));
    assert_eq!(report.frame_metrics.dropped, 1);
    assert!(report.frame_metrics.interval_ms.is_some());
}

#[test]
fn a_session_that_stopped_early_is_marked_truncated() {
    let frames = vec![(0, 0.0), (33, 0.0), (66, 0.0)];
    let report = GameReport::from_log(&log(frames, Vec::new()), &plan(7_000));
    assert!(report.summary.truncated);
    assert_eq!(report.summary.duration_ms, 66);
    assert_eq!(report.summary.planned_ms, 7_000);
}

#[test]
fn even_one_millisecond_of_missing_session_time_is_truncated() {
    let mut observed = log(Vec::new(), Vec::new());
    observed.duration_ms = 999;
    let report = GameReport::from_log(&observed, &plan(1_000));
    assert!(report.summary.truncated);
}

#[test]
fn a_session_that_ran_to_its_plan_is_not_truncated() {
    let frames = vec![(0, 0.0), (1_000, 0.0), (2_000, 0.0)];
    let report = GameReport::from_log(&log(frames, Vec::new()), &plan(2_000));
    assert!(!report.summary.truncated);
    assert_eq!(report.summary.duration_ms, 2_000);
    let json = serde_json::to_value(&report).expect("report serializes");
    assert_eq!(json["duration_ms"], 2_000);
    assert_eq!(json["planned_ms"], 2_000);
    assert_eq!(json["cursor_in_saved_frames"], false);
    assert!(json.get("summary").is_none());
}

#[test]
fn longest_still_interval_includes_earlier_runs() {
    let log = log(
        vec![(0, 0.0), (100, 0.0), (200, 0.2), (230, 0.0), (240, 0.0)],
        Vec::new(),
    );
    let report = GameReport::from_log(&log, &plan(240));
    assert_eq!(report.flow.longest_still_ms, 200);
}

#[test]
fn unavailable_metrics_and_empty_probes_are_omitted() {
    let mut empty_plan = plan(100);
    empty_plan.probes.clear();
    let report = GameReport::from_log(&log(Vec::new(), Vec::new()), &empty_plan);
    let json = serde_json::to_value(&report).expect("report serializes");
    assert!(json.get("probes").is_none());
    assert!(json["frame_metrics"].get("fps").is_none());
    assert!(json["frame_metrics"].get("interval_ms").is_none());
    assert!(json["input_metrics"].get("jitter_ms").is_none());
    assert!(json["flow"].get("active_ratio").is_none());
    assert!(json["flow"].get("rhythm_regularity").is_none());
    assert!(json["flow"].get("responsiveness").is_none());
}

#[test]
fn an_unanswered_probe_omits_only_its_missing_latency() {
    let report = GameReport::from_log(&log(vec![(0, 0.0), (33, 0.0)], vec![(0, 0, 0)]), &plan(100));
    let json = serde_json::to_value(&report).expect("report serializes");
    assert_eq!(json["probes"][0]["responded"], false);
    assert!(json["probes"][0].get("latency_ms").is_none());
    assert_eq!(json["flow"]["active_ratio"], 0.0);
    assert_eq!(json["flow"]["responsiveness"], 0.0);
}

#[test]
fn a_single_baseline_frame_is_not_evidence_of_activity_or_response() {
    let mut probe_plan = plan(100);
    probe_plan.probes[0].min_change = 0.0;
    let report = GameReport::from_log(&log(vec![(0, 0.0)], vec![(0, 0, 0)]), &probe_plan);
    let json = serde_json::to_value(&report).expect("report serializes");
    assert!(json["flow"].get("active_ratio").is_none());
    assert_eq!(json["probes"][0]["responded"], false);
    assert!(json["probes"][0].get("latency_ms").is_none());
}

#[test]
fn an_observed_zero_millisecond_latency_is_retained() {
    let report = GameReport::from_log(&log(vec![(0, 0.0), (0, 0.1)], vec![(0, 0, 0)]), &plan(100));
    let json = serde_json::to_value(&report).expect("report serializes");
    assert_eq!(json["probes"][0]["responded"], true);
    assert_eq!(json["probes"][0]["latency_ms"], 0);
}
