use std::time::{Duration, Instant};

use acdesktop_core::{
    AdapterError, ControlIndicatorEvent, CursorOverlayConfig, CursorOverlayControl,
    CursorOverlayInstruction, CursorOverlayStyle, CursorPhase, ErrorCode, Point,
};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};

use super::{hosted_watch::HostedIndicatorWatch, label, pipe_name, server, spawn, transport};

#[path = "indicator/layout.rs"]
mod layout;
#[path = "indicator/pulse.rs"]
mod pulse;
#[path = "indicator/view.rs"]
mod view;

const BUDGET: Duration = Duration::from_millis(1500);
const MINIMUM_VISIBLE: Duration = Duration::from_millis(1000);
const FLAG: &str = "--control-indicator-child";

fn name(owner: &str) -> Result<String, AdapterError> {
    acdesktop_core::context::validate_session_id(owner)
        .map_err(|error| AdapterError::new(ErrorCode::InvalidArgs, error.to_string()))?;
    let root = acdesktop_core::session::acdesktop_dir()
        .map_err(|error| AdapterError::internal(error.to_string()))?;
    Ok(pipe_name::pipe_name_for_generation(
        &root,
        owner,
        None,
        "indicator-v1",
    ))
}

pub(crate) fn show(owner: &str, session: bool, description: &str) -> Result<(), AdapterError> {
    let name = name(owner)?;
    let mut control = CursorOverlayControl::enable(owner.to_owned(), CursorOverlayStyle::default());
    if let CursorOverlayControl::Enable { label: text, .. } = &mut control {
        *text = label::clamp(description).unwrap_or_else(|| "Working".into());
    }
    match transport::reach(&name, &control, BUDGET) {
        transport::ReachOutcome::Delivered => Ok(()),
        transport::ReachOutcome::Unreachable(error) => Err(error),
        transport::ReachOutcome::NoRenderer => spawn::start_indicator(&name, &control, session),
    }
}

pub(crate) fn hide(owner: &str) -> Result<(), AdapterError> {
    let name = name(owner)?;
    let control = CursorOverlayControl::disable(owner.to_owned());
    match transport::reach(&name, &control, BUDGET) {
        transport::ReachOutcome::Delivered | transport::ReachOutcome::NoRenderer => Ok(()),
        transport::ReachOutcome::Unreachable(error) => Err(error),
    }
}

pub(crate) fn present(event: &ControlIndicatorEvent<'_>) -> Result<(), AdapterError> {
    let (owner, point, text, phase) = match event {
        ControlIndicatorEvent::Keys { owner, display } => (
            *owner,
            Point { x: 0.0, y: 0.0 },
            Some((*display).to_owned()),
            CursorPhase::Travel,
        ),
        ControlIndicatorEvent::Click { owner, point } => {
            (*owner, (*point).clone(), None, CursorPhase::Effect)
        }
    };
    let config = CursorOverlayConfig::enabled(text, 12)?;
    let instruction = CursorOverlayInstruction::new(point, &config, phase == CursorPhase::Effect)?
        .with_phase(phase);
    let control = CursorOverlayControl::present(owner.to_owned(), instruction);
    match transport::reach(&name(owner)?, &control, BUDGET) {
        transport::ReachOutcome::Delivered => Ok(()),
        transport::ReachOutcome::NoRenderer => Err(AdapterError::internal(
            "The control indicator is unavailable",
        )),
        transport::ReachOutcome::Unreachable(error) => Err(error),
    }
}

pub(crate) fn entry(arguments: &[String]) -> Option<Result<(), AdapterError>> {
    let index = arguments.iter().position(|arg| arg == FLAG)?;
    Some(run_arguments(&arguments[index + 1..]))
}

fn run_arguments(arguments: &[String]) -> Result<(), AdapterError> {
    let owner = arguments.first().ok_or_else(invalid_arguments)?;
    let mode = arguments.get(1).ok_or_else(invalid_arguments)?;
    let parent = arguments
        .get(2)
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(invalid_arguments)?;
    let token = arguments.get(3).ok_or_else(invalid_arguments)?;
    if !matches!(mode.as_str(), "session" | "process" | "hosted") {
        return Err(invalid_arguments());
    }
    let process = if mode == "process" || mode == "hosted" {
        let watch = if mode == "hosted" {
            Some(HostedIndicatorWatch::open(owner)?)
        } else {
            None
        };
        Some(Parent::open(parent, token, watch)?)
    } else {
        None
    };
    serve(owner, process)
}

fn invalid_arguments() -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        "Invalid control indicator arguments",
    )
}

struct Parent {
    process: HANDLE,
    watch: Option<HostedIndicatorWatch>,
}

impl Parent {
    fn open(
        pid: u32,
        expected: &str,
        watch: Option<HostedIndicatorWatch>,
    ) -> Result<Self, AdapterError> {
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if handle.is_null() {
            return Err(AdapterError::internal(
                "The control indicator owner is unavailable",
            ));
        }
        let process = Self {
            process: handle,
            watch,
        };
        let actual = crate::system::process_identity::token_for_pid(pid.into())?;
        if actual.as_deref() != Some(expected) {
            return Err(AdapterError::internal(
                "The control indicator owner identity changed",
            ));
        }
        Ok(process)
    }

    fn finished(&self) -> bool {
        self.watch
            .as_ref()
            .is_some_and(HostedIndicatorWatch::finished)
            || unsafe { WaitForSingleObject(self.process, 0) != WAIT_TIMEOUT }
    }
}

impl Drop for Parent {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.process) };
    }
}

fn serve(owner: &str, parent: Option<Parent>) -> Result<(), AdapterError> {
    crate::system::runtime::dpi::ensure_per_monitor_v2()?;
    let pipe = name(owner)?;
    let manifest = if parent.is_none() {
        Some(
            acdesktop_core::session::session_dir(owner)
                .map_err(|error| AdapterError::internal(error.to_string()))?
                .join(acdesktop_core::session::SESSION_MANIFEST_FILE),
        )
    } else {
        None
    };
    let listener = match server::Listener::claim(&pipe) {
        Ok(listener) => listener,
        Err(server::ClaimError::AlreadyServed) => return Ok(()),
        Err(server::ClaimError::Failed(error)) => return Err(error),
    };
    let mut view = view::IndicatorView::new();
    let mut finished = 0;
    let mut displayed_since = None;
    loop {
        match listener.next_control(view.idle_tick()) {
            server::Accepted::Control(control) => {
                let ours = control.session_id() == owner;
                if ours && control.is_enable() {
                    if manifest.as_deref().is_some_and(session_ended) {
                        return Err(AdapterError::internal("The control session has ended"));
                    }
                    let description = control
                        .label()
                        .and_then(label::clamp)
                        .unwrap_or_else(|| "Working".into());
                    view.present(&description)?;
                    displayed_since.get_or_insert_with(Instant::now);
                }
                if ours && let Some(instruction) = control.instruction() {
                    match instruction.phase() {
                        CursorPhase::Travel => {
                            view.set_keys(instruction.label().unwrap_or("Key requested"))?
                        }
                        CursorPhase::Effect => {
                            if let Err(error) = view.click(instruction.destination()) {
                                tracing::warn!(code = %error.code.as_str(), "control click effect was skipped");
                            }
                        }
                        _ => {}
                    }
                }
                if ours && (control.is_enable() || control.is_disable() || control.is_travel()) {
                    listener.acknowledge();
                }
                listener.finish();
                if ours && control.is_disable() {
                    if parent.is_some()
                        && let Some(since) = displayed_since
                    {
                        while since.elapsed() < MINIMUM_VISIBLE {
                            view.refresh()?;
                            std::thread::sleep(
                                view.idle_tick()
                                    .min(MINIMUM_VISIBLE.saturating_sub(since.elapsed())),
                            );
                        }
                    }
                    return Ok(());
                }
            }
            server::Accepted::Idle => {
                view.refresh()?;
                if parent.as_ref().is_some_and(Parent::finished) {
                    return Ok(());
                }
                if let Some(manifest) = manifest.as_deref() {
                    finished = if session_ended(manifest) {
                        finished + 1
                    } else {
                        0
                    };
                    if finished >= 2 {
                        return Ok(());
                    }
                }
            }
            server::Accepted::Broken(error) => return Err(error),
        }
    }
}

fn session_ended(path: &std::path::Path) -> bool {
    classify_session(crate::system::private_file::read_private_bounded_path(
        path,
        64 * 1024,
    ))
}

fn classify_session(reading: std::io::Result<Vec<u8>>) -> bool {
    match reading {
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
        Ok(body) => serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .is_some_and(|value| value.get("ended_at").is_some_and(|end| !end.is_null())),
    }
}

#[cfg(test)]
#[path = "indicator_tests.rs"]
mod tests;
