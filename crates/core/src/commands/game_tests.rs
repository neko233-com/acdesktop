use crate::{
    AdapterError, GameSessionLog, GameSessionRequest, WindowInfo, WindowState,
    adapter::{ActionOps, InputOps, ObservationOps, SystemOps, WindowFilter},
    commands::game::{self, GameArgs},
    system::process_id::ProcessId,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct RecordingAdapter {
    pub(super) focused: AtomicBool,
    pub(super) window: WindowInfo,
}

struct UnsupportedAdapter;
impl ObservationOps for UnsupportedAdapter {}
impl ActionOps for UnsupportedAdapter {}
impl InputOps for UnsupportedAdapter {}
impl SystemOps for UnsupportedAdapter {}

impl ObservationOps for RecordingAdapter {
    fn list_windows(
        &self,
        _filter: &WindowFilter,
        _deadline: crate::Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        Ok(vec![self.window.clone()])
    }
}

impl ActionOps for RecordingAdapter {}
impl InputOps for RecordingAdapter {}

impl SystemOps for RecordingAdapter {
    crate::adapter::guarded_interaction_lease!();

    fn supports_game_session(&self) -> bool {
        true
    }

    fn focus_window(
        &self,
        _window: &WindowInfo,
        _lease: &crate::InteractionLease,
    ) -> Result<(), AdapterError> {
        self.focused.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn run_game_session(
        &self,
        request: &GameSessionRequest<'_>,
        _deadline: crate::Deadline,
    ) -> Result<GameSessionLog, AdapterError> {
        Ok(log(request.plan.duration_ms.min(3_000)))
    }
}

fn log(duration_ms: u64) -> GameSessionLog {
    GameSessionLog {
        duration_ms,
        width: 640,
        height: 480,
        frames: (0..=duration_ms / 1_000)
            .map(|index| crate::system::game_frame_sample::FrameSample {
                at_ms: index * 1_000,
                changed_ratio: 0.0,
            })
            .collect(),
        inputs: Vec::new(),
    }
}

pub(super) fn window() -> WindowInfo {
    WindowInfo {
        id: "w-1".to_owned(),
        title: "Game".to_owned(),
        app: "Game.exe".to_owned(),
        pid: ProcessId::new(42),
        process_instance: None,
        bounds: None,
        state: WindowState::default(),
    }
}

pub(super) fn scratch_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("acdesktop-game-test-{nanos}"))
}

pub(super) fn plan_file(dir: &std::path::Path, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("a scratch directory");
    let path = dir.join("plan.json");
    std::fs::write(&path, body).expect("a plan file");
    path
}

#[test]
fn an_observation_only_plan_does_not_steal_focus() {
    let dir = scratch_dir();
    let plan = plan_file(&dir, r#"{"duration_ms": 2000, "fps": 10, "events": []}"#);
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let value = game::execute(args, &adapter, &crate::context::CommandContext::default())
        .expect("a background observation session");
    assert!(
        !adapter.focused.load(Ordering::Relaxed),
        "an observation session focused the window"
    );
    assert_eq!(value["summary"]["planned_ms"], 2000);
    assert_eq!(value["summary"]["truncated"], false);
}

#[test]
fn a_plan_with_events_focuses_the_window() {
    let dir = scratch_dir();
    let plan = plan_file(
        &dir,
        r#"{"duration_ms": 2000, "fps": 10, "events": [{"at_ms": 100, "action": "key", "key": "space"}]}"#,
    );
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    game::execute(
        args,
        &adapter,
        &crate::context::CommandContext::default().with_headed(true),
    )
    .expect("a session with input");
    assert!(
        adapter.focused.load(Ordering::Relaxed),
        "a session with input left the window unfocused"
    );
}

#[test]
fn headless_input_is_refused_before_focus_or_output() {
    let dir = scratch_dir();
    let plan = plan_file(
        &dir,
        r#"{"duration_ms": 2000, "events": [{"at_ms": 100, "action": "key", "key": "space"}]}"#,
    );
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let error = game::execute(args, &adapter, &crate::context::CommandContext::default())
        .expect_err("headless physical input must be refused");
    assert_eq!(error.code(), "POLICY_DENIED");
    assert!(!adapter.focused.load(Ordering::Relaxed));
    assert!(!dir.join("out").exists());
}

#[test]
fn unsupported_platform_refuses_before_creating_output_or_focusing() {
    let dir = scratch_dir();
    let plan = plan_file(
        &dir,
        r#"{"duration_ms": 2000, "events": [{"at_ms": 100, "action": "key", "key": "space"}]}"#,
    );
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let error = game::execute(
        args,
        &UnsupportedAdapter,
        &crate::context::CommandContext::default().with_headed(true),
    )
    .expect_err("unsupported platform must refuse");
    assert_eq!(error.code(), "PLATFORM_NOT_SUPPORTED");
    assert!(!dir.join("out").exists());
}

#[test]
fn existing_report_is_not_overwritten_and_input_is_not_started() {
    let dir = scratch_dir();
    let plan = plan_file(
        &dir,
        r#"{"duration_ms":2000,"events":[{"at_ms":100,"action":"key","key":"a"}]}"#,
    );
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("output directory");
    std::fs::write(out.join("report.json"), "original").expect("existing report");
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: out.clone(),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    assert!(
        game::execute(
            args,
            &adapter,
            &crate::context::CommandContext::default().with_headed(true)
        )
        .is_err()
    );
    assert!(!adapter.focused.load(Ordering::Relaxed));
    assert_eq!(
        std::fs::read_to_string(out.join("report.json")).expect("report"),
        "original"
    );
}

#[test]
fn cursor_capture_requires_saved_frames_before_any_focus() {
    let dir = scratch_dir();
    let plan = plan_file(
        &dir,
        r#"{"duration_ms":1000,"capture_cursor":true,"events":[{"at_ms":100,"action":"key","key":"a"}]}"#,
    );
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let error = game::execute(
        args,
        &adapter,
        &crate::context::CommandContext::default().with_headed(true),
    )
    .expect_err("cursor recording needs saved frames");
    assert_eq!(error.code(), "INVALID_ARGS");
    assert!(!adapter.focused.load(Ordering::Relaxed));
    assert!(!dir.join("out").exists());
}

#[test]
fn three_fingers_without_acknowledgement_never_focus_or_inject() {
    let dir = scratch_dir();
    let plan = plan_file(
        &dir,
        r#"{"duration_ms":1000,"events":[{"at_ms":100,"action":"gesture","gesture":"three-finger-swipe-right","x":100,"y":100}]}"#,
    );
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let error = game::execute(
        args,
        &adapter,
        &crate::context::CommandContext::default().with_headed(true),
    )
    .expect_err("three-finger plan must require acknowledgement");
    assert_eq!(error.code(), "INVALID_ARGS");
    assert!(!adapter.focused.load(Ordering::Relaxed));
    assert!(!dir.join("out").exists());
}

#[test]
fn saved_cursor_capture_is_reported() {
    let dir = scratch_dir();
    let plan = plan_file(&dir, r#"{"duration_ms":1000,"capture_cursor":true}"#);
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: true,
    };
    let result = game::execute(args, &adapter, &crate::context::CommandContext::default())
        .expect("cursor capture request");
    assert_eq!(result["summary"]["cursor_in_saved_frames"], true);
    assert_eq!(result["frames_saved"], 0);
}

#[test]
fn a_short_session_is_reported_as_truncated() {
    let dir = scratch_dir();
    let plan = plan_file(&dir, r#"{"duration_ms": 8000, "fps": 10, "events": []}"#);
    let adapter = RecordingAdapter {
        focused: AtomicBool::new(false),
        window: window(),
    };
    let args = GameArgs {
        app: None,
        window_id: Some("w-1".to_owned()),
        plan_path: plan,
        out_dir: dir.join("out"),
        duration_ms: None,
        timeout_ms: None,
        save_frames: false,
    };
    let value = game::execute(args, &adapter, &crate::context::CommandContext::default())
        .expect("a session");
    assert_eq!(value["summary"]["truncated"], true);
    assert_eq!(value["summary"]["duration_ms"], 3000);
}
