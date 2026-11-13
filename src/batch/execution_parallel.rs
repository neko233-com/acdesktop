//! Batch execution — 跨 app 并行 + 同 app 安全串行。
//!
//! # 安全模型
//!
//! - **Observation 命令**（snapshot/find/get/is/list-*）：纯读，跨 app 全并行
//! - Semantic ref actions can run across verified process identities; other actions run as barriers.
//! - Global operations run between the entries that precede and follow them.
//! - **Illegal in parallel**：wait / cursor-overlay / sandbox / batch / session / trace —— 直接 INVALID_ARGS
//!
//! # 实现
//!
//! 零第三方依赖：`std::thread::scope`让所有 bucket 并行跑；
//! `PlatformAdapter: Send + Sync` 让 adapter 可以跨线程安全引用。

use acdesktop_core::{
    AppError, Deadline, DeliverySemantics, PermissionReport, PlatformAdapter,
    context::CommandContext,
};
use serde_json::{Value, json};

use crate::{cli::Commands, cli_args::batch::BatchArgs, dispatch};

use super::result_entry::{MAX_BATCH_OUTPUT_BYTES, bounded_completed_entry, not_started_entry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Observation,
    Action,
    Global,
    Illegal,
}

fn classify(cmd: &Commands) -> Class {
    use Commands::*;
    match cmd {
        Snapshot(_) | Find(_) | Screenshot(_) | Get(_) | Is(_) | ListWindows(_) | ListDisplays
        | ListApps(_) | ListSurfaces(_) | ListNotifications(_) | Status | Version
        | Permissions(_) => Class::Observation,

        Press(_) | KeyDown(_) | KeyUp(_) | MouseMove(_) | MouseClick(_) | MouseDown(_)
        | MouseUp(_) | MouseWheel(_) | ClipboardSet(_) | ClipboardClear | ClipboardGet(_) => {
            Class::Global
        }

        Click(_)
        | DoubleClick(_)
        | TripleClick(_)
        | RightClick(_)
        | Type(_)
        | SetValue(_)
        | Clear(_)
        | Focus(_)
        | Select(_)
        | Toggle(_)
        | Check(_)
        | Uncheck(_)
        | Expand(_)
        | Collapse(_)
        | Scroll(_)
        | ScrollTo(_)
        | Hover(_)
        | Drag(_)
        | Launch(_)
        | CloseApp(_)
        | FocusWindow(_)
        | ResizeWindow(_)
        | MoveWindow(_)
        | Minimize(..)
        | Maximize(..)
        | Restore(..)
        | OpenSystemSurface(_)
        | DismissNotification(_)
        | DismissAllNotifications(..)
        | NotificationAction(_) => Class::Action,

        Wait(_) | CursorOverlay(_) | Batch(_) | Session(_) | Skills(_) | Trace(_) | Sandbox(_)
        | Game(_) | Update(_) | Config(_) => Class::Illegal,
    }
}

pub(super) fn execute_parallel(
    args: BatchArgs,
    adapter: &dyn PlatformAdapter,
    permission_report: &PermissionReport,
    context: &CommandContext,
) -> Result<Value, AppError> {
    if args.stop_on_error {
        return super::execution::execute(args, adapter, permission_report, context);
    }
    let deadline = Deadline::after(args.timeout_ms)
        .map_err(|e| e.with_disposition(DeliverySemantics::not_delivered()))?;
    let batch_context = context.clone().with_inherited_deadline(deadline);

    let prepared =
        super::preparation::prepare(&args.commands_json, permission_report, &batch_context)?;
    let total = prepared.len();

    for (i, p) in prepared.iter().enumerate() {
        if classify(&p.command) == Class::Illegal {
            return Err(AppError::invalid_input(format!(
                "--parallel batch 不支持 '{}'（序号 {}）。需要串行执行的命令请放到普通 batch 里。",
                p.name, i
            )));
        }
    }

    let names: Vec<String> = prepared.iter().map(|item| item.name.clone()).collect();
    let segments = plan_segments(prepared, super::parallel_identity::process_key);

    let all_results = std::sync::Mutex::new(vec![None; total]);
    let max_workers = args.max_workers.max(1).min(total.max(1));
    let parallelism = segments
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .min(max_workers);
    let worker_context = WorkerContext {
        adapter,
        permission_report,
        deadline,
        results: &all_results,
    };
    let mut results = Vec::with_capacity(total);
    let mut used = 0usize;
    let mut completed = 0usize;
    let mut stopped = None;
    for segment in segments {
        let mut indices: Vec<usize> = segment
            .iter()
            .flat_map(|bucket| bucket.iter().map(|(index, _)| *index))
            .collect();
        indices.sort_unstable();
        let mut buckets: std::collections::VecDeque<_> = segment.into();
        while !buckets.is_empty() {
            let joined = std::thread::scope(|scope| {
                let mut handles = Vec::new();
                let mut spawned = true;
                for _ in 0..max_workers {
                    let Some(commands) = buckets.pop_front() else {
                        break;
                    };
                    let worker = &worker_context;
                    match std::thread::Builder::new()
                        .spawn_scoped(scope, move || run_bucket_owned(commands, worker))
                    {
                        Ok(handle) => handles.push(handle),
                        Err(_) => {
                            spawned = false;
                            break;
                        }
                    }
                }
                handles
                    .into_iter()
                    .fold(spawned, |ok, handle| handle.join().is_ok() && ok)
            });
            if !joined {
                return Err(acdesktop_core::AdapterError::new(
                    acdesktop_core::ErrorCode::Internal,
                    "Parallel batch worker failed; delivery of in-flight commands is uncertain",
                )
                .with_disposition(DeliverySemantics::uncertain())
                .into());
            }
        }
        let mut stored = all_results
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut oversized = false;
        for i in indices {
            let Some(entry) = stored[i].take() else {
                return Err(acdesktop_core::AdapterError::new(
                    acdesktop_core::ErrorCode::Internal,
                    "Parallel batch lost a command result",
                )
                .with_disposition(DeliverySemantics::uncertain())
                .into());
            };
            if entry.get("execution").and_then(|v| v.as_str()) == Some("completed") {
                completed += 1;
            }
            let (entry, limit_hit) = bounded_completed_entry(i, &names[i], entry, used);
            oversized |= limit_hit;
            used =
                used.saturating_add(super::bounded_json::serialized_size(&entry).saturating_add(1));
            results.push(entry);
        }
        if oversized {
            stopped = Some(
                json!({ "reason": "output_limit", "after_index": results.len().saturating_sub(1) }),
            );
            break;
        }
    }

    let mut body = json!({
        "results": results,
        "semantics": {
            "atomic": false,
            "order": "target-group-parallel",
            "parallel_by_target": true,
            "cross_target_parallelism": parallelism,
            "batch_retries": false,
        },
        "total_entries": total,
        "completed_entries": completed,
        "not_started_entries": total.saturating_sub(completed),
        "timeout_ms": args.timeout_ms,
        "elapsed_ms": deadline.elapsed().as_millis(),
        "limits": {
            "max_entries": super::preparation::MAX_BATCH_ENTRIES,
            "max_input_bytes": super::preparation::MAX_BATCH_JSON_BYTES,
            "max_output_bytes": MAX_BATCH_OUTPUT_BYTES,
        },
    });
    if let Some(stopped) = stopped {
        body["stopped"] = stopped;
    }
    if super::bounded_json::serialized_size(&body) > MAX_BATCH_OUTPUT_BYTES {
        let disposition = if completed == 0 {
            DeliverySemantics::not_delivered()
        } else {
            DeliverySemantics::uncertain()
        };
        return Err(acdesktop_core::AdapterError::new(
            acdesktop_core::ErrorCode::Internal,
            "Batch response exceeded its output contract after final serialization",
        )
        .with_disposition(disposition)
        .into());
    }
    Ok(body)
}

type IndexedCommand = (usize, super::preparation::PreparedCommand);
type Segments = Vec<Vec<Vec<IndexedCommand>>>;

fn plan_segments(
    prepared: Vec<super::preparation::PreparedCommand>,
    key_for: impl Fn(&super::preparation::PreparedCommand) -> Option<String>,
) -> Segments {
    let mut segments = Vec::new();
    let mut buckets = std::collections::BTreeMap::<String, Vec<_>>::new();
    let mut prior_class = None;
    for (index, item) in prepared.into_iter().enumerate() {
        let class = classify(&item.command);
        let key = match class {
            Class::Observation if !matches!(&item.command, Commands::Screenshot(_)) => {
                Some(format!("read:{index}"))
            }
            Class::Action => key_for(&item).map(|key| format!("process:{key}")),
            _ => None,
        };
        if key.is_none() || prior_class.is_some_and(|prior| prior != class) {
            if !buckets.is_empty() {
                segments.push(
                    std::mem::take(&mut buckets)
                        .into_values()
                        .collect::<Vec<_>>(),
                );
            }
            prior_class = None;
        }
        if let Some(key) = key {
            buckets.entry(key).or_default().push((index, item));
            prior_class = Some(class);
        } else {
            segments.push(vec![vec![(index, item)]]);
        }
    }
    if !buckets.is_empty() {
        segments.push(buckets.into_values().collect::<Vec<_>>());
    }

    segments
}

struct WorkerContext<'a> {
    adapter: &'a dyn PlatformAdapter,
    permission_report: &'a PermissionReport,
    deadline: Deadline,
    results: &'a std::sync::Mutex<Vec<Option<Value>>>,
}

fn run_bucket_owned(
    commands: Vec<(usize, super::preparation::PreparedCommand)>,
    worker: &WorkerContext<'_>,
) {
    for (index, item) in commands {
        let entry = if worker.deadline.is_expired() {
            let error = acdesktop_core::AdapterError::timeout(
                "Batch deadline elapsed before the entry started",
            )
            .with_disposition(DeliverySemantics::not_delivered());
            not_started_entry(index, &item.name, "deadline", error.into())
        } else if let Some(session) = super::execution::session_ended_for(&item) {
            let error = super::execution::batch_session_ended(index, &item.name, &session);
            not_started_entry(index, &item.name, "session_ended", error)
        } else {
            let item_context = item.context.with_inherited_deadline(worker.deadline);
            let result = dispatch::dispatch(
                item.command,
                worker.adapter,
                worker.permission_report,
                &item_context,
            );
            super::result_entry::completed_entry(index, &item.name, result)
        };
        worker
            .results
            .lock()
            .unwrap_or_else(|error| error.into_inner())[index] = Some(entry);
    }
}

#[cfg(test)]
#[path = "execution_parallel_tests.rs"]
mod tests;
