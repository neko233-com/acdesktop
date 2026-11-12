use super::{environment_has_marker, process_start_time};

const MARKER: &str = "ACDESKTOP_INTERNAL_COMMAND_HOST=1";

fn procargs(arguments: &[&str], environment: &[&str]) -> Vec<u8> {
    let mut buffer = (arguments.len() as i32).to_ne_bytes().to_vec();
    let mut path = b"/usr/local/bin/acdesktop\0".to_vec();
    path.resize(path.len().next_multiple_of(8), 0);
    buffer.extend_from_slice(&path);
    for field in arguments.iter().chain(environment) {
        buffer.extend_from_slice(field.as_bytes());
        buffer.push(0);
    }
    buffer
}

#[test]
fn the_marker_counts_only_as_an_exact_environment_entry() {
    assert!(environment_has_marker(&procargs(
        &["acdesktop", ""],
        &["HOME=/Users/a", MARKER]
    )));
    assert!(!environment_has_marker(&procargs(
        &["acdesktop", MARKER],
        &["HOME=/Users/a"]
    )));
    for entry in [
        "ACDESKTOP_INTERNAL_COMMAND_HOST=0",
        "ACDESKTOP_INTERNAL_COMMAND_HOST=1suffix",
        "OTHER=ACDESKTOP_INTERNAL_COMMAND_HOST=1",
    ] {
        assert!(!environment_has_marker(&procargs(&["acdesktop"], &[entry])));
    }
}

#[test]
fn an_empty_first_argument_keeps_the_environment_aligned() {
    assert!(environment_has_marker(&procargs(&["", "x"], &[MARKER])));
    assert!(!environment_has_marker(&procargs(
        &["", MARKER],
        &["HOME=/Users/a"]
    )));
}

#[test]
fn malformed_argument_blocks_fail_closed() {
    assert!(!environment_has_marker(&[1, 0]));
    for count in [0, -1, i32::MAX] {
        let mut buffer = procargs(&["acdesktop"], &[MARKER]);
        buffer[..4].copy_from_slice(&count.to_ne_bytes());
        assert!(!environment_has_marker(&buffer));
    }
    let mut truncated = procargs(&["acdesktop"], &[MARKER]);
    truncated.pop();
    assert!(!environment_has_marker(&truncated));
}

#[test]
fn process_start_time_requires_the_process_owner() {
    let uid = unsafe { libc::geteuid() };
    let pid = std::process::id() as libc::pid_t;
    assert!(process_start_time(pid, uid).is_some());
    assert!(process_start_time(pid, uid ^ 1).is_none());
    if uid != 0 {
        assert!(process_start_time(1, uid).is_none());
    }
}
