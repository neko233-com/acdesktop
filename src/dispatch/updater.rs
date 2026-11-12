//! 版本检查 + 自更新
//!
//! 设计原则：
//! - 不引入 reqwest 这类重型 HTTP 依赖，直接 `exec curl`（Windows 自带 curl.exe，macOS 自带 curl）
//! - `update check` 对比本地版本与 GitHub Release 最新 tag
//! - `update apply` 直接 `exec` 安装脚本（install.ps1 / install.sh），
//!   让安装脚本做原子替换；当前进程退出，下次启动即新版本
//! - 幂等：`auto_update` 可关闭；检查结果缓存 6 小时

use std::process::Command;

use crate::cli::config::{REPO, UserConfig, update_cache_path};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 命令行入口：`acdesktop update check`
pub(crate) fn check_update(_verbose: bool) -> CheckResult {
    let latest = match fetch_latest_tag() {
        Ok(t) => t,
        Err(e) => return CheckResult::UpdateError { message: e },
    };
    evaluate_latest(latest)
}

fn evaluate_latest(latest: String) -> CheckResult {
    if is_newer(&latest, CURRENT_VERSION) {
        CheckResult::UpdateAvailable {
            current: CURRENT_VERSION.to_string(),
            latest,
        }
    } else {
        CheckResult::UpToDate {
            current: CURRENT_VERSION.to_string(),
        }
    }
}

/// 命令行入口：`acdesktop update apply` — 触发 install 脚本。
///
/// macOS 直接拉 install.sh；Windows 拉 install.ps1。
pub(crate) fn apply_update() -> ApplyResult {
    let latest = match fetch_latest_tag() {
        Ok(t) => t,
        Err(e) => {
            return ApplyResult::ApplyError {
                message: format!("检查失败: {e}"),
            };
        }
    };

    let url = install_script_url(&latest);
    let exit_code = run_install(&url, &latest);
    if exit_code == 0 {
        ApplyResult::Applied {
            target: latest,
            exit_code,
        }
    } else {
        ApplyResult::ApplyError {
            message: format!("安装失败 (exit {exit_code})"),
        }
    }
}

/// 后台静默检查（每次调用 `acdesktop` 时跑一次）
pub(crate) fn maybe_silent_check() -> Option<UpdateNotice> {
    let cfg = UserConfig::load();
    if !cfg.auto_update_enabled() {
        return None;
    }
    let now = epoch_secs();
    if now < cfg.last_version_check + cfg.check_interval_secs {
        return None;
    }
    let mut next = cfg.clone();
    next.last_version_check = now;
    next.save().ok()?;

    let latest = fetch_latest_tag().ok()?;
    if is_newer(&latest, CURRENT_VERSION) {
        let notice = UpdateNotice {
            current: CURRENT_VERSION.to_string(),
            latest,
        };
        let cache = serde_json::to_string(&notice).ok()?;
        let _ = acdesktop_core::write_private_atomic(&update_cache_path(), cache.as_bytes());
        Some(notice)
    } else {
        None
    }
}

fn fetch_latest_tag() -> Result<String, String> {
    let out = Command::new(curl_bin())
        .args([
            "-fsSLI",
            "-o",
            if cfg!(target_os = "windows") {
                "NUL"
            } else {
                "/dev/null"
            },
            "-w",
            "%{url_effective}",
            "--max-time",
            "10",
            &format!("https://github.com/{REPO}/releases/latest"),
        ])
        .output()
        .map_err(|e| format!("curl 启动失败: {e}"))?;
    if !out.status.success() {
        return Err(format!("curl 失败 (exit {:?})", out.status.code()));
    }
    let url = String::from_utf8_lossy(&out.stdout);
    let tag = url
        .trim()
        .rsplit_once('/')
        .map(|(_, t)| t.to_string())
        .ok_or_else(|| "无法从重定向 URL 解析 tag".to_string())?;
    if !valid_tag(&tag) {
        return Err("Release tag is not a stable semantic version".to_string());
    }
    Ok(tag)
}

fn valid_tag(tag: &str) -> bool {
    let Some(version) = tag.strip_prefix('v') else {
        return false;
    };
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
                && part.parse::<u64>().is_ok()
        })
}

fn curl_bin() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "curl.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "curl"
    }
}

fn is_newer(latest_tag: &str, current: &str) -> bool {
    let latest = strip_v(latest_tag);
    current.trim() != latest && version_tuple(current) < version_tuple(latest)
}

fn strip_v(s: &str) -> &str {
    s.strip_prefix('v').unwrap_or(s)
}

fn version_tuple(v: &str) -> (u64, u64, u64) {
    let mut it = v.split('.');
    let a = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let b = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let c = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (a, b, c)
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn install_script_url(tag: &str) -> String {
    let file = if cfg!(target_os = "windows") {
        "install.ps1"
    } else {
        "install.sh"
    };
    format!(
        "https://raw.githubusercontent.com/{REPO}/{}/scripts/{file}",
        if tag.starts_with('v') {
            tag
        } else {
            &format!("v{tag}")
        }
    )
}

fn run_install(url: &str, tag: &str) -> i32 {
    if cfg!(target_os = "windows") {
        let out = Command::new("pwsh")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!("$ProgressPreference='SilentlyContinue'; & ([scriptblock]::Create((irm '{url}'))) -Version '{tag}'"),
            ])
            .status();
        out.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
    } else {
        let out = Command::new("bash")
            .arg("-o")
            .arg("pipefail")
            .arg("-c")
            .arg(format!("curl -fsSL {url} | bash -s -- --version {tag}"))
            .status();
        out.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(crate) enum CheckResult {
    UpToDate {
        current: String,
    },
    UpdateAvailable {
        current: String,
        latest: String,
    },
    #[serde(rename = "error")]
    UpdateError {
        message: String,
    },
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(crate) enum ApplyResult {
    Applied {
        target: String,
        exit_code: i32,
    },
    #[serde(rename = "error")]
    ApplyError {
        message: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct UpdateNotice {
    pub current: String,
    pub latest: String,
}

#[cfg(test)]
#[path = "updater_tests.rs"]
mod tests;
