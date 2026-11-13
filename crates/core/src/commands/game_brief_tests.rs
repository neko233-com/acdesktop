use super::tests::{RecordingAdapter, plan_file, scratch_dir, window};
use super::{GameArgs, execute};
use std::sync::atomic::AtomicBool;

#[test]
fn brief_response_keeps_full_probe_details_only_in_the_report_file() {
    let dir = scratch_dir();
    let probes: Vec<_> = (0..200)
        .map(|index| serde_json::json!({"name": format!("probe-{index}"), "event": 0}))
        .collect();
    let body = serde_json::json!({
        "duration_ms": 1000,
        "brief_response": true,
        "events": [{"at_ms": 10, "action": "key", "key": "space"}],
        "probes": probes,
    });
    let plan = plan_file(&dir, &body.to_string());
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let value = execute(
        args,
        &adapter,
        &crate::context::CommandContext::default().with_headed(true),
    )
    .expect("a brief session");
    assert!(value["summary"].get("probes").is_none());
    assert_eq!(value["summary"]["probe_summary"]["total"], 200);
    assert_eq!(value["summary"]["probe_summary"]["responded"], 0);
    let file: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("out/report.json")).expect("report"))
            .expect("report JSON");
    assert_eq!(file["probes"].as_array().expect("full probes").len(), 200);
    assert!(serde_json::to_vec(&value).expect("response").len() < 2_000);
    assert!(serde_json::to_vec(&file).expect("report").len() > 10_000);
}

#[test]
fn default_response_retains_per_probe_details() {
    let dir = scratch_dir();
    let body = r#"{"duration_ms":1000,"events":[{"at_ms":10,"action":"key","key":"space"}],"probes":[{"name":"jump","event":0}]}"#;
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan_file(&dir, body),
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let value = execute(
        args,
        &adapter,
        &crate::context::CommandContext::default().with_headed(true),
    )
    .expect("a default session");
    assert_eq!(value["summary"]["probes"][0]["name"], "jump");
    assert!(value["summary"].get("probe_summary").is_none());
}

#[test]
fn brief_response_without_probes_reports_zero_counts() {
    let dir = scratch_dir();
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan_file(&dir, r#"{"duration_ms":1000,"brief_response":true}"#),
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let value = execute(args, &adapter, &crate::context::CommandContext::default())
        .expect("a brief observation session");
    assert_eq!(value["summary"]["probe_summary"]["total"], 0);
    assert_eq!(value["summary"]["probe_summary"]["responded"], 0);
    assert!(value["summary"].get("probes").is_none());
}
