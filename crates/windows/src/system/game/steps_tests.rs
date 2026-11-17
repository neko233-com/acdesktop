use acdesktop_core::GameSessionPlan;

use super::{StepAction, build_steps, validate_touch_steps};

fn steps_for(json: &str) -> Vec<super::Step> {
    let plan = GameSessionPlan::parse(json).expect("plan parses");
    build_steps(&plan)
}

#[test]
fn a_key_hold_becomes_a_press_and_a_release() {
    let steps = steps_for(
        r#"{"duration_ms":1000,"events":[{"at_ms":100,"action":"key","key":"W","hold_ms":300}]}"#,
    );
    assert_eq!(steps.len(), 2);
    assert!(matches!(steps[0].action, StepAction::KeyDown { .. }));
    assert!(matches!(steps[1].action, StepAction::KeyUp { .. }));
    assert_eq!(steps[0].at_ms, 100);
    assert_eq!(steps[1].at_ms, 400);
}

#[test]
fn a_click_becomes_a_down_and_an_up() {
    let steps = steps_for(
        r#"{"duration_ms":1000,"events":[{"at_ms":50,"action":"mouse-click","x":10,"y":20}]}"#,
    );
    assert_eq!(steps.len(), 2);
    assert!(matches!(
        steps[0].action,
        StepAction::MouseButton { down: true, .. }
    ));
    assert!(matches!(
        steps[1].action,
        StepAction::MouseButton { down: false, .. }
    ));
    assert!(steps[1].at_ms > steps[0].at_ms);
}

#[test]
fn a_gesture_expands_to_frame_steps() {
    let steps = steps_for(
        r#"{"duration_ms":1000,"events":[{"at_ms":10,"action":"gesture","gesture":"tap","x":5,"y":6}]}"#,
    );
    assert_eq!(steps.len(), 2);
    assert!(
        steps
            .iter()
            .all(|step| matches!(step.action, StepAction::Touch(_)))
    );
    assert!(steps.iter().all(|step| step.at_ms >= 10));
}

#[test]
fn a_touch_hold_is_released() {
    let steps = steps_for(
        r#"{"duration_ms":1000,"events":[{"at_ms":0,"action":"touch","contacts":[{"id":0,"x":1,"y":2}],"hold_ms":100}]}"#,
    );
    assert_eq!(steps.len(), 2);
    let last = steps.last().expect("two steps");
    assert_eq!(last.at_ms, 100);
    match &last.action {
        StepAction::Touch(contacts) => {
            assert_eq!(contacts[0].phase, acdesktop_core::TouchPhase::Up)
        }
        _ => panic!("expected a release frame"),
    }
}

#[test]
fn steps_are_sorted_by_time() {
    let steps = steps_for(
        r#"{"duration_ms":1000,"events":[
            {"at_ms":300,"action":"key-up","key":"A"},
            {"at_ms":100,"action":"key-down","key":"A"}
        ]}"#,
    );
    assert_eq!(steps[0].at_ms, 100);
    assert_eq!(steps[1].at_ms, 300);
}

#[test]
fn path_keeps_two_fingers_held_then_releases_them() {
    let steps = steps_for(
        r#"{"duration_ms":1000,"events":[{"at_ms":100,"action":"touch-path","frames":[
        {"at_ms":0,"contacts":[{"id":0,"x":10,"y":10,"phase":"down"}]},
        {"at_ms":100,"contacts":[{"id":0,"x":20,"y":10,"phase":"move"},{"id":1,"x":50,"y":50,"phase":"down"}]},
        {"at_ms":200,"contacts":[{"id":0,"x":20,"y":10,"phase":"move"},{"id":1,"x":50,"y":50,"phase":"up"}]},
        {"at_ms":300,"contacts":[{"id":0,"x":20,"y":10,"phase":"up"}]}
    ]}]}"#,
    );
    assert_eq!(
        steps.iter().map(|step| step.at_ms).collect::<Vec<_>>(),
        vec![100, 200, 300, 400]
    );
    validate_touch_steps(&steps).expect("complete simultaneous touch path");
}

#[test]
fn omitted_or_unreleased_fingers_are_rejected() {
    let missing = steps_for(
        r#"{"duration_ms":1000,"events":[{"at_ms":0,"action":"touch-path","frames":[
        {"at_ms":0,"contacts":[{"id":0,"x":10,"y":10,"phase":"down"}]},
        {"at_ms":20,"contacts":[{"id":1,"x":50,"y":50,"phase":"down"}]}
    ]}]}"#,
    );
    assert!(validate_touch_steps(&missing).is_err());
    assert!(validate_touch_steps(&missing[..1]).is_err());
}

#[test]
fn lifting_a_finger_at_a_new_position_is_rejected() {
    let steps = steps_for(
        r#"{"duration_ms":1000,"events":[{"at_ms":0,"action":"touch-path","frames":[
        {"at_ms":0,"contacts":[{"id":0,"x":10,"y":10,"phase":"down"}]},
        {"at_ms":20,"contacts":[{"id":0,"x":11,"y":10,"phase":"up"}]}
    ]}]}"#,
    );
    assert!(validate_touch_steps(&steps).is_err());
}
