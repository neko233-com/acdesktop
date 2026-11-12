use std::sync::atomic::{AtomicBool, Ordering};

use acdesktop_core::{
    PermissionReport, SignalBaseline, SignalCompleteness, SignalFilter, WindowState,
};

use super::execute;
use crate::cli_args::batch::BatchArgs;

struct AtomicEventAdapter {
    opened: AtomicBool,
}

impl acdesktop_core::ObservationOps for AtomicEventAdapter {
    fn list_apps(
        &self,
        _deadline: acdesktop_core::Deadline,
    ) -> Result<Vec<acdesktop_core::AppInfo>, acdesktop_core::AdapterError> {
        Ok(vec![acdesktop_core::AppInfo {
            name: "Fixture".into(),
            pid: acdesktop_core::ProcessId::new(42),
            bundle_id: Some("com.example.fixture".into()),
            process_instance: Some("fixture-42".into()),
            presentation: None,
        }])
    }
}

impl acdesktop_core::ActionOps for AtomicEventAdapter {}

impl acdesktop_core::InputOps for AtomicEventAdapter {
    fn clear_clipboard(
        &self,
        _lease: &acdesktop_core::InteractionLease,
    ) -> Result<acdesktop_core::OperationReport, acdesktop_core::AdapterError> {
        self.opened.store(true, Ordering::SeqCst);
        Ok(acdesktop_core::OperationReport::delivered_unverified(
            "test",
            acdesktop_core::OperationTarget::Clipboard,
        ))
    }
}

impl acdesktop_core::SystemOps for AtomicEventAdapter {
    fn acquire_interaction_lease(
        &self,
        deadline: acdesktop_core::Deadline,
    ) -> Result<acdesktop_core::InteractionLease, acdesktop_core::AdapterError> {
        acdesktop_core::InteractionLease::guarded(deadline, ())
    }

    fn capture_signal_baseline(
        &self,
        _filter: &SignalFilter,
        _deadline: acdesktop_core::Deadline,
    ) -> Result<SignalBaseline, acdesktop_core::AdapterError> {
        let windows = self
            .opened
            .load(Ordering::SeqCst)
            .then(|| acdesktop_core::WindowInfo {
                id: "w-sync".into(),
                title: "Synchronous".into(),
                app: "Fixture".into(),
                pid: acdesktop_core::ProcessId::new(42),
                process_instance: Some("fixture-42".into()),
                bounds: None,
                state: WindowState {
                    is_focused: true,
                    ..WindowState::default()
                },
            })
            .into_iter()
            .collect();
        Ok(SignalBaseline {
            windows,
            apps: Vec::new(),
            surfaces: Vec::new(),
            completeness: SignalCompleteness::complete(),
        })
    }
}

#[test]
fn action_then_event_wait_uses_a_pre_action_baseline() {
    let args = BatchArgs {
        commands_json: serde_json::json!([
            {"command": "clipboard-clear", "args": {}},
            {
                "command": "wait",
                "args": {"event": "window-opened", "app": "Fixture", "timeout": 100}
            }
        ])
        .to_string(),
        stop_on_error: true,
        timeout_ms: 60_000,
    };
    let adapter = AtomicEventAdapter {
        opened: AtomicBool::new(false),
    };

    let value = execute(
        args,
        &adapter,
        &PermissionReport::default(),
        &acdesktop_core::CommandContext::default(),
    )
    .unwrap();

    assert_eq!(value["results"][0]["ok"], true, "{value}");
    assert_eq!(value["results"][1]["ok"], true, "{value}");
    assert_eq!(value["results"][1]["data"]["event"]["window_id"], "w-sync");
}

#[test]
fn wait_as_first_entry_runs_with_event_baseline_none() {
    let args = BatchArgs {
        commands_json: serde_json::json!([
            {
                "command": "wait",
                "args": {"event": "window-opened", "app": "Fixture", "timeout": 100}
            },
            {"command": "version", "args": {}}
        ])
        .to_string(),
        stop_on_error: false,
        timeout_ms: 60_000,
    };
    let adapter = AtomicEventAdapter {
        opened: AtomicBool::new(false),
    };

    let value = execute(
        args,
        &adapter,
        &PermissionReport::default(),
        &acdesktop_core::CommandContext::default(),
    )
    .unwrap();

    assert_eq!(value["results"][0]["execution"], "completed", "{value}");
    assert_eq!(value["results"][0]["ok"], false, "{value}");
    assert_eq!(value["results"][0]["error"]["code"], "TIMEOUT", "{value}");
    assert_eq!(value["results"][1]["execution"], "completed", "{value}");
    assert_eq!(value["results"][1]["ok"], true, "{value}");
    assert!(value.get("stopped").is_none(), "{value}");
}
