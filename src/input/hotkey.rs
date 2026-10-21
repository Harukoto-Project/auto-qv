use std::fmt;
use std::str::FromStr;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_ESCAPE, VK_F1, VK_RETURN, VK_SPACE, VK_TAB,
};

use crate::error::AutoQvError;

const FUNCTION_KEY_COUNT: u32 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: u32,
    pub key: u32,
}

impl Hotkey {
    pub const DEFAULT_START: Self = Self {
        modifiers: MOD_ALT,
        key: b'P' as u32,
    };
    pub const DEFAULT_ABORT: Self = Self {
        modifiers: MOD_ALT,
        key: b'C' as u32,
    };
}

impl FromStr for Hotkey {
    type Err = AutoQvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = |reason: &str| AutoQvError::Config(format!("invalid hotkey {s:?}: {reason}"));

        let tokens: Vec<&str> = s.split('+').map(str::trim).collect();
        let Some((key_token, modifier_tokens)) = tokens.split_last() else {
            return Err(invalid("empty"));
        };

        let mut modifiers = 0;
        for token in modifier_tokens {
            modifiers |= parse_modifier(token)
                .ok_or_else(|| invalid(&format!("unknown modifier {token:?}")))?;
        }

        let (key, requires_modifier) =
            parse_key(key_token).ok_or_else(|| invalid(&format!("unknown key {key_token:?}")))?;
        if requires_modifier && modifiers == 0 {
            return Err(invalid("this key requires at least one modifier"));
        }

        Ok(Self { modifiers, key })
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (flag, name) in [
            (MOD_CONTROL, "Ctrl"),
            (MOD_ALT, "Alt"),
            (MOD_SHIFT, "Shift"),
            (MOD_WIN, "Win"),
        ] {
            if self.modifiers & flag != 0 {
                write!(f, "{name}+")?;
            }
        }
        write!(f, "{}", key_name(self.key))
    }
}

fn parse_modifier(token: &str) -> Option<u32> {
    match token.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(MOD_CONTROL),
        "alt" => Some(MOD_ALT),
        "shift" => Some(MOD_SHIFT),
        "win" | "super" => Some(MOD_WIN),
        _ => None,
    }
}

fn parse_key(token: &str) -> Option<(u32, bool)> {
    let lower = token.to_ascii_lowercase();

    let mut chars = lower.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        if ch.is_ascii_lowercase() {
            return Some((u32::from(ch.to_ascii_uppercase()), true));
        }
        if ch.is_ascii_digit() {
            return Some((u32::from(ch), true));
        }
    }

    if let Some(number) = lower.strip_prefix('f').and_then(|n| n.parse::<u32>().ok())
        && (1..=FUNCTION_KEY_COUNT).contains(&number)
    {
        return Some((u32::from(VK_F1) + number - 1, false));
    }

    match lower.as_str() {
        "esc" | "escape" => Some((u32::from(VK_ESCAPE), false)),
        "space" => Some((u32::from(VK_SPACE), true)),
        "enter" | "return" => Some((u32::from(VK_RETURN), true)),
        "tab" => Some((u32::from(VK_TAB), true)),
        _ => None,
    }
}

fn key_name(key: u32) -> String {
    let function_first = u32::from(VK_F1);
    match key {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(key).map_or_else(String::new, String::from),
        k if (function_first..function_first + FUNCTION_KEY_COUNT).contains(&k) => {
            format!("F{}", k - function_first + 1)
        }
        k if k == u32::from(VK_ESCAPE) => "Esc".to_string(),
        k if k == u32::from(VK_SPACE) => "Space".to_string(),
        k if k == u32::from(VK_RETURN) => "Enter".to_string(),
        k if k == u32::from(VK_TAB) => "Tab".to_string(),
        k => format!("0x{k:02X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifier_and_letter() {
        let hotkey: Hotkey = "Alt+P".parse().unwrap();

        assert_eq!(hotkey.modifiers, MOD_ALT);
        assert_eq!(hotkey.key, u32::from(b'P'));
    }

    #[test]
    fn parsing_is_case_insensitive_and_combines_modifiers() {
        let hotkey: Hotkey = "ctrl + shift + f5".parse().unwrap();

        assert_eq!(hotkey.modifiers, MOD_CONTROL | MOD_SHIFT);
        assert_eq!(hotkey.key, u32::from(VK_F1) + 4);
    }

    #[test]
    fn allows_function_keys_and_escape_without_modifier() {
        assert!("F8".parse::<Hotkey>().is_ok());
        assert!("Esc".parse::<Hotkey>().is_ok());
    }

    #[test]
    fn rejects_bare_letters_and_digits() {
        assert!("P".parse::<Hotkey>().is_err());
        assert!("5".parse::<Hotkey>().is_err());
        assert!("Space".parse::<Hotkey>().is_err());
    }

    #[test]
    fn rejects_malformed_input() {
        assert!("".parse::<Hotkey>().is_err());
        assert!("Alt+".parse::<Hotkey>().is_err());
        assert!("Foo+P".parse::<Hotkey>().is_err());
        assert!("Alt+F25".parse::<Hotkey>().is_err());
        assert!("Alt+PP".parse::<Hotkey>().is_err());
    }

    #[test]
    fn display_normalizes_order_and_case() {
        let hotkey: Hotkey = "alt+ctrl+p".parse().unwrap();

        assert_eq!(hotkey.to_string(), "Ctrl+Alt+P");
        assert_eq!(
            "shift+f12".parse::<Hotkey>().unwrap().to_string(),
            "Shift+F12"
        );
        assert_eq!("escape".parse::<Hotkey>().unwrap().to_string(), "Esc");
    }
}
