#![cfg(target_os = "windows")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "windows_junction_fixture.rs"]
mod windows_junction_fixture;
use windows_junction_fixture::{entries_under, plant_junction};

#[path = "unique_scratch_dir.rs"]
mod unique_scratch_dir;
use unique_scratch_dir::unique_scratch_dir;

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn create(label: &str) -> Self {
        Self {
            root: unique_scratch_dir("install", label),
        }
    }

    fn dir(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::create_dir_all(&path).expect("create scratch subdirectory");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn run_session_start(home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_acdesktop"))
        .args(["session", "start"])
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("ACDESKTOP_HOME")
        .env_remove("ACDESKTOP_SESSION")
        .output()
        .expect("binary starts")
}

fn parse_envelope(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("stdout is one JSON envelope")
}

#[test]
fn session_start_through_a_junction_home_is_refused_by_the_installed_windows_ops() {
    let scratch = Scratch::create("junction");
    let home = scratch.dir("junction-home");
    let target = scratch.dir("junction-target");
    plant_junction(&home.join(".acdesktop"), &target);

    let output = run_session_start(&home);
    let envelope = parse_envelope(&output);

    assert_eq!(
        output.status.code(),
        Some(1),
        "session start must fail structurally when ~/.acdesktop is a junction; \
         success means the portable default wrote through the junction"
    );
    assert_eq!(envelope["ok"], false);
    let leaked = entries_under(&target);
    assert!(
        leaked.is_empty(),
        "no session artifact — file or directory — may land under the junction target: {leaked:?}"
    );
}

#[test]
fn session_start_in_a_real_home_succeeds_as_the_junction_control() {
    let scratch = Scratch::create("control");
    let home = scratch.dir("real-home");

    let output = run_session_start(&home);
    let envelope = parse_envelope(&output);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(envelope["ok"], true);
    let session_id = envelope["data"]["session_id"]
        .as_str()
        .expect("session start reports its session id");
    let manifest = home
        .join(".acdesktop")
        .join("sessions")
        .join(session_id)
        .join("session.json");
    assert!(
        manifest.is_file(),
        "the control session manifest must exist under the real home"
    );
}

#[test]
fn indicator_exits_when_its_home_directory_is_removed() {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };

    let scratch = Scratch::create("indicator-home-removal");
    let home = scratch.dir("home");
    let envelope = parse_envelope(&run_session_start(&home));
    assert_eq!(envelope["ok"], true, "{envelope}");
    let id = envelope["data"]["session_id"].as_str().unwrap();
    let query = format!(
        "Get-CimInstance Win32_Process -Filter \"Name='acdesktop.exe'\" | Where-Object {{ $_.CommandLine -like '*--control-indicator-child {id}*' }} | ForEach-Object {{ $_.ProcessId }}"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &query])
        .output()
        .unwrap();
    let pid: u32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("indicator process");
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    assert!(!handle.is_null());
    drop(scratch);
    let result = unsafe { WaitForSingleObject(handle, 6000) };
    unsafe { CloseHandle(handle) };
    assert_eq!(
        result, WAIT_OBJECT_0,
        "indicator must exit after its home is removed"
    );
}

#[test]
fn config_set_refuses_a_junction_without_writing_to_its_target() {
    let scratch = Scratch::create("config-junction");
    let home = scratch.dir("home");
    let target = scratch.dir("target");
    plant_junction(&home.join(".acdesktop"), &target);
    let output = Command::new(env!("CARGO_BIN_EXE_acdesktop"))
        .args(["config", "set", "auto_update", "false"])
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env_remove("ACDESKTOP_CONFIG_DIR")
        .output()
        .expect("binary starts");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(parse_envelope(&output)["ok"], false);
    assert!(entries_under(&target).is_empty());
}
