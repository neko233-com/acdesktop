use super::*;
use serde_json::json;

#[test]
fn configuration_rejects_unknown_fields_and_wrong_types() {
    let mut config = config::UserConfig::default();
    assert!(set_field(&mut config, "auto_update", json!("false")).is_err());
    assert!(set_field(&mut config, "unknown", json!(true)).is_err());
    assert!(config.auto_update);
}

#[test]
fn configuration_updates_supported_fields() {
    let mut config = config::UserConfig::default();
    set_field(&mut config, "auto-update", json!(false)).unwrap();
    set_field(&mut config, "fast_mode", json!(true)).unwrap();
    set_field(&mut config, "check_interval_secs", json!(42)).unwrap();
    assert!(!config.auto_update);
    assert!(config.fast_mode);
    assert_eq!(config.check_interval_secs, 42);
}
