//! Turns a plan into the flat, time-ordered list the session loop plays.
//!
//! A hold becomes a press and a release; a click becomes a down and an up; a
//! gesture expands to its whole frame sequence. Building this once keeps the
//! loop free of per-event branching and makes the schedule inspectable in a
//! test.

use acdesktop_core::{
    GameEvent, GameSessionPlan, GestureSpec, TouchContact, TouchPhase, gesture_frames,
};

const CLICK_HOLD_MS: u64 = 20;
const MIN_TOUCH_HOLD_MS: u64 = 16;

pub(in crate::system) enum StepAction {
    KeyDown {
        key: String,
    },
    KeyUp {
        key: String,
    },
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseButton {
        x: i32,
        y: i32,
        button: String,
        down: bool,
    },
    Touch(Vec<TouchContact>),
}

pub(in crate::system) struct Step {
    pub(in crate::system) at_ms: u64,
    pub(in crate::system) event: usize,
    pub(in crate::system) label: &'static str,
    pub(in crate::system) action: StepAction,
}

pub(in crate::system) fn build_steps(plan: &GameSessionPlan) -> Vec<Step> {
    let mut steps = Vec::new();
    for (event, planned) in plan.events.iter().enumerate() {
        push_event(&mut steps, event, planned);
    }
    steps.sort_by_key(|step| step.at_ms);
    steps
}

pub(in crate::system) fn validate_touch_steps(
    steps: &[Step],
) -> Result<(), acdesktop_core::AdapterError> {
    let mut held = std::collections::BTreeMap::new();
    for step in steps {
        let StepAction::Touch(contacts) = &step.action else {
            continue;
        };
        let mut seen = std::collections::BTreeSet::new();
        for contact in contacts {
            if !seen.insert(contact.id)
                || (contact.phase == TouchPhase::Down && held.contains_key(&contact.id))
                || (contact.phase == TouchPhase::Up
                    && held.get(&contact.id) != Some(&(contact.x, contact.y)))
            {
                return Err(acdesktop_core::AdapterError::new(
                    acdesktop_core::ErrorCode::InvalidArgs,
                    "Invalid touch contact sequence",
                ));
            }
        }
        if held.keys().any(|id| !seen.contains(id)) {
            return Err(acdesktop_core::AdapterError::new(
                acdesktop_core::ErrorCode::InvalidArgs,
                "Each touch frame must include every held contact",
            ));
        }
        for contact in contacts {
            if contact.phase == TouchPhase::Up {
                held.remove(&contact.id);
            } else {
                held.insert(contact.id, (contact.x, contact.y));
            }
        }
    }
    if !held.is_empty() {
        return Err(acdesktop_core::AdapterError::new(
            acdesktop_core::ErrorCode::InvalidArgs,
            "Touch sequence must release every contact",
        ));
    }
    Ok(())
}

fn push_event(steps: &mut Vec<Step>, event: usize, planned: &GameEvent) {
    match planned {
        GameEvent::Key {
            at_ms,
            key,
            hold_ms,
        } => {
            steps.push(Step {
                at_ms: *at_ms,
                event,
                label: "key",
                action: StepAction::KeyDown { key: key.clone() },
            });
            steps.push(Step {
                at_ms: at_ms.saturating_add(*hold_ms),
                event,
                label: "key",
                action: StepAction::KeyUp { key: key.clone() },
            });
        }
        GameEvent::KeyDown { at_ms, key } => steps.push(Step {
            at_ms: *at_ms,
            event,
            label: "key-down",
            action: StepAction::KeyDown { key: key.clone() },
        }),
        GameEvent::KeyUp { at_ms, key } => steps.push(Step {
            at_ms: *at_ms,
            event,
            label: "key-up",
            action: StepAction::KeyUp { key: key.clone() },
        }),
        GameEvent::MouseMove { at_ms, x, y } => steps.push(Step {
            at_ms: *at_ms,
            event,
            label: "mouse-move",
            action: StepAction::MouseMove { x: *x, y: *y },
        }),
        GameEvent::MouseDown {
            at_ms,
            x,
            y,
            button,
        } => push_button(steps, event, *at_ms, *x, *y, button, true, "mouse-down"),
        GameEvent::MouseUp {
            at_ms,
            x,
            y,
            button,
        } => push_button(steps, event, *at_ms, *x, *y, button, false, "mouse-up"),
        GameEvent::MouseClick {
            at_ms,
            x,
            y,
            button,
        } => {
            push_button(steps, event, *at_ms, *x, *y, button, true, "mouse-click");
            push_button(
                steps,
                event,
                at_ms.saturating_add(CLICK_HOLD_MS),
                *x,
                *y,
                button,
                false,
                "mouse-click",
            );
        }
        GameEvent::Gesture {
            at_ms,
            gesture,
            x,
            y,
            duration_ms,
            distance,
        } => {
            let spec = GestureSpec {
                name: *gesture,
                x: *x,
                y: *y,
                duration_ms: *duration_ms,
                distance: *distance,
            };
            for frame in gesture_frames(&spec, *at_ms) {
                steps.push(Step {
                    at_ms: frame.at_ms,
                    event,
                    label: "gesture",
                    action: StepAction::Touch(frame.contacts),
                });
            }
        }
        GameEvent::Touch {
            at_ms,
            contacts,
            hold_ms,
        } => {
            steps.push(Step {
                at_ms: *at_ms,
                event,
                label: "touch",
                action: StepAction::Touch(contacts.clone()),
            });
            let release = contacts
                .iter()
                .map(|contact| TouchContact {
                    phase: TouchPhase::Up,
                    ..*contact
                })
                .collect::<Vec<_>>();
            if !release.is_empty() {
                steps.push(Step {
                    at_ms: at_ms.saturating_add((*hold_ms).max(MIN_TOUCH_HOLD_MS)),
                    event,
                    label: "touch",
                    action: StepAction::Touch(release),
                });
            }
        }
        GameEvent::TouchPath { at_ms, frames } => {
            for frame in frames {
                steps.push(Step {
                    at_ms: at_ms.saturating_add(frame.at_ms),
                    event,
                    label: "touch-path",
                    action: StepAction::Touch(frame.contacts.clone()),
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_button(
    steps: &mut Vec<Step>,
    event: usize,
    at_ms: u64,
    x: i32,
    y: i32,
    button: &str,
    down: bool,
    label: &'static str,
) {
    steps.push(Step {
        at_ms,
        event,
        label,
        action: StepAction::MouseButton {
            x,
            y,
            button: button.to_owned(),
            down,
        },
    });
}

#[cfg(test)]
#[path = "steps_tests.rs"]
mod tests;
