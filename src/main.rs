#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod batch;
mod cli;
mod cli_args;
mod command_policy;
mod config;
mod diagnostic;
mod dispatch;
mod mcp;
mod updater;
mod visual_debug;

/// Shared blanket-default `PlatformAdapter` test double, sourced once from
/// `tests/support/noop_ops.rs` (also consumed by the standalone
/// `conformance` integration-test crate) and reused by any in-crate unit
/// test that needs "some adapter" without exercising a live capability.
#[cfg(test)]
#[path = "../tests/support/noop_ops.rs"]
mod test_noop_ops;

use acdesktop_core::{
    AdapterError, AppError, DeliverySemantics, ErrorCode, SystemOps,
    context::{CommandContext, WaitSelector},
    response::output::{ErrorPayload, Response},
    session::resolve_active_session,
};
use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, Commands};
use cli_args::sandbox::SandboxActionCli;
use cli_args::skills::SkillsAction;
use std::io::{BufWriter, Write};
use std::process::ExitCode;

const EXIT_INVALID_ARGS: u8 = 2;

fn main() -> ExitCode {
    let code = run_command();
    #[cfg(target_os = "linux")]
    acdesktop_linux::end_input_session();
    code
}

fn run_command() -> ExitCode {
    if let Some(result) = build_adapter().run_cursor_overlay_child() {
        return if result.is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    #[cfg(target_os = "macos")]
    if let Some(exit_code) = run_permission_prompt_helper() {
        return exit_code;
    }
    #[cfg(target_os = "macos")]
    if let Some(exit_code) = acdesktop_macos::permission_guide_from_env() {
        return ExitCode::from(exit_code);
    }
    #[cfg(target_os = "macos")]
    acdesktop_macos::stop_legacy_command_hosts();
    run()
}

#[cfg(target_os = "macos")]
fn run_permission_prompt_helper() -> Option<ExitCode> {
    let (exit_code, response) = acdesktop_macos::permission_prompt_helper_from_env()?;
    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let result = writer
        .write_all(response.as_bytes())
        .and_then(|()| writer.write_all(b"\n"))
        .and_then(|()| writer.flush());
    Some(match result {
        Ok(()) => ExitCode::from(exit_code),
        Err(error) => report_output_failure(error),
    })
}

fn run() -> ExitCode {
    #[cfg(target_os = "windows")]
    let _ = acdesktop_core::install_private_file_ops(Box::new(
        acdesktop_windows::WindowsPrivateFile,
    ));

    let matches = match root_command().try_get_matches() {
        Ok(matches) => matches,
        Err(e) => return report_parse_failure(e),
    };
    if let Some(("mcp", mcp_matches)) = matches.subcommand() {
        return mcp::serve(mcp_matches);
    }
    let mut cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => return report_parse_failure(e),
    };

    init_tracing(cli.visual_debug.verbose);

    // 极快模式：跳过所有 post_action_wait（--wait-for / --wait-for-gone / --wait-timeout）
    if cli.fast_mode() {
        cli.post_action_wait.wait_for = None;
        cli.post_action_wait.wait_for_gone = None;
        cli.post_action_wait.wait_timeout = None;
    }

    let mut cmd = match cli.command.take() {
        Some(c) => c,
        None => {
            return match root_command().print_help() {
                Ok(()) => ExitCode::SUCCESS,
                Err(write_err) => report_output_failure(write_err),
            };
        }
    };

    let cmd_name = cmd.name();

    // ---------- 不需要 PlatformAdapter / stdin 的命令，提前处理 ----------
    match &cmd {
        Commands::Update(args) => return finish(cmd_name, handle_update(args)),
        Commands::Config(args) => return finish(cmd_name, handle_config(args)),
        _ => {}
    }
    // 每次调用时静默跑一次自动更新检查（可通过 config / env 关闭）
    updater::maybe_silent_check()
        .map(|n| eprintln!("acdesktop: 有新版本 {} → 当前 {}，运行 `acdesktop update apply` 升级", n.latest, n.current));

    if let Commands::Sandbox(args) = &cmd
        && let SandboxActionCli::Exec(exec) = &args.action
    {
        return dispatch::sandbox::run_exec(&cli, exec, cmd_name, &build_adapter());
    }
    let stdin = cli_args::stdin::input(&mut cmd).is_some();
    if let Err(error) = cli_args::stdin::resolve_stdin_text(&mut cmd, std::io::stdin().lock()) {
        return finish(cmd_name, Err(error));
    }
    finish(
        cmd_name,
        cli_args::stdin::redact_result(execute(cli, cmd), stdin),
    )
}

/// The parsed command tree: every `Commands` variant plus `mcp`, which serves them.
fn root_command() -> clap::Command {
    Cli::command().subcommand(mcp::command())
}

fn report_parse_failure(e: clap::Error) -> ExitCode {
    if matches!(
        e.kind(),
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
    ) {
        return match e.print() {
            Ok(()) => ExitCode::SUCCESS,
            Err(write_err) => report_output_failure(write_err),
        };
    }
    let summary = diagnostic::clap_error_summary(&e.to_string());
    let error = AppError::invalid_input(diagnostic::bounded_text(&summary, 512));
    let command_name = diagnostic::clap_command_name(std::env::args().skip(1));
    if let Err(write_err) = emit_response(&Response::err(
        &command_name,
        ErrorPayload::from_app_error(&error),
    )) {
        report_output_failure(write_err)
    } else {
        ExitCode::from(EXIT_INVALID_ARGS)
    }
}

fn execute(cli: Cli, cmd: Commands) -> Result<serde_json::Value, AppError> {
    execute_with_deadline(cli, cmd, None, None)
}

fn execute_with_deadline(
    cli: Cli,
    cmd: Commands,
    deadline: Option<acdesktop_core::Deadline>,
    baseline: Option<acdesktop_core::SignalBaseline>,
) -> Result<serde_json::Value, AppError> {
    if let Some(deadline) = deadline.filter(|deadline| deadline.is_expired()) {
        return Err(pre_dispatch_error(deadline.timeout_error().into()));
    }
    cli.visual_debug.validate(&cmd)?;
    acdesktop_core::validate_state_root_env().map_err(pre_dispatch_error)?;
    match cmd {
        Commands::Version => {
            acdesktop_core::commands::version::execute().map_err(pre_dispatch_error)
        }
        Commands::Skills(a) => {
            let result = match a.action.unwrap_or(SkillsAction::List) {
                SkillsAction::List => acdesktop_core::commands::skills::list(),
                SkillsAction::Path => acdesktop_core::commands::skills::path(),
                SkillsAction::Get(g) => acdesktop_core::commands::skills::get(
                    acdesktop_core::commands::skills::GetArgs {
                        name: g.name,
                        full: g.full,
                        reference: g.reference,
                    },
                ),
            };
            result.map_err(pre_dispatch_error)
        }
        cmd => {
            let wait_selector = build_wait_selector(&cli)?;
            let session_id = resolve_active_session(
                cli.identity.session.as_deref(),
                std::env::var("ACDESKTOP_SESSION").ok().as_deref(),
            )
            .map_err(pre_dispatch_error)?;
            let context = CommandContext::new(session_id, cli.trace, cli.trace_strict)
                .map_err(pre_dispatch_error)?
                .with_headed(cli.interaction.headed)
                .with_wait_selector(wait_selector.clone());
            let agent_id = cli
                .identity
                .agent_id
                .clone()
                .or_else(|| std::env::var("ACDESKTOP_AGENT_ID").ok());
            let mut context = context
                .with_agent_id(agent_id)
                .map_err(pre_dispatch_error)?;
            if let Some(deadline) = deadline {
                context = context.with_inherited_deadline(deadline);
            }
            context = context.with_event_baseline(baseline.map(Ok));
            if let Some(wait) = wait_selector.as_ref() {
                validate_wait_for_command(&cmd, wait)?;
            }
            execute_with_adapter(cmd, &context, &cli.visual_debug, deadline)
        }
    }
}

fn build_wait_selector(cli: &Cli) -> Result<Option<WaitSelector>, AppError> {
    let query = cli
        .post_action_wait
        .wait_for
        .as_ref()
        .map(|raw| (raw, false))
        .or_else(|| {
            cli.post_action_wait
                .wait_for_gone
                .as_ref()
                .map(|raw| (raw, true))
        });
    let Some((query_raw, gone)) = query else {
        if cli.post_action_wait.wait_timeout.is_some() {
            return Err(AppError::invalid_input_with_suggestion(
                "--wait-timeout requires --wait-for or --wait-for-gone",
                "Add a selector wait flag or remove --wait-timeout",
            ));
        }
        return Ok(None);
    };
    Ok(Some(WaitSelector {
        query_raw: query_raw.clone(),
        gone,
        timeout_ms: cli.post_action_wait.wait_timeout.unwrap_or(30_000),
    }))
}

fn validate_wait_for_command(cmd: &Commands, wait: &WaitSelector) -> Result<(), AppError> {
    if !cmd.supports_post_action_wait() {
        return Err(AppError::invalid_input_with_suggestion(
            format!(
                "Command '{}' does not support --wait-for or --wait-for-gone",
                cmd.name()
            ),
            "Use snapshot --wait-for \"<selector>\" or a supported ref action (click, type, …).",
        ));
    }
    acdesktop_core::commands::query::validate_selector(&wait.query_raw)?;
    Ok(())
}

fn execute_with_adapter(
    mut cmd: Commands,
    context: &CommandContext,
    debug: &visual_debug::options::DebugOptions,
    deadline: Option<acdesktop_core::Deadline>,
) -> Result<serde_json::Value, AppError> {
    #[cfg(target_os = "windows")]
    if let Err(bootstrap_error) = acdesktop_windows::ensure_owned_process_mta_and_dpi() {
        return Err(pre_dispatch_error(bootstrap_error.into()));
    }

    let adapter = build_adapter();
    let adapter: &dyn acdesktop_core::PlatformAdapter = &adapter;
    let report = if command_policy::requires_permission_report(&cmd) {
        match deadline
            .map_or_else(acdesktop_core::Deadline::standard, Ok)
            .map_err(AppError::from)
            .and_then(|deadline| adapter.permission_report(deadline).map_err(AppError::from))
        {
            Ok(report) => report,
            Err(err) => return Err(pre_dispatch_error(err)),
        }
    } else {
        acdesktop_core::PermissionReport::default()
    };
    command_policy::preflight(&cmd, &report)?;
    command_policy::preflight_context(&cmd, context)?;

    if debug.debug && report.screen_recording_denied() {
        return Err(pre_dispatch_error(
            AdapterError::new(
                ErrorCode::PermDenied,
                "Visual debug requires Screen Recording permission",
            )
            .with_suggestion(
                report
                    .screen_recording_suggestion()
                    .unwrap_or("Grant Screen Recording permission and retry"),
            )
            .into(),
        ));
    }
    let capture = match visual_debug::DebugCapture::prepare(&mut cmd, debug, adapter, context) {
        Ok(capture) => capture,
        Err(error) => return Err(pre_dispatch_error(error)),
    };
    if let Some(deadline) = deadline.filter(|deadline| deadline.is_expired()) {
        return Err(pre_dispatch_error(deadline.timeout_error().into()));
    }
    let mut result = dispatch::dispatch(cmd, adapter, &report, context);
    if let Some(capture) = capture {
        capture.finish(&mut result, adapter, context);
    }
    result
}

fn pre_dispatch_error(error: AppError) -> AppError {
    match error {
        AppError::Adapter(mut source) => {
            source.disposition = DeliverySemantics::not_delivered();
            source.into()
        }
        other => AdapterError::new(ErrorCode::Internal, other.to_string())
            .with_disposition(DeliverySemantics::not_delivered())
            .into(),
    }
}

fn handle_update(args: &cli_args::update::UpdateArgs) -> Result<serde_json::Value, AppError> {
    use cli_args::update::{AutoAction, UpdateAction};
    use config::UserConfig;
    use serde_json::json;
    match &args.action {
        UpdateAction::Check => {
            let r = updater::check_update(false);
            Ok(serde_json::to_value(&r).map_err(|e| AppError::invalid_input(e.to_string()))?)
        }
        UpdateAction::Apply => {
            let r = updater::apply_update();
            Ok(serde_json::to_value(&r).map_err(|e| AppError::invalid_input(e.to_string()))?)
        }
        UpdateAction::Auto(a) => match a {
            AutoAction::Status => {
                let cfg = UserConfig::load();
                Ok(json!({
                    "auto_update": cfg.auto_update,
                    "effective": cfg.auto_update_enabled(),
                    "last_check": cfg.last_version_check,
                    "check_interval_secs": cfg.check_interval_secs,
                }))
            }
            AutoAction::On => {
                let mut cfg = UserConfig::load();
                cfg.auto_update = true;
                cfg.save().map_err(|e| AppError::invalid_input(e.to_string()))?;
                Ok(json!({ "ok": true, "auto_update": true }))
            }
            AutoAction::Off => {
                let mut cfg = UserConfig::load();
                cfg.auto_update = false;
                cfg.save().map_err(|e| AppError::invalid_input(e.to_string()))?;
                Ok(json!({ "ok": true, "auto_update": false }))
            }
        },
    }
}

fn handle_config(args: &cli_args::config::ConfigArgs) -> Result<serde_json::Value, AppError> {
    use cli_args::config::ConfigAction;
    use config::UserConfig;
    use serde_json::{Value, json};
    match &args.action {
        ConfigAction::Show => {
            let cfg = UserConfig::load();
            serde_json::to_value(&cfg).map_err(|e| AppError::invalid_input(e.to_string()))
        }
        ConfigAction::Path => Ok(json!({ "path": config::config_path().to_string_lossy() })),
        ConfigAction::Set { key, value } => {
            let mut cfg = UserConfig::load();
            let v: Value = serde_json::from_str(value)
                .unwrap_or_else(|_| Value::String(value.clone()));
            set_field(&mut cfg, key, v)?;
            cfg.save().map_err(|e| AppError::invalid_input(e.to_string()))?;
            serde_json::to_value(&cfg).map_err(|e| AppError::invalid_input(e.to_string()))
        }
    }
}

fn set_field(cfg: &mut config::UserConfig, key: &str, value: serde_json::Value) -> Result<(), AppError> {
    let err = |msg: &str| AppError::invalid_input(msg.to_string());
    match key {
        "auto_update" | "auto-update" => {
            cfg.auto_update = value.as_bool().ok_or_else(|| err("auto_update 需要布尔值 true/false"))?;
        }
        "fast_mode" | "fast-mode" => {
            cfg.fast_mode = value.as_bool().ok_or_else(|| err("fast_mode 需要布尔值 true/false"))?;
        }
        "check_interval_secs" | "check-interval-secs" => {
            cfg.check_interval_secs = value.as_u64().ok_or_else(|| err("check_interval_secs 需要正整数"))?;
        }
        "last_version_check" | "last-version-check" => {
            cfg.last_version_check = value.as_u64().ok_or_else(|| err("last_version_check 需要正整数"))?;
        }
        other => return Err(err(&format!("未知配置字段: {other}（支持 auto_update / fast_mode / check_interval_secs / last_version_check）"))),
    }
    Ok(())
}

fn finish(cmd_name: &str, result: Result<serde_json::Value, AppError>) -> ExitCode {
    let response = response(cmd_name, result);
    if let Err(write_err) = emit_response(&response) {
        return report_output_failure(write_err);
    }
    if response.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn response(cmd_name: &str, result: Result<serde_json::Value, AppError>) -> Response {
    match result {
        Ok(data) => Response::ok(cmd_name, data),
        Err(error) => Response::err(cmd_name, ErrorPayload::from_app_error(&error)),
    }
}

fn report_output_failure(write_err: std::io::Error) -> ExitCode {
    eprintln!("acdesktop: failed to write response to stdout: {write_err}");
    ExitCode::FAILURE
}

fn emit_response(response: &impl serde::Serialize) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    serde_json::to_writer(&mut writer, response).map_err(std::io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn build_adapter() -> impl acdesktop_core::PlatformAdapter {
    #[cfg(target_os = "macos")]
    {
        acdesktop_macos::MacOSAdapter::new()
    }

    #[cfg(target_os = "windows")]
    {
        acdesktop_windows::WindowsAdapter::new()
    }

    #[cfg(target_os = "linux")]
    {
        acdesktop_linux::release_input_on_signals();
        acdesktop_linux::LinuxAdapter::new()
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    compile_error!("Unsupported platform")
}

fn init_tracing(verbose: bool) {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter = if verbose { "debug" } else { "warn" };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
}

#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod main_tests;
