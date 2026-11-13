use super::*;

#[test]
fn a_minimal_plan_uses_the_defaults() {
    let plan = GameSessionPlan::parse(r#"{"duration_ms": 1000}"#).expect("plan parses");
    assert_eq!(plan.fps, 30);
    assert_eq!(plan.duration_ms, 1_000);
    assert!(plan.events.is_empty());
    assert!(plan.probes.is_empty());
    assert!(!plan.capture_cursor);
    assert!(!plan.brief_response);
}

#[test]
fn cursor_capture_is_an_explicit_plan_option() {
    let plan = GameSessionPlan::parse(r#"{"duration_ms":1000,"capture_cursor":true}"#)
        .expect("plan parses");
    assert!(plan.capture_cursor);
}

#[test]
fn three_finger_input_requires_explicit_system_gesture_acknowledgement() {
    for event in [
        r#"{"at_ms":10,"action":"gesture","gesture":"three-finger-swipe-right","x":100,"y":100}"#,
        r#"{"at_ms":10,"action":"touch","contacts":[{"id":0,"x":10,"y":10},{"id":1,"x":20,"y":10},{"id":2,"x":30,"y":10}]}"#,
        r#"{"at_ms":10,"action":"touch-path","frames":[{"at_ms":0,"contacts":[{"id":0,"x":10,"y":10},{"id":1,"x":20,"y":10},{"id":2,"x":30,"y":10}]}]}"#,
    ] {
        let mut plan =
            GameSessionPlan::parse(&format!(r#"{{"duration_ms":1000,"events":[{event}]}}"#))
                .expect("plan parses");
        assert!(plan.validate().is_err());
        plan.acknowledge_system_gesture_conflict = true;
        assert!(plan.validate().is_ok());
    }
}

#[test]
fn a_plan_parses_events_and_probes() {
    let plan = GameSessionPlan::parse(
        r#"{
            "duration_ms": 2000,
            "fps": 60,
            "events": [
                {"at_ms": 100, "action": "key", "key": "W", "hold_ms": 200},
                {"at_ms": 400, "action": "mouse-move", "x": 640, "y": 360},
                {"at_ms": 500, "action": "gesture", "gesture": "tap", "x": 10, "y": 20}
            ],
            "probes": [{"name": "jump", "event": 0, "timeout_ms": 500}]
        }"#,
    )
    .expect("plan parses");
    assert_eq!(plan.fps, 60);
    assert_eq!(plan.events.len(), 3);
    assert_eq!(plan.events[0].at_ms(), 100);
    assert_eq!(plan.events[2].at_ms(), 500);
    assert_eq!(plan.probes.len(), 1);
    assert_eq!(plan.probes[0].event, 0);
    assert_eq!(plan.probes[0].min_change, 0.002);
}

#[test]
fn an_unknown_field_is_rejected() {
    let result = GameSessionPlan::parse(r#"{"duration_ms": 1, "typo": true}"#);
    assert!(result.is_err());
}

#[test]
fn oversized_or_unordered_paths_fail_before_playback() {
    let plan = GameSessionPlan::parse(
        r#"{"duration_ms":1000,"events":[{"at_ms":50,"action":"touch-path","frames":[
        {"at_ms":10,"contacts":[{"id":0,"x":1,"y":2}]},
        {"at_ms":10,"contacts":[{"id":0,"x":1,"y":2,"phase":"up"}]}
    ]}]}"#,
    )
    .expect("plan parses");
    assert!(plan.validate().is_err());
    let large = GameSessionPlan::parse(r#"{"duration_ms":1000,"events":[{"at_ms":0,"action":"gesture","gesture":"long-press","x":1,"y":2,"duration_ms":4294967295}]}"#).expect("plan parses");
    assert!(large.validate().is_err());
}
