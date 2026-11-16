use super::*;
use crate::system::cursor_overlay::hosted_watch::HostedIndicatorWatch;

#[test]
fn dropping_a_hosted_lease_signals_its_watcher() {
    unsafe {
        windows_sys::Win32::Foundation::SetLastError(
            windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS,
        )
    };
    let lease = HostedIndicatorLease::new().unwrap();
    let watch = HostedIndicatorWatch::open(lease.owner()).unwrap();
    assert!(!watch.finished());
    drop(lease);
    assert!(watch.finished());
}

#[test]
fn concurrently_live_hosted_leases_have_distinct_valid_owners() {
    let first = HostedIndicatorLease::new().unwrap();
    let second = HostedIndicatorLease::new().unwrap();
    assert_ne!(first.owner(), second.owner());
    acdesktop_core::context::validate_session_id(first.owner()).unwrap();
    acdesktop_core::context::validate_session_id(second.owner()).unwrap();
}
