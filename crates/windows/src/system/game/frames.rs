//! Writes sampled frames to disk off the session loop.
//!
//! Encoding a frame to PNG costs tens of milliseconds, which would stretch the
//! input timeline and inflate the measured jitter. One writer thread drains a
//! bounded queue with a cached WIC encoder, so the loop pays only the hand-off
//! and a frame the encoder cannot keep up with is skipped rather than delaying
//! the session.

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::thread::JoinHandle;

use acdesktop_core::{AdapterError, ErrorCode};

use crate::system::capture::persistent::FramePixels;
use crate::system::capture::png_codec::PngEncoder;

const QUEUE_DEPTH: usize = 4;
const BYTES_PER_PIXEL: u32 = 4;

pub(in crate::system) struct FrameWriter {
    sender: Option<SyncSender<(usize, u64, FramePixels)>>,
    handle: Option<JoinHandle<Result<(), AdapterError>>>,
    index: usize,
    slots: Arc<AtomicUsize>,
}

impl FrameWriter {
    pub(in crate::system) fn open(dir: &Path) -> Result<Self, AdapterError> {
        let (sender, receiver) = sync_channel::<(usize, u64, FramePixels)>(QUEUE_DEPTH);
        let target: PathBuf = dir.to_path_buf();
        let slots = Arc::new(AtomicUsize::new(QUEUE_DEPTH));
        let worker_slots = Arc::clone(&slots);
        let handle = std::thread::Builder::new()
            .name("acdesktop-frame-writer".to_owned())
            .spawn(move || -> Result<(), AdapterError> {
                let encoder = PngEncoder::new()?;
                let manifest_path = target.join("frames.jsonl");
                let file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&manifest_path)
                    .map_err(|error| {
                        AdapterError::new(
                            ErrorCode::ActionFailed,
                            format!("Could not create {}: {error}", manifest_path.display()),
                        )
                    })?;
                let mut manifest = BufWriter::new(file);
                for (index, at_ms, frame) in receiver {
                    worker_slots.fetch_add(1, Ordering::Release);
                    let stride = frame.width.checked_mul(BYTES_PER_PIXEL).ok_or_else(|| {
                        AdapterError::internal("frame width overflows its stride")
                    })?;
                    let png = encoder.encode(&frame.pixels, frame.width, frame.height, stride)?;
                    let path = target.join(format!("frame-{index:04}.png"));
                    std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                        .and_then(|mut file| file.write_all(&png))
                        .map_err(|error| {
                            AdapterError::new(
                                ErrorCode::ActionFailed,
                                format!("Could not write {}: {error}", path.display()),
                            )
                        })?;
                    writeln!(
                        manifest,
                        "{}",
                        serde_json::json!({"file": format!("frame-{index:04}.png"), "at_ms": at_ms})
                    )
                    .map_err(|error| {
                        AdapterError::new(
                            ErrorCode::ActionFailed,
                            format!("Could not write {}: {error}", manifest_path.display()),
                        )
                    })?;
                }
                manifest.flush().map_err(|error| {
                    AdapterError::new(
                        ErrorCode::ActionFailed,
                        format!("Could not flush {}: {error}", manifest_path.display()),
                    )
                })?;
                Ok(())
            })
            .map_err(|error| {
                AdapterError::new(
                    ErrorCode::Internal,
                    format!("Could not start the frame writer: {error}"),
                )
            })?;
        Ok(Self {
            sender: Some(sender),
            handle: Some(handle),
            index: 0,
            slots,
        })
    }

    pub(in crate::system) fn submit(
        &mut self,
        frame: &FramePixels,
        at_ms: u64,
    ) -> Result<(), AdapterError> {
        let Some(sender) = self.sender.as_ref() else {
            return Ok(());
        };
        if self
            .slots
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |available| {
                available.checked_sub(1)
            })
            .is_err()
        {
            return Ok(());
        }
        match sender.try_send((self.index, at_ms, frame.clone())) {
            Ok(()) => {
                self.index += 1;
                Ok(())
            }
            Err(error) => {
                self.slots.fetch_add(1, Ordering::Release);
                match error {
                    TrySendError::Full(_) => Ok(()),
                    TrySendError::Disconnected(_) => Err(AdapterError::new(
                        ErrorCode::ActionFailed,
                        "The frame writer stopped before the session ended".to_owned(),
                    )),
                }
            }
        }
    }

    pub(in crate::system) fn finish(mut self) -> Result<(), AdapterError> {
        self.sender.take();
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        handle.join().map_err(|_| {
            AdapterError::new(
                ErrorCode::Internal,
                "The frame writer panicked while saving frames".to_owned(),
            )
        })?
    }
}

#[cfg(test)]
#[path = "frames_tests.rs"]
mod tests;
