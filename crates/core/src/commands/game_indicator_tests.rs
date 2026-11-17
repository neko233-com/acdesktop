use std::sync::Mutex;

use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps, WindowFilter};
use crate::{AdapterError, Deadline, GameSessionLog, GameSessionRequest, WindowInfo};

use super::tests::{plan_file, scratch_dir, window};
use super::{GameArgs, execute};

#[derive(Default)]
struct IndicatorAdapter {
    calls: Mutex<Vec<String>>,
    fail_show: bool,
}

impl ObservationOps for IndicatorAdapter {
    fn list_windows(
        &self,
        _filter: &WindowFilter,
        _deadline: Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        Ok(vec![window()])
    }
}
impl ActionOps for IndicatorAdapter {}
impl InputOps for IndicatorAdapter {}
impl SystemOps for IndicatorAdapter {
    crate::adapter::guarded_interaction_lease!();

    fn requires_control_indicator(&self) -> bool {
        true
    }

    fn show_control_indicator(
        &self,
        owner: &str,
        _session: bool,
        _description: &str,
    ) -> Result<(), AdapterError> {
        self.calls.lock().unwrap().push(format!("show:{owner}"));
        if self.fail_show {
            Err(AdapterError::internal("indicator unavailable"))
        } else {
            Ok(())
        }
    }

    fn hide_control_indicator(&self, owner: &str) -> Result<(), AdapterError> {
        self.calls.lock().unwrap().push(format!("hide:{owner}"));
        Ok(())
    }

    fn supports_game_session(&self) -> bool {
        true
    }

    fn focus_window(
        &self,
        _window: &WindowInfo,
        _lease: &crate::InteractionLease,
    ) -> Result<(), AdapterError> {
        self.calls.lock().unwrap().push("focus".into());
        Ok(())
    }

    fn run_game_session(
        &self,
        request: &GameSessionRequest<'_>,
        _deadline: Deadline,
    ) -> Result<GameSessionLog, AdapterError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("run:{}", request.indicator_owner.unwrap_or("none")));
        Ok(GameSessionLog {
            duration_ms: 1000,
            width: 100,
            height: 100,
            frames: vec![],
            inputs: vec![],
        })
    }
}

fn run(adapter: &IndicatorAdapter, events: &str) -> Result<serde_json::Value, crate::AppError> {
    let dir = scratch_dir();
    let plan = plan_file(
        &dir,
        &format!(r#"{{"duration_ms":1000,"events":{events}}}"#),
    );
    execute(
        GameArgs {
            app: None,
            window_id: Some("w-1".into()),
            plan_path: plan,
            out_dir: dir.join("out"),
            duration_ms: None,
            timeout_ms: None,
            save_frames: false,
        },
        adapter,
        &crate::CommandContext::default().with_headed(true),
    )
}

#[test]
fn input_session_shows_indicator_before_focus_and_cleans_up() {
    let adapter = IndicatorAdapter::default();
    run(&adapter, r#"[{"at_ms":100,"action":"key","key":"space"}]"#).unwrap();
    let calls = adapter.calls.lock().unwrap();
    let owner = calls[0].strip_prefix("show:").unwrap();
    assert_eq!(
        calls.as_slice(),
        [
            format!("show:{owner}"),
            "focus".into(),
            format!("run:{owner}"),
            format!("hide:{owner}")
        ]
    );
}

#[test]
fn indicator_failure_refuses_before_focus_and_observation_has_no_indicator() {
    let adapter = IndicatorAdapter {
        fail_show: true,
        ..Default::default()
    };
    assert!(run(&adapter, r#"[{"at_ms":100,"action":"key","key":"space"}]"#).is_err());
    assert_eq!(adapter.calls.lock().unwrap().len(), 1);
    adapter.calls.lock().unwrap().clear();
    run(&adapter, "[]").unwrap();
    assert_eq!(adapter.calls.lock().unwrap().as_slice(), ["run:none"]);
}
