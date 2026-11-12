use acdesktop_core::AppError;

use crate::{cli::config, cli_args, dispatch::updater};

pub(crate) fn handle_update(
    args: &cli_args::update::UpdateArgs,
) -> Result<serde_json::Value, AppError> {
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
                cfg.save()
                    .map_err(|e| AppError::invalid_input(e.to_string()))?;
                Ok(json!({ "ok": true, "auto_update": true }))
            }
            AutoAction::Off => {
                let mut cfg = UserConfig::load();
                cfg.auto_update = false;
                cfg.save()
                    .map_err(|e| AppError::invalid_input(e.to_string()))?;
                Ok(json!({ "ok": true, "auto_update": false }))
            }
        },
    }
}

pub(crate) fn handle_config(
    args: &cli_args::config::ConfigArgs,
) -> Result<serde_json::Value, AppError> {
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
            let v: Value =
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone()));
            set_field(&mut cfg, key, v)?;
            cfg.save()
                .map_err(|e| AppError::invalid_input(e.to_string()))?;
            serde_json::to_value(&cfg).map_err(|e| AppError::invalid_input(e.to_string()))
        }
    }
}

fn set_field(
    cfg: &mut config::UserConfig,
    key: &str,
    value: serde_json::Value,
) -> Result<(), AppError> {
    let err = |msg: &str| AppError::invalid_input(msg.to_string());
    match key {
        "auto_update" | "auto-update" => {
            cfg.auto_update = value
                .as_bool()
                .ok_or_else(|| err("auto_update 需要布尔值 true/false"))?;
        }
        "fast_mode" | "fast-mode" => {
            cfg.fast_mode = value
                .as_bool()
                .ok_or_else(|| err("fast_mode 需要布尔值 true/false"))?;
        }
        "check_interval_secs" | "check-interval-secs" => {
            cfg.check_interval_secs = value
                .as_u64()
                .ok_or_else(|| err("check_interval_secs 需要正整数"))?;
        }
        "last_version_check" | "last-version-check" => {
            cfg.last_version_check = value
                .as_u64()
                .ok_or_else(|| err("last_version_check 需要正整数"))?;
        }
        other => {
            return Err(err(&format!(
                "未知配置字段: {other}（支持 auto_update / fast_mode / check_interval_secs / last_version_check）"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "user_config_tests.rs"]
mod tests;
