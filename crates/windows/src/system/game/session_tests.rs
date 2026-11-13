use super::*;
use crate::system::game::steps::StepAction;

fn step(at_ms: u64, event: usize) -> Step {
    Step {
        at_ms,
        event,
        label: "mouse-move",
        action: StepAction::MouseMove { x: 10, y: 20 },
    }
}

#[test]
fn a_session_cannot_succeed_with_undispatched_steps() {
    let steps = vec![step(10, 0), step(999, 1)];
    let error = ensure_all_steps_dispatched(&steps, 1).expect_err("the last step is missing");
    assert_eq!(error.code, acdesktop_core::ErrorCode::ActionFailed);
    assert_eq!(
        error.disposition,
        acdesktop_core::DeliverySemantics::unknown()
    );
    let details = error.details.expect("pending step details");
    assert_eq!(details["completed_steps"], 1);
    assert_eq!(details["planned_steps"], 2);
    assert_eq!(details["first_pending_at_ms"], 999);
    assert_eq!(details["first_pending_event"], 1);
    assert!(ensure_all_steps_dispatched(&steps, 2).is_ok());
}

#[test]
fn a_session_with_no_dispatched_steps_reports_not_delivered() {
    let error = ensure_all_steps_dispatched(&[step(10, 0)], 0).expect_err("nothing was sent");
    assert_eq!(
        error.disposition,
        acdesktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
fn a_failed_step_cannot_make_prior_input_safe_to_retry() {
    let failure = AdapterError::new(acdesktop_core::ErrorCode::ActionFailed, "touch failed")
        .with_details(serde_json::json!({"native_error": 5}))
        .with_disposition(acdesktop_core::DeliverySemantics::not_delivered());
    let after_input = account_for_prior_steps(failure.clone(), 1);
    assert_eq!(
        after_input.disposition,
        acdesktop_core::DeliverySemantics::unknown()
    );
    assert_eq!(
        after_input.details.expect("native details")["native_error"],
        5
    );
    assert_eq!(
        account_for_prior_steps(failure, 0).disposition,
        acdesktop_core::DeliverySemantics::not_delivered()
    );
}
