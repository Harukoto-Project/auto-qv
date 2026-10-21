const NOT_AT_LINE_START: &str = "、。，．,.)]}）］｝〕〉》」』】!?！？";

pub fn is_prohibited_at_line_start(ch: char) -> bool {
    NOT_AT_LINE_START.contains(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_punctuation_and_closing_brackets() {
        for ch in ['、', '。', '」', '）', '.', ',', ')', '！', '?'] {
            assert!(is_prohibited_at_line_start(ch), "{ch}");
        }
    }

    #[test]
    fn ignores_ordinary_characters() {
        for ch in ['あ', '漢', 'a', '「', '(', ' '] {
            assert!(!is_prohibited_at_line_start(ch), "{ch}");
        }
    }
}
