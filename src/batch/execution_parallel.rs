//! Batch execution — 跨 app 并行 + 同 app 安全串行。
//!
//! # 安全模型
//!
//! - **Observation 命令**（snapshot/find/get/is/list-*）：纯读，跨 app 全并行
//! - **Action 命令**（click/type/drag/scroll/launch…）：跨 app 并行，**同 app 严格按原始顺序串行**
//! - **Global 命令**（press/key/down/up/mouse-*/clipboard-*）：全局单例资源，**等所有 bucket 结束后串行**
//! - **Illegal in parallel**：wait / cursor-overlay / sandbox / batch / session / trace —— 直接 INVALID_ARGS
//!
//! # 实现
//!
//! 零第三方依赖：`std::thread::scope`（1.63+）让所有 bucket 并行跑；
//! `PlatformAdapter: Send + Sync` 让 adapter 可以跨线程安全引用。

use acdesktop_core::{
    AppError, Deadline, DeliverySemantics, PermissionReport, PlatformAdapter,
    context::CommandContext,
};
use serde_json::{Value, json};

use crate::{cli::Commands, cli_args::batch::BatchArgs, dispatch};

use super::{
    result_entry::{MAX_BATCH_OUTPUT_BYTES, bounded_completed_entry, not_started_entry},
};

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
        Snapshot(_) | Find(_) | Screenshot(_) | Get(_) | Is(_) | ListWindows(_)
        | ListDisplays | ListApps(_) | ListSurfaces(_) | ListNotifications(_)
        | Status | Version | Permissions(_) => Class::Observation,

        Press(_) | KeyDown(_) | KeyUp(_)
        | MouseMove(_) | MouseClick(_) | MouseDown(_) | MouseUp(_) | MouseWheel(_)
        | ClipboardSet(_) | ClipboardClear | ClipboardGet(_) => Class::Global,

        Click(_) | DoubleClick(_) | TripleClick(_) | RightClick(_) | Type(_)
        | SetValue(_) | Clear(_) | Focus(_) | Select(_) | Toggle(_) | Check(_)
        | Uncheck(_) | Expand(_) | Collapse(_) | Scroll(_) | ScrollTo(_)
        | Hover(_) | Drag(_)
        | Launch(_) | CloseApp(_) | FocusWindow(_) | ResizeWindow(_) | MoveWindow(_)
        | Minimize(..) | Maximize(..) | Restore(..)
        | OpenSystemSurface(_)
        | DismissNotification(_) | DismissAllNotifications(..) | NotificationAction(_) => Class::Action,

        Wait(_) | CursorOverlay(_) | Batch(_)
        | Session(_) | Skills(_) | Trace(_) | Sandbox(_)
        | Update(_) | Config(_) => Class::Illegal,
    }
}

/// 从命令里提取"归属于哪个 target bucket"。没有明确 app 的命令默认进 `_ref_only` 或 `_global`。
fn target_key(cmd: &Commands) -> String {
    use Commands::*;
    match cmd {
        // app 是 String（必填）
        Launch(a) => a.app.clone(),
        CloseApp(a) => a.app.clone(),
        // app 是 Option<String>
        FocusWindow(a) => a.app.clone().unwrap_or_else(|| "_global".to_string()),
        ResizeWindow(a) => a.scope.app.clone().unwrap_or_else(|| "_unknown".to_string()),
        MoveWindow(a) => a.scope.app.clone().unwrap_or_else(|| "_unknown".to_string()),
        // RefArgs 没有 app 字段，只靠 ref_id —— 无法判断属于哪个 app，保守进 ref_only 桶
        Click(_) | DoubleClick(_) | TripleClick(_) | RightClick(_)
        | Clear(_) | Focus(_) | Toggle(_) | Check(_) | Uncheck(_)
        | Expand(_) | Collapse(_) | ScrollTo(_)
        | Type(_) | SetValue(_) | Select(_) | Scroll(_) | Hover(_) | Drag(_)
        => "_ref_only".to_string(),
        _ => "_global".to_string(),
    }
}

pub(super) fn execute_parallel(
    args: BatchArgs,
    adapter: &dyn PlatformAdapter,
    permission_report: &PermissionReport,
    context: &CommandContext,
) -> Result<Value, AppError> {
    let deadline = Deadline::after(args.timeout_ms)
        .map_err(|e| e.with_disposition(DeliverySemantics::not_delivered()))?;
    let batch_context = context.clone().with_inherited_deadline(deadline);

    let mut prepared =
        super::preparation::prepare(&args.commands_json, permission_report, &batch_context)?;
    let total = prepared.len();

    // --- 前置安全检查 ---
    for (i, p) in prepared.iter().enumerate() {
        if classify(&p.command) == Class::Illegal {
            return Err(AppError::invalid_input(format!(
                "--parallel batch 不支持 '{}'（序号 {}）。需要串行执行的命令请放到普通 batch 里。",
                p.name, i
            )));
        }
    }

    // --- 分桶：move Commands 进桶，避免 Clone ---
    // buckets: String key → Vec<(original_index, command, class)>
    // Commands 不可 Clone，所以必须 move。
    let mut buckets: std::collections::BTreeMap<String, Vec<(usize, Commands, Class)>> =
        std::collections::BTreeMap::new();
    for (i, p) in prepared.iter_mut().enumerate() {
        let cmd = std::mem::replace(&mut p.command, Commands::Version);
        let cls = classify(&cmd);
        let key = match cls {
            Class::Global => "_global".to_string(),
            _ => target_key(&cmd),
        };
        buckets.entry(key).or_default().push((i, cmd, cls));
    }

    // 分 non_global vs global（global 最后串行）
    let mut non_global_buckets = Vec::new();
    let mut global_bucket: Option<(String, Vec<(usize, Commands, Class)>)> = None;
    for (k, v) in buckets {
        if k == "_global" {
            global_bucket = Some((k, v));
        } else {
            non_global_buckets.push((k, v));
        }
    }

    let all_results = std::sync::Mutex::new(vec![None; total]);
    let elapsed = std::time::Instant::now();
    let max_workers = args.max_workers.max(1).min(non_global_buckets.len().max(1));

    // --- 跨桶并行：所有 non-global bucket 一起跑，由 OS 线程调度 ---
    // thread::scope 自动等待所有 spawn 完成后再退出。
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for (bucket_key, bucket_cmds) in non_global_buckets {
            let ctx = batch_context.clone();
            let results_ref = &all_results;
            let deadline_ref = deadline;
            let perm_ref = permission_report;
            let adapter_ref = adapter;

            handles.push(s.spawn(move || {
                run_bucket_owned(
                    &bucket_key,
                    bucket_cmds,
                    adapter_ref,
                    perm_ref,
                    &ctx,
                    &deadline_ref,
                    results_ref,
                )
            }));
        }
        for h in handles {
            let _ = h.join();
        }
    });

    // --- global bucket 串行（最后跑，不和其他桶抢输入设备） ---
    if let Some((key, cmds)) = global_bucket {
        run_bucket_owned(
            &key,
            cmds,
            adapter,
            permission_report,
            &batch_context,
            &deadline,
            &all_results,
        );
    }

    // --- 组装输出 ---
    let mut results: Vec<Value> = Vec::with_capacity(total);
    let mut used = 0usize;
    let mut completed = 0usize;
    let mut oversized = false;

    for i in 0..total {
        let entry = all_results.lock().unwrap()[i]
            .take()
            .unwrap_or_else(|| {
                not_started_entry(
                    i,
                    prepared[i].name.as_str(),
                    "parallel_bucket_aborted",
                    AppError::invalid_input(format!("命令 {i} 因桶级 abort 未执行")),
                )
            });
        if entry.get("execution").and_then(|v| v.as_str()) == Some("completed") {
            completed += 1;
        }
        let (entry, oversized_now) =
            bounded_completed_entry(i, prepared[i].name.as_str(), entry, used);
        if oversized_now {
            oversized = true;
        }
        let size = super::bounded_json::serialized_size(&entry).saturating_add(1);
        used = used.saturating_add(size);
        results.push(entry);
    }

    let mut body = json!({
        "results": results,
        "semantics": {
            "atomic": false,
            "order": "target-group-parallel",
            "parallel_by_target": true,
            "cross_target_parallelism": max_workers,
            "batch_retries": false,
        },
        "total_entries": total,
        "completed_entries": completed,
        "not_started_entries": total.saturating_sub(completed),
        "timeout_ms": args.timeout_ms,
        "elapsed_ms": elapsed.elapsed().as_millis(),
        "limits": {
            "max_entries": super::preparation::MAX_BATCH_ENTRIES,
            "max_input_bytes": super::preparation::MAX_BATCH_JSON_BYTES,
            "max_output_bytes": MAX_BATCH_OUTPUT_BYTES,
        },
    });
    if oversized {
        body["stopped"] = json!({ "reason": "output_limit" });
    }

    Ok(body)
}

/// 桶内执行：按原始顺序串行处理每个 command（action 类同桶串行防止竞态）
fn run_bucket_owned(
    bucket_key: &str,
    cmds: Vec<(usize, Commands, Class)>,
    adapter: &dyn PlatformAdapter,
    permission_report: &PermissionReport,
    batch_context: &CommandContext,
    deadline: &Deadline,
    results: &std::sync::Mutex<Vec<Option<Value>>>,
) {
    for (index, cmd, _class) in cmds {
        if deadline.is_expired() {
            let error = AppError::invalid_input(format!("并行 bucket '{bucket_key}' 超时"));
            let entry = not_started_entry(index, bucket_key, "deadline", error);
            results.lock().unwrap()[index] = Some(entry);
            break;
        }
        let item_context = batch_context.clone().with_inherited_deadline(*deadline);
        let execution = dispatch::dispatch(cmd, adapter, permission_report, &item_context);
        let entry = super::result_entry::completed_entry(index, bucket_key, execution);
        results.lock().unwrap()[index] = Some(entry);
    }
}
