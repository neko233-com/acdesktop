//! The game session loop: keep one capture session open, play the session
//! input timeline on schedule, and sample the window on a fixed cadence.
//!
//! The loop owns the wall clock (`Instant`) and asks the capture session for a
//! frame only when a sample is due, so a slow frame never shifts the input
//! schedule. Samples are recorded only when the capture API returns a frame.
//! Frames are compared in place and dropped after
//! their sample; only the compact sample list survives, and `--save-frames`
//! hands buffers to a writer thread so encoding never stretches the timeline.

use std::time::{Duration, Instant};

use acdesktop_core::{
    AdapterError, Deadline, FrameSample, GameSessionLog, GameSessionRequest, InputSample,
};

use super::frames::FrameWriter;
use super::input::{InputGuard, dispatch};
use super::observe::changed_ratio;
use super::steps::{Step, build_steps, validate_touch_steps};
use crate::system::capture::persistent::{PersistentSession, wait_until};
use crate::system::window::window_ops;

const MAX_FPS: u32 = 240;
const MIN_WAKE_US: u64 = 100;

pub(crate) fn run(
    request: &GameSessionRequest<'_>,
    deadline: Deadline,
) -> Result<GameSessionLog, AdapterError> {
    let handle = window_ops::parse_handle(&request.window.id);
    if handle.is_null() {
        return Err(AdapterError::new(
            acdesktop_core::ErrorCode::WindowNotFound,
            format!("Window '{}' has no native handle", request.window.id),
        ));
    }
    let origin = crate::system::window::client_origin::client_origin(handle)?;
    let evidence = crate::system::window::window_identity::WindowIdentityEvidence::from_info(
        handle,
        request.window,
    )
    .ok_or_else(|| {
        AdapterError::new(
            acdesktop_core::ErrorCode::StaleRef,
            "Game session requires a process-instance token",
        )
    })?;
    evidence.verify_stored()?;
    let capture = PersistentSession::open(handle, false, deadline)?;
    let cursor_capture = if request.plan.capture_cursor && request.frames_dir.is_some() {
        Some(PersistentSession::open(handle, true, deadline)?)
    } else {
        None
    };
    let (width, height) = capture.dimensions();
    let steps: Vec<Step> = build_steps(request.plan);
    validate_touch_steps(&steps)?;
    super::keys::validate(&steps)?;
    if !steps.is_empty() {
        crate::input::elevation::ensure_target_integrity_allows_input(request.window.pid)?;
    }
    if steps
        .last()
        .is_some_and(|step| step.at_ms >= request.plan.duration_ms)
    {
        return Err(AdapterError::new(
            acdesktop_core::ErrorCode::InvalidArgs,
            "Every input step must finish before the session duration",
        ));
    }
    let fps = request.plan.fps.clamp(1, MAX_FPS);
    let frame_span_us = 1_000_000u64 / u64::from(fps);

    let planned_end_ms = request
        .plan
        .duration_ms
        .max(steps.last().map(|step| step.at_ms).unwrap_or(0));
    let budget = deadline.remaining_slice(Duration::from_millis(planned_end_ms))?;
    let start = Instant::now();
    let end = start + budget;

    let mut frames: Vec<FrameSample> = Vec::new();
    let mut inputs: Vec<InputSample> = Vec::new();
    let mut guard = InputGuard::new(origin);
    let mut writer = match request.frames_dir {
        Some(dir) => Some(FrameWriter::open(dir)?),
        None => None,
    };
    let mut previous: Option<Vec<u8>> = None;
    let mut step_index = 0usize;
    let mut next_frame_us = 0u64;

    let outcome = (|| -> Result<(), AdapterError> {
        while Instant::now() < end {
            let elapsed_us = start.elapsed().as_micros() as u64;
            while let Some(step) = steps.get(step_index) {
                if step.at_ms * 1_000 > elapsed_us {
                    break;
                }
                if !crate::system::window::window_activate::is_owned_foreground(handle, &evidence) {
                    return Err(AdapterError::new(
                        acdesktop_core::ErrorCode::ActionFailed,
                        "Game target lost foreground or window ownership; input stopped",
                    ));
                }
                if crate::system::window::client_origin::client_origin(handle)? != origin {
                    return Err(AdapterError::new(
                        acdesktop_core::ErrorCode::ActionFailed,
                        "Game window moved; input stopped",
                    ));
                }
                super::target::verify_step(handle, origin, &step.action)?;
                let dispatched_ms = start.elapsed().as_micros() as u64 / 1_000;
                dispatch(&step.action, deadline, &mut guard)?;
                inputs.push(InputSample {
                    event: step.event,
                    scheduled_ms: step.at_ms,
                    dispatched_ms,
                    kind: step.label.to_owned(),
                });
                step_index += 1;
            }
            if elapsed_us >= next_frame_us {
                if let Some(frame) = capture.grab(deadline)? {
                    let ratio = changed_ratio(
                        previous.as_deref(),
                        &frame.pixels,
                        frame.width,
                        frame.height,
                    );
                    if cursor_capture.is_none() {
                        if let Some(writer) = writer.as_mut() {
                            writer.submit(&frame, start.elapsed().as_millis() as u64)?;
                        }
                    }
                    previous = Some(frame.pixels);
                    frames.push(FrameSample {
                        at_ms: start.elapsed().as_millis() as u64,
                        changed_ratio: ratio,
                    });
                }
                if let (Some(cursor_capture), Some(writer)) = (&cursor_capture, &mut writer) {
                    if let Some(frame) = cursor_capture.grab(deadline)? {
                        writer.submit(&frame, start.elapsed().as_millis() as u64)?;
                    }
                }
                next_frame_us = (elapsed_us / frame_span_us + 1) * frame_span_us;
            }
            let next_step_us = steps
                .get(step_index)
                .map(|step| step.at_ms * 1_000)
                .unwrap_or(u64::MAX);
            let next_wake_us = next_frame_us
                .min(next_step_us)
                .max(elapsed_us + MIN_WAKE_US);
            let target = start + Duration::from_micros(next_wake_us);
            if target >= end {
                wait_until(end, deadline)?;
                break;
            }
            wait_until(target, deadline)?;
        }
        Ok(())
    })();
    let duration_ms = start.elapsed().as_millis() as u64;
    guard.release_all();
    let saved = match writer.take() {
        Some(writer) => writer.finish(),
        None => Ok(()),
    };
    outcome.map_err(|error| account_for_prior_steps(error, step_index))?;
    saved.map_err(|error| account_for_prior_steps(error, step_index))?;
    ensure_all_steps_dispatched(&steps, step_index)?;

    Ok(GameSessionLog {
        duration_ms,
        width,
        height,
        frames,
        inputs,
    })
}

fn account_for_prior_steps(mut error: AdapterError, completed: usize) -> AdapterError {
    if completed > 0 {
        error.disposition = acdesktop_core::DeliverySemantics::unknown();
    }
    error
}

fn ensure_all_steps_dispatched(steps: &[Step], completed: usize) -> Result<(), AdapterError> {
    let Some(pending) = steps.get(completed) else {
        return Ok(());
    };
    let mut error = AdapterError::new(
        acdesktop_core::ErrorCode::ActionFailed,
        "Game session ended before every input step was dispatched",
    )
    .with_details(serde_json::json!({
        "completed_steps": completed,
        "planned_steps": steps.len(),
        "first_pending_event": pending.event,
        "first_pending_at_ms": pending.at_ms,
    }))
    .with_suggestion(
        "Increase the session duration or timeout, and inspect the target before retrying.",
    );
    if completed == 0 {
        error = error.with_disposition(acdesktop_core::DeliverySemantics::not_delivered());
    }
    Err(error)
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
