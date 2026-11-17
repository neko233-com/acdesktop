use super::*;
use std::sync::Mutex;

use acdesktop_core::{ActionOps, AdapterError, InputOps, ObservationOps, SystemOps};

#[derive(Default)]
struct Recorder {
    calls: Mutex<Vec<String>>,
    events: Mutex<Vec<String>>,
    fail: bool,
    fail_hide: bool,
}

impl ObservationOps for Recorder {}
impl ActionOps for Recorder {}
impl InputOps for Recorder {}
impl SystemOps for Recorder {
    fn requires_control_indicator(&self) -> bool {
        true
    }

    fn show_control_indicator(
        &self,
        owner: &str,
        session: bool,
        desc: &str,
    ) -> Result<(), AdapterError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("show:{owner}:{session}:{desc}"));
        if self.fail {
            Err(AdapterError::internal("indicator unavailable"))
        } else {
            Ok(())
        }
    }

    fn hide_control_indicator(&self, owner: &str) -> Result<(), AdapterError> {
        self.calls.lock().unwrap().push(format!("hide:{owner}"));
        if self.fail_hide {
            Err(AdapterError::internal("indicator teardown failed"))
        } else {
            Ok(())
        }
    }

    fn present_control_indicator(
        &self,
        event: &ControlIndicatorEvent<'_>,
    ) -> Result<(), AdapterError> {
        if let ControlIndicatorEvent::Keys { display, .. } = event {
            self.events.lock().unwrap().push((*display).to_owned());
        }
        Ok(())
    }
}

fn press() -> Commands {
    Commands::Press(crate::cli_args::actions::PressArgs {
        combo: "a".into(),
        app: None,
        window_id: None,
        force: false,
    })
}

#[test]
fn transient_keyboard_indicator_uses_description_and_cleans_up() {
    let adapter = Recorder::default();
    let context = CommandContext::default().with_description(Some("Typing a message".into()));
    let guard = begin(&press(), &adapter, &context).unwrap().unwrap();
    let first = adapter.calls.lock().unwrap()[0].clone();
    assert!(first.ends_with(":false:Typing a message"));
    let owner = first.split(':').nth(1).unwrap();
    drop(guard);
    assert_eq!(adapter.calls.lock().unwrap()[1], format!("hide:{owner}"));
}

#[test]
fn default_description_does_not_reveal_typed_text() {
    let adapter = Recorder::default();
    let command = Commands::Type(crate::cli_args::actions::TypeArgs {
        ref_id: "@s12345678:e1".into(),
        text: Some("private text".into()),
        timeout_ms: None,
        stdin: false,
        interval_ms: None,
    });
    drop(begin(&command, &adapter, &CommandContext::default()).unwrap());
    let calls = adapter.calls.lock().unwrap();
    assert!(calls[0].ends_with(":false:type"));
    assert!(!calls.iter().any(|call| call.contains("private text")));
    assert_eq!(
        adapter.events.lock().unwrap().as_slice(),
        ["Text input requested"]
    );
}

#[test]
fn shortcut_keys_are_visible_but_printable_input_is_hidden() {
    assert_eq!(key_display("ctrl+s"), "Key requested: ctrl+s");
    assert_eq!(key_display("Enter"), "Key requested: Enter");
    assert_eq!(key_display("shift+a"), "Character key requested");
    assert_eq!(key_display("a"), "Character key requested");
    assert_eq!(key_display("ctrl+private-password"), "Key requested");
}

#[test]
fn session_indicator_remains_visible_after_keyboard_command() {
    let _home = crate::dispatch::test_support::HomeGuard::new();
    let session = acdesktop_core::session::start_session(Default::default()).unwrap();
    let adapter = Recorder::default();
    let context = CommandContext::new(Some(session.id.clone()), None, false).unwrap();
    drop(begin(&press(), &adapter, &context).unwrap());
    assert_eq!(
        adapter.calls.lock().unwrap().as_slice(),
        [format!("show:{}:true:press", session.id)]
    );
}

#[test]
fn required_indicator_failure_refuses_before_the_action() {
    let adapter = Recorder {
        fail: true,
        ..Default::default()
    };
    assert!(begin(&press(), &adapter, &CommandContext::default()).is_err());
    assert_eq!(adapter.calls.lock().unwrap().len(), 1);
}

#[test]
fn namespace_without_manifest_uses_a_transient_indicator() {
    let _home = crate::dispatch::test_support::HomeGuard::new();
    let adapter = Recorder::default();
    let context = CommandContext::new(Some("snapshot-namespace".into()), None, false).unwrap();
    drop(begin(&press(), &adapter, &context).unwrap());
    let calls = adapter.calls.lock().unwrap();
    assert!(calls[0].ends_with(":false:press"));
    assert!(calls[1].starts_with("hide:once-"));
}

#[test]
fn session_start_and_end_control_the_persistent_indicator() {
    let _home = crate::dispatch::test_support::HomeGuard::new();
    let adapter = Recorder::default();
    let start = crate::cli_args::session::SessionArgs {
        action: crate::cli_args::session::SessionAction::Start(
            crate::cli_args::session::SessionStartArgs {
                name: None,
                no_trace: true,
                screenshots: false,
                cursor: false,
                multi_agent: false,
            },
        ),
    };
    let context = CommandContext::default().with_description(Some("Preparing".into()));
    let result = super::super::session::dispatch(start, &adapter, &context).unwrap();
    assert_eq!(result["control_indicator"]["rendered"], true);
    let id = result["session_id"].as_str().unwrap();
    let end = crate::cli_args::session::SessionArgs {
        action: crate::cli_args::session::SessionAction::End(
            crate::cli_args::session::SessionEndArgs {
                id: Some(id.into()),
            },
        ),
    };
    super::super::session::dispatch(end, &adapter, &CommandContext::default()).unwrap();
    assert_eq!(
        adapter.calls.lock().unwrap().as_slice(),
        [format!("show:{id}:true:Preparing"), format!("hide:{id}"),]
    );
}

#[test]
fn failed_required_indicator_seals_the_new_session() {
    let _home = crate::dispatch::test_support::HomeGuard::new();
    let adapter = Recorder {
        fail: true,
        ..Default::default()
    };
    let start = crate::cli_args::session::SessionArgs {
        action: crate::cli_args::session::SessionAction::Start(
            crate::cli_args::session::SessionStartArgs {
                name: None,
                no_trace: true,
                screenshots: false,
                cursor: false,
                multi_agent: false,
            },
        ),
    };
    assert!(super::super::session::dispatch(start, &adapter, &CommandContext::default()).is_err());
    let sessions = acdesktop_core::session::list_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].ended_at.is_some());
}

#[test]
fn session_end_reports_indicator_teardown_failure_after_sealing() {
    let _home = crate::dispatch::test_support::HomeGuard::new();
    let manifest = acdesktop_core::session::start_session(Default::default()).unwrap();
    let adapter = Recorder {
        fail_hide: true,
        ..Default::default()
    };
    let end = crate::cli_args::session::SessionArgs {
        action: crate::cli_args::session::SessionAction::End(
            crate::cli_args::session::SessionEndArgs {
                id: Some(manifest.id.clone()),
            },
        ),
    };
    let error =
        super::super::session::dispatch(end, &adapter, &CommandContext::default()).unwrap_err();
    assert!(error.to_string().contains("control indicator teardown"));
    assert!(
        acdesktop_core::session::read_manifest(&manifest.id)
            .unwrap()
            .unwrap()
            .ended_at
            .is_some()
    );
}
