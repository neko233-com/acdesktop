//! 用户级配置：`~/.acdesktop/config.json`
//!
//! 字段都有默认值，文件缺失时自动写入一份。
//!
//! ```json
//! {
//!   "auto_update": true,
//!   "fast_mode": false,
//!   "last_version_check": 0
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserConfig {
    /// 是否在每次 `acdesktop` 调用时静默检查新版本（有更新会提示，但不自动安装）。
    #[serde(default = "default_true")]
    pub auto_update: bool,
    /// 是否启用极快模式：跳过 post_action_wait、减少内部等待、使用短超时时。
    #[serde(default)]
    pub fast_mode: bool,
    /// 上次检查版本的 unix epoch 秒。
    #[serde(default)]
    pub last_version_check: u64,
    /// 检查间隔（秒）。默认 6 小时。
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
}

fn default_true() -> bool {
    true
}
fn default_check_interval() -> u64 {
    6 * 60 * 60
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            auto_update: true,
            fast_mode: false,
            last_version_check: 0,
            check_interval_secs: default_check_interval(),
        }
    }
}

impl UserConfig {
    /// 加载；文件不存在时返回默认并在 save 时落盘。
    pub(crate) fn load() -> Self {
        let path = config_path();
        match acdesktop_core::read_private_bounded(&path, 64 * 1024) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                eprintln!("acdesktop: 配置文件解析失败，使用默认值 ({path:?}): {e}");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub(crate) fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        acdesktop_core::write_private_atomic(&path, text.as_bytes())?;
        Ok(())
    }

    /// 环境变量一键关闭自动更新，用于 CI / 脚本场景。
    pub(crate) fn auto_update_enabled(&self) -> bool {
        if std::env::var_os("ACDESKTOP_NO_AUTO_UPDATE").is_some() {
            return false;
        }
        self.auto_update
    }
}

pub(crate) fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ACDESKTOP_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    let home = dirs_home();
    home.join(".acdesktop")
}

pub(crate) fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn dirs_home() -> PathBuf {
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h);
    }
    if let Some(h) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(h);
    }
    PathBuf::from(".")
}

/// 仓库名占位（在 publish 阶段会替换成真实 owner）
pub(crate) const REPO: &str = "neko233-com/acdesktop";

/// 缓存更新检查结果（JSON）。
pub(crate) fn update_cache_path() -> PathBuf {
    config_dir().join("update-check.json")
}
