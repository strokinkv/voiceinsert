use std::collections::BTreeSet;

const MODIFIER_ORDER: [&str; 4] = ["Ctrl", "Alt", "Shift", "Win"];

pub fn normalize_hotkey(value: &str) -> anyhow::Result<String> {
    let mut modifiers = BTreeSet::new();
    let mut key = None;

    for raw_part in value.split('+') {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(modifier) = normalize_modifier(part) {
            modifiers.insert(modifier);
            continue;
        }

        let normalized_key =
            normalize_key(part).ok_or_else(|| anyhow::anyhow!("Unknown hotkey key: {part}"))?;
        if key.replace(normalized_key).is_some() {
            anyhow::bail!("Hotkey must contain only one main key");
        }
    }

    let key = key.ok_or_else(|| anyhow::anyhow!("Hotkey must include a main key"))?;
    let mut parts = Vec::new();
    for modifier in MODIFIER_ORDER {
        if modifiers.contains(modifier) {
            parts.push(modifier.to_string());
        }
    }
    parts.push(key);

    Ok(parts.join("+"))
}

fn normalize_modifier(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some("Ctrl"),
        "alt" => Some("Alt"),
        "shift" => Some("Shift"),
        "win" | "windows" | "meta" => Some("Win"),
        _ => None,
    }
}

fn normalize_key(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let key = match lower.as_str() {
        "space" => "Space".to_string(),
        "enter" | "return" => "Enter".to_string(),
        "esc" | "escape" => "Esc".to_string(),
        "tab" => "Tab".to_string(),
        "backspace" => "Backspace".to_string(),
        "delete" | "del" => "Delete".to_string(),
        "insert" | "ins" => "Insert".to_string(),
        "up" | "uparrow" => "Up".to_string(),
        "down" | "downarrow" => "Down".to_string(),
        "left" | "leftarrow" => "Left".to_string(),
        "right" | "rightarrow" => "Right".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "pageup" | "pgup" => "PageUp".to_string(),
        "pagedown" | "pgdn" => "PageDown".to_string(),
        _ => normalize_alpha_numeric_or_function_key(value)?,
    };

    Some(key)
}

fn normalize_alpha_numeric_or_function_key(value: &str) -> Option<String> {
    let upper = value.to_ascii_uppercase();

    if upper.len() == 1 {
        let character = upper.chars().next()?;
        if character.is_ascii_alphanumeric() {
            return Some(upper);
        }
    }

    if let Some(number) = upper.strip_prefix('F') {
        let parsed = number.parse::<u8>().ok()?;
        if (1..=24).contains(&parsed) {
            return Some(format!("F{parsed}"));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::normalize_hotkey;

    #[test]
    fn normalizes_modifier_order() {
        assert_eq!(normalize_hotkey("space+ctrl").unwrap(), "Ctrl+Space");
        assert_eq!(normalize_hotkey("Y+Alt").unwrap(), "Alt+Y");
        assert_eq!(
            normalize_hotkey("shift + win + ctrl + f12").unwrap(),
            "Ctrl+Shift+Win+F12"
        );
    }

    #[test]
    fn accepts_hotkeys_without_modifier() {
        assert_eq!(normalize_hotkey("Space").unwrap(), "Space");
        assert_eq!(normalize_hotkey("f23").unwrap(), "F23");
        assert_eq!(normalize_hotkey("y").unwrap(), "Y");
    }

    #[test]
    fn accepts_common_named_keys() {
        assert_eq!(normalize_hotkey("ctrl+escape").unwrap(), "Ctrl+Esc");
        assert_eq!(normalize_hotkey("alt+pageup").unwrap(), "Alt+PageUp");
    }
}
