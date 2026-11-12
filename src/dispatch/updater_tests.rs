use super::*;

#[test]
fn stable_release_tag_validation_rejects_shell_metacharacters_and_invalid_versions() {
    assert!(valid_tag("v1.2.3"));
    for invalid in [
        "1.2.3",
        "v01.2.3",
        "v1.2",
        "v1.2.3;exit",
        "v1.2.3-rc1",
        "v1.2.3/extra",
    ] {
        assert!(!valid_tag(invalid), "{invalid}");
    }
}

#[test]
fn version_comparison_orders_stable_releases() {
    assert!(is_newer("v1.2.4", "1.2.3"));
    assert!(!is_newer("v1.2.3", "1.2.3"));
    assert!(!is_newer("v1.2.2", "1.2.3"));
}

#[test]
fn check_result_reports_newer_and_current_versions_correctly() {
    assert!(matches!(
        evaluate_latest("v999999.0.0".into()),
        CheckResult::UpdateAvailable { .. }
    ));
    assert!(matches!(
        evaluate_latest(format!("v{CURRENT_VERSION}")),
        CheckResult::UpToDate { .. }
    ));
}
