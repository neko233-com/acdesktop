use super::shows_pointer;
use crate::Action;

struct BoundsAdapter(Option<crate::Rect>);

impl crate::ObservationOps for BoundsAdapter {
    fn get_element_bounds(
        &self,
        _: &crate::NativeHandle,
        _: crate::Deadline,
    ) -> Result<Option<crate::Rect>, crate::AdapterError> {
        Ok(self.0)
    }
}
impl crate::ActionOps for BoundsAdapter {}
impl crate::InputOps for BoundsAdapter {}
impl crate::SystemOps for BoundsAdapter {}

#[derive(Default)]
struct ClickEventAdapter(std::sync::Mutex<Vec<crate::Point>>);

impl crate::ObservationOps for ClickEventAdapter {}
impl crate::ActionOps for ClickEventAdapter {}
impl crate::InputOps for ClickEventAdapter {}
impl crate::SystemOps for ClickEventAdapter {
    fn present_control_indicator(
        &self,
        event: &crate::ControlIndicatorEvent<'_>,
    ) -> Result<(), crate::AdapterError> {
        if let crate::ControlIndicatorEvent::Click { point, .. } = event {
            self.0.lock().unwrap().push((*point).clone());
        }
        Ok(())
    }
}

#[test]
fn click_indicator_uses_verified_input_point_instead_of_old_cursor_position() {
    let adapter = ClickEventAdapter::default();
    let context =
        crate::CommandContext::default().with_control_indicator_owner(Some("test".into()));
    let preflight = super::ActionabilityPreflight {
        verified_point: Some(crate::Point { x: -4.0, y: 5.0 }),
        presentation_point: Some(crate::Point { x: 8.0, y: 9.0 }),
        presentation_bounds: None,
        pointer_delivery: crate::actionability::PointerDelivery::Physical,
    };
    super::TRAVELLED_BEFORE_LEASE.set(Some((100.0, 100.0)));
    super::after_dispatch(
        &adapter,
        &context,
        &preflight,
        true,
        &Ok(crate::ActionResult::delivered_unverified("click")),
    );
    assert_eq!(
        *adapter.0.lock().unwrap(),
        [crate::Point { x: -4.0, y: 5.0 }]
    );
}

#[test]
fn semantic_click_indicator_uses_live_screen_bounds_only_when_available() {
    let bounds = crate::Rect {
        x: -100.0,
        y: 30.0,
        width: 40.0,
        height: 20.0,
    };
    let handle = crate::NativeHandle::null();
    let deadline = crate::Deadline::standard().unwrap();
    assert_eq!(
        super::live_indicator_point(&BoundsAdapter(Some(bounds)), &handle, deadline),
        Some(crate::Point { x: -80.0, y: 40.0 })
    );
    assert_eq!(
        super::live_indicator_point(&BoundsAdapter(None), &handle, deadline),
        None
    );
}

#[test]
fn semantic_click_refreshes_the_indicator_point_only_after_delivery() {
    let bounds = crate::Rect {
        x: -100.0,
        y: 30.0,
        width: 40.0,
        height: 20.0,
    };
    let adapter = BoundsAdapter(Some(bounds));
    let context =
        crate::CommandContext::default().with_control_indicator_owner(Some("test".into()));
    let entry = crate::adapter::minimal_ref_entry("button", Some("Run"));
    let deadline = crate::Deadline::standard().unwrap();
    let wait = crate::ref_action::ref_action_wait_context::RefActionWaitContext {
        adapter: &adapter,
        entry: &entry,
        ref_id: "test",
        context: &context,
    };
    let action_context =
        crate::ref_action::ref_action_context::RefActionContext::new(wait, deadline);
    let handle = crate::NativeHandle::null();
    let target = super::ResolvedRefAction::new(action_context, &handle);
    let mut preflight = super::ActionabilityPreflight {
        verified_point: None,
        presentation_point: None,
        presentation_bounds: None,
        pointer_delivery: crate::actionability::PointerDelivery::Semantic,
    };
    super::refresh_point(
        &target,
        &mut preflight,
        true,
        &Err(crate::AdapterError::internal("blocked")),
    );
    assert_eq!(preflight.presentation_point, None);
    super::refresh_point(
        &target,
        &mut preflight,
        true,
        &Ok(crate::ActionResult::delivered_unverified("click")),
    );
    assert_eq!(
        preflight.presentation_point,
        Some(crate::Point { x: -80.0, y: 40.0 })
    );
}

#[test]
fn pressable_controls_show_the_pointer_however_they_are_reached() {
    for role in [
        "button",
        "link",
        "checkbox",
        "tab",
        "menuitem",
        "switch",
        "disclosure",
    ] {
        for action in [
            Action::Click,
            Action::ScrollTo,
            Action::SetFocus,
            Action::Toggle,
        ] {
            assert!(shows_pointer(role, &action), "{role} {action:?}");
        }
    }
}

#[test]
fn text_entry_keeps_the_arrow_even_when_clicked() {
    for role in ["textfield", "combobox", "datefield"] {
        for action in [Action::Click, Action::DoubleClick, Action::SetFocus] {
            assert!(!shows_pointer(role, &action), "{role} {action:?}");
        }
    }
}

#[test]
fn other_elements_show_the_pointer_only_when_clicked() {
    for role in ["group", "cell", "statictext", "unknown", "row"] {
        assert!(shows_pointer(role, &Action::Click), "{role}");
        assert!(shows_pointer(role, &Action::RightClick), "{role}");
        assert!(!shows_pointer(role, &Action::ScrollTo), "{role}");
        assert!(
            !shows_pointer(role, &Action::SetValue("x".into())),
            "{role}"
        );
    }
}

#[path = "presentation_lease_tests.rs"]
mod lease;

#[path = "presentation_shared_cursor_tests.rs"]
mod shared_cursor;

#[path = "presentation_caret_tests.rs"]
mod caret;
