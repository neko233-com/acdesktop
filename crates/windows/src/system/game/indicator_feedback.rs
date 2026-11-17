use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use acdesktop_core::{ControlIndicatorEvent, Point, TouchPhase};

use super::steps::StepAction;

enum Feedback {
    Keys(String),
    Click(Point),
}

pub(in crate::system) struct FeedbackWorker {
    sender: Option<mpsc::SyncSender<Feedback>>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FeedbackWorker {
    pub(in crate::system) fn start(owner: Option<&str>) -> Option<Self> {
        let owner = owner?.to_owned();
        let (sender, receiver) = mpsc::sync_channel(16);
        let stopped = Arc::new(AtomicBool::new(false));
        let thread_stopped = Arc::clone(&stopped);
        let thread = std::thread::Builder::new()
            .name("acdesktop-game-feedback".into())
            .spawn(move || {
                while let Ok(feedback) = receiver.recv() {
                    if thread_stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let event = match &feedback {
                        Feedback::Keys(display) => ControlIndicatorEvent::Keys {
                            owner: &owner,
                            display,
                        },
                        Feedback::Click(point) => ControlIndicatorEvent::Click {
                            owner: &owner,
                            point,
                        },
                    };
                    let _ = crate::system::cursor_overlay::indicator::present_with_budget(
                        &event,
                        Duration::from_millis(250),
                    );
                }
            })
            .ok()?;
        Some(Self {
            sender: Some(sender),
            stopped,
            thread: Some(thread),
        })
    }

    pub(in crate::system) fn present(&self, action: &StepAction, origin: (i32, i32)) {
        if let (Some(sender), Some(feedback)) = (&self.sender, feedback_for(action, origin)) {
            let _ = sender.try_send(feedback);
        }
    }
}

impl Drop for FeedbackWorker {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn feedback_for(action: &StepAction, origin: (i32, i32)) -> Option<Feedback> {
    match action {
        StepAction::KeyDown { key } | StepAction::KeyUp { key } => {
            Some(Feedback::Keys(acdesktop_core::control_key_label(key)))
        }
        StepAction::MouseButton {
            x, y, down: true, ..
        } => Some(Feedback::Click(point(*x, *y, origin))),
        StepAction::Touch(contacts) => contacts
            .iter()
            .find(|contact| contact.phase == TouchPhase::Down)
            .map(|contact| Feedback::Click(point(contact.x, contact.y, origin))),
        _ => None,
    }
}

fn point(x: i32, y: i32, origin: (i32, i32)) -> Point {
    Point {
        x: f64::from(x) + f64::from(origin.0),
        y: f64::from(y) + f64::from(origin.1),
    }
}

#[cfg(test)]
#[path = "indicator_feedback_tests.rs"]
mod tests;
