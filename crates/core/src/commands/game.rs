use crate::{GameReport, GameSessionPlan, GameSessionRequest, PlatformAdapter};
use std::io::{Read, Write};
use std::path::PathBuf;

const SESSION_ALLOWANCE_MS: u64 = 20_000;
const MAX_PLAN_BYTES: u64 = 8 * 1024 * 1024;

pub struct GameArgs {
    pub app: Option<String>,
    pub window_id: Option<String>,
    pub plan_path: PathBuf,
    pub out_dir: PathBuf,
    pub duration_ms: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub save_frames: bool,
}

pub fn execute(
    args: GameArgs,
    adapter: &dyn PlatformAdapter,
    context: &crate::context::CommandContext,
) -> Result<serde_json::Value, crate::AppError> {
    let plan = override_duration(read_plan(&args)?, args.duration_ms);
    plan.validate().map_err(crate::AppError::invalid_input)?;
    if plan.capture_cursor && !args.save_frames {
        return Err(crate::AppError::invalid_input(
            "capture_cursor requires --save-frames",
        ));
    }
    if !plan.events.is_empty() {
        crate::commands::point_resolve::require_cursor_policy(context, "game input")?;
    }
    if !adapter.supports_game_session() {
        return Err(crate::AdapterError::not_supported("run_game_session").into());
    }
    let window = crate::commands::window_target::resolve_window_for_app(
        args.app.as_deref(),
        args.window_id.as_deref(),
        adapter,
    )?;
    let timeout_ms = args
        .timeout_ms
        .unwrap_or_else(|| plan.duration_ms.saturating_add(SESSION_ALLOWANCE_MS));
    let deadline = crate::Deadline::after(timeout_ms)?;
    let lease = adapter.acquire_interaction_lease(deadline)?;
    let frames_dir = prepare_output(&args)?;
    if !plan.events.is_empty() {
        adapter.focus_window(&window, &lease)?;
    }
    let request = GameSessionRequest {
        window: &window,
        plan: &plan,
        lease: &lease,
        frames_dir: frames_dir.as_deref(),
    };
    let log = adapter.run_game_session(&request, deadline)?;
    let report = GameReport::from_log(&log, &plan);
    let report_path = args.out_dir.join("report.json");
    let bytes = serde_json::to_vec_pretty(&report).map_err(|error| {
        after_session_error(
            crate::AppError::invalid_input_with_suggestion(
                format!("The game report could not be serialized: {error}"),
                "Report this as a bug; the report is produced from a fixed schema.",
            ),
            !log.inputs.is_empty(),
        )
    })?;
    write_new_report(&report_path, &bytes).map_err(|error| {
        after_session_error(
            crate::refs::unwritable_output("the game report", &report_path, error),
            !log.inputs.is_empty(),
        )
    })?;
    let probe_count = report.probes.len();
    let responded = report.probes.iter().filter(|probe| probe.responded).count();
    let mut data = serde_json::json!({
        "report": report_path.to_string_lossy(),
        "plan": args.plan_path.to_string_lossy(),
        "window": window.id,
        "summary": report
    });
    if plan.brief_response
        && let Some(summary) = data
            .get_mut("summary")
            .and_then(serde_json::Value::as_object_mut)
    {
        summary.remove("probes");
        summary.insert(
            "probe_summary".to_owned(),
            serde_json::json!({
                "total": probe_count,
                "responded": responded,
            }),
        );
    }
    if let Some(frames) = frames_dir {
        data["frames_saved"] = count_files(&frames).into();
    }
    Ok(data)
}

fn write_new_report(path: &std::path::Path, bytes: &[u8]) -> Result<(), crate::AppError> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn after_session_error(error: crate::AppError, input_dispatched: bool) -> crate::AppError {
    match error {
        crate::AppError::Adapter(mut error) if input_dispatched => {
            error.disposition = crate::DeliverySemantics::unknown();
            crate::AppError::Adapter(error)
        }
        other => other,
    }
}

fn count_files(dir: &std::path::Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "png"))
                .count() as u64
        })
        .unwrap_or(0)
}

fn read_plan(args: &GameArgs) -> Result<GameSessionPlan, crate::AppError> {
    let file = std::fs::File::open(&args.plan_path).map_err(|error| {
        crate::AppError::invalid_input_with_suggestion(
            format!(
                "The plan '{}' could not be read: {error}",
                args.plan_path.display()
            ),
            "Pass a readable JSON plan with --plan.",
        )
    })?;
    let mut text = String::new();
    file.take(MAX_PLAN_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|error| {
            crate::AppError::invalid_input(format!(
                "The game plan could not be read as UTF-8: {error}"
            ))
        })?;
    if text.len() as u64 > MAX_PLAN_BYTES {
        return Err(crate::AppError::invalid_input(
            "The game plan exceeds 8 MiB",
        ));
    }
    GameSessionPlan::parse(&text).map_err(|error| {
        crate::AppError::invalid_input_with_suggestion(
            format!(
                "The plan '{}' is not valid: {error}",
                args.plan_path.display()
            ),
            "See the game documentation for the plan schema.",
        )
    })
}

fn override_duration(plan: GameSessionPlan, duration_ms: Option<u64>) -> GameSessionPlan {
    match duration_ms {
        Some(duration) => GameSessionPlan {
            duration_ms: duration,
            ..plan
        },
        None => plan,
    }
}

fn prepare_output(args: &GameArgs) -> Result<Option<PathBuf>, crate::AppError> {
    std::fs::create_dir_all(&args.out_dir).map_err(|error| {
        crate::AppError::invalid_input_with_suggestion(
            format!(
                "The output directory '{}' could not be created: {error}",
                args.out_dir.display()
            ),
            "Pass a writable directory with --out.",
        )
    })?;
    if args.out_dir.join("report.json").exists() {
        return Err(crate::AppError::invalid_input(
            "The output directory already contains report.json; choose a fresh directory",
        ));
    }
    if !args.save_frames {
        return Ok(None);
    }
    let frames = args.out_dir.join("frames");
    std::fs::create_dir(&frames).map_err(|error| {
        crate::AppError::invalid_input_with_suggestion(
            format!(
                "The frame directory '{}' could not be created: {error}",
                frames.display()
            ),
            "Pass a writable directory with --out.",
        )
    })?;
    Ok(Some(frames))
}

#[cfg(test)]
#[path = "game_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "game_delivery_tests.rs"]
mod delivery_tests;

#[cfg(test)]
#[path = "game_brief_tests.rs"]
mod brief_tests;
