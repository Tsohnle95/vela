//! Native key events encoded with the reference xterm keyboard protocol.
use gpui::KeyDownEvent;

pub(super) fn encode(event: &KeyDownEvent, application_cursor: bool) -> Vec<u8> {
    let stroke = &event.keystroke;
    let key = stroke.key.as_str();
    let m = stroke.modifiers;
    let modifiers = u8::from(m.shift)
        | (u8::from(m.alt) << 1)
        | (u8::from(m.control) << 2)
        | (u8::from(m.platform) << 3);
    let navigation = |suffix: char| {
        if modifiers > 0 {
            format!("\x1b[1;{}{suffix}", modifiers + 1).into_bytes()
        } else if application_cursor {
            format!("\x1bO{suffix}").into_bytes()
        } else {
            format!("\x1b[{suffix}").into_bytes()
        }
    };
    let tilde = |number: u8| {
        if modifiers > 0 {
            format!("\x1b[{number};{}~", modifiers + 1).into_bytes()
        } else {
            format!("\x1b[{number}~").into_bytes()
        }
    };
    let alt_prefix = |byte: u8| if m.alt { vec![0x1b, byte] } else { vec![byte] };
    match key {
        "backspace" => return alt_prefix(if m.control { 0x08 } else { 0x7f }),
        "tab" => {
            return if m.shift {
                b"\x1b[Z".to_vec()
            } else {
                vec![b'\t']
            };
        }
        "enter" => return alt_prefix(b'\r'),
        "escape" => return alt_prefix(0x1b),
        "left" | "right" | "up" | "down" => {
            if m.platform {
                return Vec::new();
            }
            return navigation(match key {
                "left" => 'D',
                "right" => 'C',
                "up" => 'A',
                _ => 'B',
            });
        }
        "home" => return navigation('H'),
        "end" => return navigation('F'),
        "insert" => {
            return if m.shift || m.control {
                Vec::new()
            } else {
                b"\x1b[2~".to_vec()
            };
        }
        "delete" => return tilde(3),
        "pageup" | "pagedown" => {
            if m.shift {
                return Vec::new();
            } // native scrollback owns these
            let number = if key == "pageup" { 5 } else { 6 };
            return if m.control {
                tilde(number)
            } else {
                format!("\x1b[{number}~").into_bytes()
            };
        }
        _ => {}
    }
    if let Some(number) = key
        .strip_prefix('f')
        .and_then(|value| value.parse::<u8>().ok())
    {
        if (1..=4).contains(&number) {
            let suffix = char::from(b'P' + number - 1);
            return if modifiers > 0 {
                format!("\x1b[1;{}{suffix}", modifiers + 1).into_bytes()
            } else {
                format!("\x1bO{suffix}").into_bytes()
            };
        }
        if let Some(code) = number
            .checked_sub(5)
            .and_then(|index| [15, 17, 18, 19, 20, 21, 23, 24].get(index as usize))
            .copied()
        {
            return tilde(code);
        }
    }
    if m.platform {
        return Vec::new();
    }
    if m.control && !m.shift && !m.alt {
        if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() {
            return vec![key.as_bytes()[0].to_ascii_uppercase() - b'@'];
        }
        return match key {
            "space" => vec![0],
            "3" | "[" => vec![0x1b],
            "4" | "\\" => vec![0x1c],
            "5" | "]" => vec![0x1d],
            "6" => vec![0x1e],
            "7" => vec![0x1f],
            "8" => vec![0x7f],
            _ => Vec::new(),
        };
    }
    if m.alt && !cfg!(target_os = "macos") {
        if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() {
            let byte = if m.control {
                key.as_bytes()[0].to_ascii_uppercase() - b'@'
            } else if m.shift {
                key.as_bytes()[0].to_ascii_uppercase()
            } else {
                key.as_bytes()[0].to_ascii_lowercase()
            };
            return vec![0x1b, byte];
        }
        let text = if key == "space" {
            Some(" ")
        } else {
            stroke.key_char.as_deref()
        };
        if let Some(text) = text {
            let mut bytes = vec![0x1b];
            bytes.extend_from_slice(text.as_bytes());
            return bytes;
        }
    }
    if m.control {
        return match key {
            "_" => vec![0x1f],
            "@" => vec![0],
            _ => Vec::new(),
        };
    }
    if key == "space" {
        return vec![b' '];
    }
    stroke.key_char.as_deref().unwrap_or("").as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(value: &str) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: gpui::Keystroke::parse(value).unwrap(),
            is_held: false,
            prefer_character_input: false,
        }
    }
    #[test]
    fn special_keys_match_all_reference_modifier_and_application_mode_combinations() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/terminal-keyboard-reference.json"
        ))
        .unwrap();
        let cases = reference["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 832);
        for case in cases {
            let stroke = case["key"].as_str().unwrap();
            let expected = case["bytes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|byte| byte.as_u64().unwrap() as u8)
                .collect::<Vec<_>>();
            assert_eq!(
                encode(&key(stroke), case["application_cursor"].as_bool().unwrap()),
                expected,
                "{stroke}; application_cursor={}",
                case["application_cursor"]
            );
        }
    }
    #[test]
    fn control_named_keys_are_never_mistaken_for_control_letters() {
        for (key, expected) in [
            ("ctrl-left", &b"\x1b[1;5D"[..]),
            ("ctrl-up", &b"\x1b[1;5A"[..]),
            ("ctrl-enter", &b"\r"[..]),
            ("ctrl-backspace", &b"\x08"[..]),
            ("ctrl-f1", &b"\x1b[1;5P"[..]),
            ("ctrl-delete", &b"\x1b[3;5~"[..]),
        ] {
            assert_eq!(encode(&self::key(key), false), expected, "{key}");
        }
        assert_eq!(encode(&key("ctrl-c"), false), b"\x03");
        assert!(encode(&key("ctrl-shift-c"), false).is_empty());
    }
    #[test]
    fn navigation_application_mode_modifiers_and_function_keys_match_xterm() {
        assert_eq!(encode(&key("left"), true), b"\x1bOD");
        assert_eq!(encode(&key("alt-shift-right"), true), b"\x1b[1;4C");
        assert_eq!(encode(&key("shift-tab"), false), b"\x1b[Z");
        assert_eq!(encode(&key("alt-backspace"), false), b"\x1b\x7f");
        assert_eq!(encode(&key("f12"), false), b"\x1b[24~");
        assert_eq!(encode(&key("ctrl-pageup"), false), b"\x1b[5;5~");
        assert!(encode(&key("shift-pageup"), false).is_empty());
        assert_eq!(encode(&key("ctrl-["), false), b"\x1b");
        assert_eq!(encode(&key("ctrl-space"), false), b"\0");
    }
}
