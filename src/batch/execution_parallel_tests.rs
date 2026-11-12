use super::*;
use acdesktop_core::{AdapterError, InteractionLease, OperationReport, OperationTarget, ProcessId};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Default)]
struct Adapter {
    active: AtomicUsize,
    peak: AtomicUsize,
    events: Mutex<Vec<String>>,
    oversized: bool,
}

impl acdesktop_core::ObservationOps for Adapter {
    fn list_apps(&self, _: Deadline) -> Result<Vec<acdesktop_core::AppInfo>, AdapterError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(30));
        self.active.fetch_sub(1, Ordering::SeqCst);
        if self.oversized {
            Ok(vec![acdesktop_core::AppInfo {
                name: "x".repeat(MAX_BATCH_OUTPUT_BYTES),
                pid: ProcessId::new(1),
                bundle_id: None,
                process_instance: None,
                presentation: None,
            }])
        } else {
            Ok(Vec::new())
        }
    }
}
impl acdesktop_core::ActionOps for Adapter {}
impl acdesktop_core::InputOps for Adapter {
    fn clear_clipboard(&self, _: &InteractionLease) -> Result<OperationReport, AdapterError> {
        self.events.lock().unwrap().push("clear".into());
        Ok(OperationReport::delivered_unverified(
            "test",
            OperationTarget::Clipboard,
        ))
    }
}
impl acdesktop_core::SystemOps for Adapter {
    fn acquire_interaction_lease(
        &self,
        deadline: Deadline,
    ) -> Result<InteractionLease, AdapterError> {
        InteractionLease::guarded(deadline, ())
    }

    fn launch_app(
        &self,
        id: &str,
        _: &acdesktop_core::system::launch_options::LaunchOptions,
        _: &InteractionLease,
    ) -> Result<acdesktop_core::system::launch_result::LaunchResult, AdapterError> {
        if id == "fail" {
            return Err(AdapterError::not_supported("fixture"));
        }
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(30));
        self.events.lock().unwrap().push(id.into());
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(acdesktop_core::system::launch_result::LaunchResult {
            app: id.into(),
            pid: ProcessId::new(1),
            process_instance: None,
            window: None,
            cdp: None,
            renderer: None,
            suggestion: None,
        })
    }
}

fn run(adapter: &Adapter, commands: Value, max_workers: usize, stop_on_error: bool) -> Value {
    super::execute_parallel(
        BatchArgs {
            commands_json: commands.to_string(),
            stop_on_error,
            timeout_ms: 60_000,
            parallel: true,
            max_workers,
        },
        adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .unwrap()
}

#[test]
fn worker_limit_is_enforced_and_result_names_are_commands() {
    let adapter = Adapter::default();
    let output = run(
        &adapter,
        json!([
            {"command":"list-apps", "args":{}},
            {"command":"list-apps", "args":{}},
            {"command":"list-apps", "args":{}}
        ]),
        1,
        false,
    );
    assert_eq!(adapter.peak.load(Ordering::SeqCst), 1);
    assert_eq!(output["completed_entries"], 3);
    assert_eq!(output["results"][0]["command"], "list-apps");
    let concurrent = Adapter::default();
    run(
        &concurrent,
        json!([
            {"command":"list-apps", "args":{}},
            {"command":"list-apps", "args":{}},
            {"command":"list-apps", "args":{}},
            {"command":"list-apps", "args":{}}
        ]),
        2,
        false,
    );
    assert_eq!(concurrent.peak.load(Ordering::SeqCst), 2);
}

#[test]
fn global_operations_remain_between_surrounding_commands() {
    let adapter = Adapter::default();
    run(
        &adapter,
        json!([
            {"command":"launch", "args":{"app":"first"}},
            {"command":"clipboard-clear", "args":{}},
            {"command":"launch", "args":{"app":"last"}}
        ]),
        2,
        false,
    );
    assert_eq!(*adapter.events.lock().unwrap(), ["first", "clear", "last"]);
}

#[test]
fn stop_on_error_does_not_start_a_later_action() {
    let adapter = Adapter::default();
    let output = run(
        &adapter,
        json!([
            {"command":"launch", "args":{"app":"fail"}},
            {"command":"clipboard-clear", "args":{}}
        ]),
        2,
        true,
    );
    assert_eq!(output["semantics"]["order"], "sequential");
    assert!(adapter.events.lock().unwrap().is_empty());
}

#[test]
fn oversized_observation_stops_later_side_effects() {
    let adapter = Adapter {
        oversized: true,
        ..Adapter::default()
    };
    let output = run(
        &adapter,
        json!([
            {"command":"list-apps", "args":{}},
            {"command":"clipboard-clear", "args":{}}
        ]),
        2,
        false,
    );
    assert_eq!(output["stopped"]["reason"], "output_limit");
    assert_eq!(output["completed_entries"], 1);
    assert!(adapter.events.lock().unwrap().is_empty());
}

#[test]
fn planning_groups_same_identity_and_separates_observation_and_global_barriers() {
    let commands = [
        ("click", json!({"ref_id":"@s12345678:e1"})),
        ("click", json!({"ref_id":"@s12345678:e2"})),
        ("click", json!({"ref_id":"@s12345678:e3"})),
        ("clipboard-clear", json!({})),
        ("list-apps", json!({})),
        ("click", json!({"ref_id":"@s12345678:e1"})),
    ];
    let items = commands
        .into_iter()
        .map(|(name, args)| super::super::preparation::PreparedCommand {
            name: name.into(),
            command: crate::batch::parse_command(acdesktop_core::commands::batch::BatchCommand {
                command: name.into(),
                args,
                session: None,
            })
            .unwrap(),
            context: CommandContext::default(),
        })
        .collect();
    let segments = super::plan_segments(items, |item| match &item.command {
        Commands::Click(args) if args.ref_id.ends_with(":e2") => Some("process-b".into()),
        Commands::Click(_) => Some("process-a".into()),
        _ => None,
    });
    let indices: Vec<Vec<Vec<usize>>> = segments
        .into_iter()
        .map(|segment| {
            segment
                .into_iter()
                .map(|bucket| bucket.into_iter().map(|(i, _)| i).collect())
                .collect()
        })
        .collect();
    assert_eq!(
        indices,
        vec![
            vec![vec![0, 2], vec![1]],
            vec![vec![3]],
            vec![vec![4]],
            vec![vec![5]]
        ]
    );
}
