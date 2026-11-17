//! Plays one [`super::steps::StepAction`] and owns the release of anything it
//! left held.
//!
//! A game session presses keys and buttons that outlive a single event, so an
//! error part-way through must not leave a key stuck down. [`InputGuard`]
//! records every press and releases it on the way out, the same contract the
//! one-shot command guards keep.

use std::time::Duration;

use acdesktop_core::{AdapterError, Deadline, ErrorCode, TouchContact, TouchPhase};

use super::steps::StepAction;
use crate::input::keyboard::{key_input, post_keyboard_inputs, resolve_key};
use crate::input::mouse::mouse_send::{
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, button_input, move_input, post_mouse_inputs,
};
use crate::input::mouse::{NormalizedPoint, normalize_point};
use crate::input::touch::send_touch_frame;

const CLEANUP_BUDGET: Duration = Duration::from_millis(500);

#[derive(Default)]
pub(in crate::system) struct InputGuard {
    origin: (i32, i32),
    keys: Vec<u16>,
    buttons: Vec<u32>,
    contacts: Vec<TouchContact>,
}

impl InputGuard {
    pub(in crate::system) fn new(origin: (i32, i32)) -> Self {
        Self {
            origin,
            ..Self::default()
        }
    }

    fn press_key(&mut self, vk: u16) {
        post_keyboard_inputs(&[key_input(vk, false)]);
        if !self.keys.contains(&vk) {
            self.keys.push(vk);
        }
    }

    fn release_key(&mut self, vk: u16) {
        post_keyboard_inputs(&[key_input(vk, true)]);
        self.keys.retain(|held| *held != vk);
    }

    fn press_button(&mut self, x: i32, y: i32, down: u32) {
        post_mouse_inputs(&[move_input(self.absolute(x, y))]);
        post_mouse_inputs(&[button_input(down)]);
        if !self.buttons.contains(&down) {
            self.buttons.push(down);
        }
    }

    fn release_button(&mut self, up: u32) {
        post_mouse_inputs(&[button_input(up)]);
        self.buttons.retain(|held| held_up_flag(*held) != Some(up));
    }

    fn send_touch(
        &mut self,
        contacts: &[TouchContact],
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        let pressed = self.press_unseen(contacts);
        let translated = self.absolute_contacts(&pressed);
        send_touch_frame(&translated, deadline)?;
        self.contacts = translated
            .into_iter()
            .filter(|contact| contact.phase != TouchPhase::Up)
            .collect();
        Ok(())
    }

    /// Turns a `move` on a contact the frame has not pressed yet into a
    /// `down`. A plan names a finger by id and lets `move` mean "put it
    /// here", but the OS rejects an update for a pointer that was never
    /// pressed, so the first frame for an id has to be a down.
    fn press_unseen(&self, contacts: &[TouchContact]) -> Vec<TouchContact> {
        contacts
            .iter()
            .map(|contact| match contact.phase {
                TouchPhase::Move if !self.contacts.iter().any(|held| held.id == contact.id) => {
                    TouchContact {
                        phase: TouchPhase::Down,
                        ..*contact
                    }
                }
                _ => *contact,
            })
            .collect()
    }

    fn absolute(&self, x: i32, y: i32) -> NormalizedPoint {
        normalize_point(&acdesktop_core::Point {
            x: f64::from(x + self.origin.0),
            y: f64::from(y + self.origin.1),
        })
    }

    fn absolute_contacts(&self, contacts: &[TouchContact]) -> Vec<TouchContact> {
        contacts
            .iter()
            .map(|contact| TouchContact {
                x: contact.x + self.origin.0,
                y: contact.y + self.origin.1,
                ..*contact
            })
            .collect()
    }

    pub(in crate::system) fn release_all(&mut self) {
        for vk in self.keys.drain(..) {
            post_keyboard_inputs(&[key_input(vk, true)]);
        }
        for down in self.buttons.drain(..) {
            if let Some(up) = held_up_flag(down) {
                post_mouse_inputs(&[button_input(up)]);
            }
        }
        if !self.contacts.is_empty() {
            let release = self
                .contacts
                .drain(..)
                .map(|contact| TouchContact {
                    phase: TouchPhase::Up,
                    ..contact
                })
                .collect::<Vec<_>>();
            if let Ok(cleanup) = Deadline::from_duration(CLEANUP_BUDGET) {
                let _ = send_touch_frame(&release, cleanup);
            }
        }
    }
}

fn held_up_flag(down: u32) -> Option<u32> {
    match down {
        MOUSEEVENTF_LEFTDOWN => Some(MOUSEEVENTF_LEFTUP),
        MOUSEEVENTF_RIGHTDOWN => Some(MOUSEEVENTF_RIGHTUP),
        MOUSEEVENTF_MIDDLEDOWN => Some(MOUSEEVENTF_MIDDLEUP),
        _ => None,
    }
}

fn button_flags(button: &str) -> Result<(u32, u32), AdapterError> {
    match button {
        "left" => Ok((MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP)),
        "right" => Ok((MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP)),
        "middle" => Ok((MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP)),
        other => Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            format!("Unknown mouse button '{other}'"),
        )),
    }
}

pub(in crate::system) fn dispatch(
    action: &StepAction,
    deadline: Deadline,
    guard: &mut InputGuard,
) -> Result<(), AdapterError> {
    match action {
        StepAction::KeyDown { key } => {
            guard.press_key(resolve_key(&key.to_lowercase())?.vk);
            Ok(())
        }
        StepAction::KeyUp { key } => {
            guard.release_key(resolve_key(&key.to_lowercase())?.vk);
            Ok(())
        }
        StepAction::MouseMove { x, y } => {
            post_mouse_inputs(&[move_input(guard.absolute(*x, *y))]);
            Ok(())
        }
        StepAction::MouseButton { x, y, button, down } => {
            let (down_flag, up_flag) = button_flags(button)?;
            if *down {
                guard.press_button(*x, *y, down_flag);
            } else {
                guard.release_button(up_flag);
            }
            Ok(())
        }
        StepAction::Touch(contacts) => guard.send_touch(contacts, deadline),
    }
}

#[cfg(all(test, target_os = "windows"))]
#[path = "input_tests.rs"]
mod tests;
