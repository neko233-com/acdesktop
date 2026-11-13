use super::*;
use crate::{
    AdapterError, AppError, DeliverySemantics, GameSessionLog, GameSessionRequest, InputSample,
    WindowInfo, WindowState,
    adapter::{ActionOps, InputOps, ObservationOps, SystemOps, WindowFilter},
    system::process_id::ProcessId,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct ReportCollisionAdapter {
    window: WindowInfo,
    output: PathBuf,
    input: bool,
    regular: bool,
}

impl ObservationOps for ReportCollisionAdapter {
    fn list_windows(
        &self,
        _: &WindowFilter,
        _: crate::Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        Ok(vec![self.window.clone()])
    }
}
impl ActionOps for ReportCollisionAdapter {}
impl InputOps for ReportCollisionAdapter {}
impl SystemOps for ReportCollisionAdapter {
    crate::adapter::guarded_interaction_lease!();
    fn supports_game_session(&self) -> bool {
        true
    }
    fn focus_window(
        &self,
        _: &WindowInfo,
        _: &crate::InteractionLease,
    ) -> Result<(), AdapterError> {
        Ok(())
    }
    fn run_game_session(
        &self,
        _: &GameSessionRequest<'_>,
        _: crate::Deadline,
    ) -> Result<GameSessionLog, AdapterError> {
        if self.regular {
            std::fs::write(self.output.join("report.json"), "other writer")
                .expect("competing report");
        } else {
            std::fs::create_dir(self.output.join("report.json")).expect("competing directory");
        }
        Ok(GameSessionLog {
            duration_ms: 1000,
            width: 100,
            height: 100,
            frames: Vec::new(),
            inputs: if self.input {
                vec![InputSample {
                    event: 0,
                    scheduled_ms: 10,
                    dispatched_ms: 10,
                    kind: "mouse-move".into(),
                }]
            } else {
                vec![]
            },
        })
    }
}

fn collision_disposition(input: bool, regular: bool) -> DeliverySemantics {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("acdesktop-game-delivery-{nonce}"));
    std::fs::create_dir(&dir).expect("scratch directory");
    let plan_path = dir.join("plan.json");
    let events = if input {
        r#"[{"at_ms":10,"action":"mouse-move","x":10,"y":10}]"#
    } else {
        "[]"
    };
    std::fs::write(
        &plan_path,
        format!(r#"{{"duration_ms":1000,"events":{events}}}"#),
    )
    .expect("plan");
    let output = dir.join("out");
    let adapter = ReportCollisionAdapter {
        window: WindowInfo {
            id: "w-1".into(),
            title: "game".into(),
            app: "game.exe".into(),
            pid: ProcessId::new(42),
            process_instance: None,
            bounds: None,
            state: WindowState::default(),
        },
        output: output.clone(),
        input,
        regular,
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".into()),
        plan_path,
        out_dir: output.clone(),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let error = execute(
        args,
        &adapter,
        &crate::context::CommandContext::default().with_headed(true),
    )
    .expect_err("report collision");
    if regular {
        assert_eq!(
            std::fs::read_to_string(output.join("report.json")).expect("competing report"),
            "other writer"
        );
    }
    match error {
        AppError::Adapter(error) => error.disposition,
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn report_write_failure_never_marks_prior_input_safe_to_retry() {
    assert_eq!(
        collision_disposition(true, false),
        DeliverySemantics::unknown()
    );
    assert_eq!(
        collision_disposition(false, false),
        DeliverySemantics::not_delivered()
    );
}

#[test]
fn a_report_created_during_the_session_is_not_overwritten() {
    assert_eq!(
        collision_disposition(true, true),
        DeliverySemantics::unknown()
    );
}
