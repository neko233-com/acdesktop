use super::validate;
use crate::system::game::steps::build_steps;
use acdesktop_core::GameSessionPlan;

#[test]
fn blocked_shortcuts_are_refused_in_both_key_orders() {
    for (first, second) in [("alt", "tab"), ("tab", "alt")] {
        let json = format!(
            r#"{{"duration_ms":1000,"events":[{{"at_ms":0,"action":"key-down","key":"{first}"}},{{"at_ms":10,"action":"key-down","key":"{second}"}}]}}"#
        );
        let plan = GameSessionPlan::parse(&json).expect("plan parses");
        assert!(validate(&build_steps(&plan)).is_err());
    }
}

#[test]
fn ordinary_uppercase_movement_keys_resolve() {
    let plan = GameSessionPlan::parse(
        r#"{"duration_ms":1000,"events":[{"at_ms":10,"action":"key","key":"W","hold_ms":100}]}"#,
    )
    .expect("plan parses");
    validate(&build_steps(&plan)).expect("movement key is allowed");
}

#[test]
fn overlapping_holds_and_unowned_releases_are_rejected() {
    for events in [
        r#"[{"at_ms":0,"action":"key-down","key":"a"},{"at_ms":10,"action":"key-down","key":"A"}]"#,
        r#"[{"at_ms":0,"action":"key-up","key":"a"}]"#,
    ] {
        let plan = GameSessionPlan::parse(&format!(r#"{{"duration_ms":1000,"events":{events}}}"#))
            .expect("plan parses");
        assert!(validate(&build_steps(&plan)).is_err());
    }
}

#[test]
fn mouse_button_release_requires_a_prior_press() {
    let plan = GameSessionPlan::parse(
        r#"{"duration_ms":1000,"events":[{"at_ms":0,"action":"mouse-up","x":20,"y":30}]}"#,
    )
    .expect("plan parses");
    assert!(validate(&build_steps(&plan)).is_err());
}
