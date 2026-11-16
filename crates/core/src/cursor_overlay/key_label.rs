pub fn control_key_label(combo: &str) -> String {
    let compact: String = combo.chars().filter(|c| !c.is_whitespace()).collect();
    let parts: Vec<_> = compact.split('+').collect();
    let key = parts.last().copied().unwrap_or_default();
    let modifiers = &parts[..parts.len().saturating_sub(1)];
    let allowed_modifiers = [
        "ctrl", "control", "alt", "option", "meta", "cmd", "command", "super", "win", "shift",
    ];
    if compact.len() > 80
        || !modifiers.iter().all(|part| {
            allowed_modifiers
                .iter()
                .any(|candidate| part.eq_ignore_ascii_case(candidate))
        })
    {
        return "Key requested".into();
    }
    let named = [
        "return",
        "enter",
        "escape",
        "esc",
        "tab",
        "space",
        "delete",
        "backspace",
        "forwarddelete",
        "home",
        "end",
        "pageup",
        "pagedown",
        "left",
        "right",
        "up",
        "down",
        "cmd",
        "command",
        "shift",
        "alt",
        "option",
        "ctrl",
        "control",
        "meta",
    ];
    let function_key = key
        .strip_prefix('f')
        .or_else(|| key.strip_prefix('F'))
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=12).contains(&n));
    let single_key = key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric();
    if !single_key
        && !function_key
        && !named
            .iter()
            .any(|candidate| key.eq_ignore_ascii_case(candidate))
    {
        return "Key requested".into();
    }
    let has_command_modifier = parts
        .iter()
        .take(parts.len().saturating_sub(1))
        .any(|part| {
            [
                "ctrl", "control", "alt", "option", "meta", "cmd", "command", "super", "win",
            ]
            .iter()
            .any(|candidate| part.eq_ignore_ascii_case(candidate))
        });
    if key.chars().count() == 1 && !has_command_modifier {
        return "Character key requested".into();
    }
    format!("Key requested: {compact}")
}
