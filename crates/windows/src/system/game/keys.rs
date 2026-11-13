use super::steps::{Step, StepAction};
use crate::input::keyboard::resolve_key;
use acdesktop_core::{AdapterError, KeyCombo, Modifier};
use std::collections::BTreeMap;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_LWIN, VK_MENU, VK_SHIFT};

pub(in crate::system) fn validate(steps: &[Step]) -> Result<(), AdapterError> {
    let mut held = BTreeMap::<u16, String>::new();
    let mut buttons = std::collections::BTreeSet::new();
    for step in steps {
        if let StepAction::MouseButton { button, down, .. } = &step.action {
            if !matches!(button.as_str(), "left" | "right" | "middle")
                || (*down && !buttons.insert(button.clone()))
                || (!*down && !buttons.remove(button))
            {
                return Err(AdapterError::new(
                    acdesktop_core::ErrorCode::InvalidArgs,
                    "Invalid mouse button sequence in game plan",
                ));
            }
        }
        let (key, down) = match &step.action {
            StepAction::KeyDown { key } => (key, true),
            StepAction::KeyUp { key } => (key, false),
            _ => continue,
        };
        let normalized = key.to_lowercase();
        let vk = resolve_key(&normalized)?.vk;
        if down {
            if held.contains_key(&vk) {
                return Err(AdapterError::new(
                    acdesktop_core::ErrorCode::InvalidArgs,
                    "Game plan presses an already held key",
                ));
            }
            held.insert(vk, normalized);
            let modifiers = modifiers(&held);
            for (held_vk, name) in &held {
                if is_modifier(*held_vk) {
                    continue;
                }
                let combo = KeyCombo {
                    key: name.clone(),
                    modifiers: modifiers.clone(),
                };
                if crate::input::blocked_combo::is_blocked(&combo) {
                    return Err(AdapterError::policy_denied(
                        "Game input plan contains a blocked system shortcut",
                    ));
                }
            }
        } else if held.remove(&vk).is_none() {
            return Err(AdapterError::new(
                acdesktop_core::ErrorCode::InvalidArgs,
                "Game plan releases a key it did not press",
            ));
        }
    }
    Ok(())
}

fn is_modifier(vk: u16) -> bool {
    matches!(vk, VK_LWIN | VK_CONTROL | VK_MENU | VK_SHIFT)
}

fn modifiers(held: &BTreeMap<u16, String>) -> Vec<Modifier> {
    [
        (VK_LWIN, Modifier::Meta),
        (VK_CONTROL, Modifier::Ctrl),
        (VK_MENU, Modifier::Alt),
        (VK_SHIFT, Modifier::Shift),
    ]
    .into_iter()
    .filter_map(|(key, modifier)| held.contains_key(&key).then_some(modifier))
    .collect()
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
