use super::*;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Recorder {
    calls: Arc<Mutex<Vec<String>>>,
    fail: bool,
    fail_after_delivery: bool,
}

impl acdesktop_core::ObservationOps for Recorder {}
impl acdesktop_core::ActionOps for Recorder {}
impl acdesktop_core::InputOps for Recorder {
    fn mouse_event(
        &self,
        event: acdesktop_core::MouseEvent,
        _: &acdesktop_core::InteractionLease,
    ) -> Result<acdesktop_core::OperationReport, AdapterError> {
        if self.fail_after_delivery {
            return Err(AdapterError::new(
                acdesktop_core::ErrorCode::ActionFailed,
                "postcondition failed",
            )
            .with_disposition(DeliverySemantics::delivered_unverified()));
        }
        Ok(acdesktop_core::OperationReport::delivered_unverified(
            "mouse",
            acdesktop_core::OperationTarget::Pointer { point: event.point },
        ))
    }
}
impl acdesktop_core::SystemOps for Recorder {
    fn requires_control_indicator(&self) -> bool {
        true
    }

    fn acquire_interaction_lease(
        &self,
        deadline: acdesktop_core::Deadline,
    ) -> Result<acdesktop_core::InteractionLease, AdapterError> {
        acdesktop_core::InteractionLease::guarded(deadline, ())
    }

    fn show_control_indicator(
        &self,
        _: &str,
        session: bool,
        description: &str,
    ) -> Result<(), AdapterError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("show:{session}:{description}"));
        if self.fail {
            Err(AdapterError::internal("unavailable"))
        } else {
            Ok(())
        }
    }

    fn hide_control_indicator(&self, _: &str) -> Result<(), AdapterError> {
        self.calls.lock().unwrap().push("hide".into());
        Ok(())
    }

    fn present_control_indicator(
        &self,
        event: &ControlIndicatorEvent<'_>,
    ) -> Result<(), AdapterError> {
        match event {
            ControlIndicatorEvent::Keys { display, .. } => {
                self.calls.lock().unwrap().push((*display).to_owned())
            }
            ControlIndicatorEvent::Click { point, .. } => self
                .calls
                .lock()
                .unwrap()
                .push(format!("click:{},{}", point.x, point.y)),
        }
        Ok(())
    }
}

fn adapter(recorder: Recorder) -> AdAdapter {
    AdAdapter {
        inner: Box::new(recorder),
        session_id: None,
        _session_lease: None,
    }
}

#[test]
fn input_is_hidden_and_the_transient_indicator_is_cleaned_up() {
    let recorder = Recorder::default();
    let calls = recorder.calls.clone();
    let adapter = adapter(recorder);
    let guard = begin(
        &adapter,
        "type",
        Some(&Action::TypeText("private text".into())),
    )
    .unwrap()
    .unwrap();
    guard.click(
        &Point { x: -4.0, y: 10.0 },
        DeliverySemantics::delivered_unverified(),
    );
    drop(guard);
    assert_eq!(
        *calls.lock().unwrap(),
        [
            "show:false:type",
            "Text input requested",
            "click:-4,10",
            "hide"
        ]
    );
}

#[test]
fn unavailable_indicator_refuses_before_delivery() {
    let adapter = adapter(Recorder {
        fail: true,
        ..Default::default()
    });
    let error = begin(&adapter, "click", None).err().unwrap();
    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::NotDelivered
    );
}

#[test]
fn key_labels_reveal_shortcuts_without_revealing_text() {
    let key = |key: &str, modifiers| {
        Action::PressKey(acdesktop_core::KeyCombo {
            key: key.into(),
            modifiers,
        })
    };
    assert_eq!(
        keyboard_cue(&key("a", vec![])).as_deref(),
        Some("Character key requested")
    );
    assert_eq!(
        keyboard_cue(&key("s", vec![acdesktop_core::Modifier::Ctrl])).as_deref(),
        Some("Key requested: Ctrl+s")
    );
    assert_eq!(
        keyboard_cue(&key("secret", vec![acdesktop_core::Modifier::Ctrl])).as_deref(),
        Some("Key requested")
    );
}

#[test]
fn ffi_mouse_click_shows_the_indicator_and_click_effect() {
    let recorder = Recorder::default();
    let calls = recorder.calls.clone();
    let handle = crate::adapter::register_adapter(adapter(recorder)).unwrap();
    let event = crate::types::AdMouseEvent {
        kind: 3,
        point: crate::types::AdPoint { x: -4.0, y: 10.0 },
        button: 0,
        click_count: 1,
    };
    let result = unsafe { crate::input::mouse::ad_mouse_event(handle, &event) };
    unsafe { crate::adapter::ad_adapter_destroy(handle) };
    assert_eq!(result, crate::error::AdResult::Ok);
    assert_eq!(
        *calls.lock().unwrap(),
        ["show:false:mouse-input", "click:-4,10", "hide"]
    );
}

#[test]
fn ffi_click_effect_remains_visible_after_a_post_delivery_error() {
    let recorder = Recorder {
        fail_after_delivery: true,
        ..Default::default()
    };
    let calls = recorder.calls.clone();
    let handle = crate::adapter::register_adapter(adapter(recorder)).unwrap();
    let event = crate::types::AdMouseEvent {
        kind: 3,
        point: crate::types::AdPoint { x: -4.0, y: 10.0 },
        button: 0,
        click_count: 1,
    };
    let result = unsafe { crate::input::mouse::ad_mouse_event(handle, &event) };
    unsafe { crate::adapter::ad_adapter_destroy(handle) };
    assert_eq!(result, crate::error::AdResult::ErrActionFailed);
    assert_eq!(
        *calls.lock().unwrap(),
        ["show:false:mouse-input", "click:-4,10", "hide"]
    );
}

#[test]
fn ffi_clipboard_write_stops_when_indicator_is_unavailable() {
    let recorder = Recorder {
        fail: true,
        ..Default::default()
    };
    let calls = recorder.calls.clone();
    let handle = crate::adapter::register_adapter(adapter(recorder)).unwrap();
    let result =
        unsafe { crate::input::clipboard::ad_set_clipboard(handle, c"private text".as_ptr()) };
    unsafe { crate::adapter::ad_adapter_destroy(handle) };
    assert_eq!(result, crate::error::AdResult::ErrInternal);
    assert_eq!(*calls.lock().unwrap(), ["show:false:set-clipboard"]);
}
