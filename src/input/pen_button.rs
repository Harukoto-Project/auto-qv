use std::str::FromStr;

use crate::error::AutoQvError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PenButton {
    #[default]
    Left,
    Right,
    Middle,
}

impl FromStr for PenButton {
    type Err = AutoQvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "middle" => Ok(Self::Middle),
            _ => Err(AutoQvError::Config(format!(
                "unknown pen button {s:?} (expected left, right or middle)"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_buttons_case_insensitively() {
        assert_eq!("left".parse::<PenButton>().unwrap(), PenButton::Left);
        assert_eq!("Right".parse::<PenButton>().unwrap(), PenButton::Right);
        assert_eq!(" MIDDLE ".parse::<PenButton>().unwrap(), PenButton::Middle);
    }

    #[test]
    fn rejects_unknown_buttons() {
        assert!("x1".parse::<PenButton>().is_err());
        assert!("".parse::<PenButton>().is_err());
    }

    #[test]
    fn defaults_to_left() {
        assert_eq!(PenButton::default(), PenButton::Left);
    }
}
