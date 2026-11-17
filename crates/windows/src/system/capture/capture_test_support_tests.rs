use super::HomeIsolation;

#[test]
fn isolated_home_redirects_and_restores_an_existing_state_root() {
    const MARKER: &str = "ACDESKTOP_TEST_ISOLATED_STATE_ROOT";
    if std::env::var_os(MARKER).is_some() {
        let previous = std::env::var_os("ACDESKTOP_HOME").expect("state root supplied by parent");
        {
            let isolated = HomeIsolation::enter("acdesktop-test-state-isolation");
            assert_eq!(
                std::env::var_os("ACDESKTOP_HOME"),
                Some(isolated.path().join(".acdesktop").into_os_string())
            );
        }
        assert_eq!(std::env::var_os("ACDESKTOP_HOME"), Some(previous));
        return;
    }
    let state =
        std::env::temp_dir().join(format!("acdesktop-state-sentinel-{}", std::process::id()));
    let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("system::capture::capture_test_support::tests::isolated_home_redirects_and_restores_an_existing_state_root")
        .arg("--nocapture")
        .env(MARKER, "1")
        .env("ACDESKTOP_HOME", state)
        .status()
        .expect("isolated test process");
    assert!(status.success(), "isolated test process failed");
}
