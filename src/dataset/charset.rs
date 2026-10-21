use std::collections::HashSet;

const KANA_RANGES: [(u32, u32); 3] = [(0x3041, 0x3096), (0x309b, 0x309e), (0x30a1, 0x30fe)];

const KANJI_PUNCTUATION: [u32; 4] = [0x3001, 0x3002, 0x3005, 0x3006];
const KANJI_RANGES: [(u32, u32); 4] = [
    (0x3400, 0x4dbf),
    (0x4e00, 0x9fff),
    (0xf900, 0xfaff),
    (0x20000, 0x2fa1f),
];

const HANGUL_RANGES: [(u32, u32); 2] = [(0x3131, 0x3163), (0xac00, 0xd7a3)];

pub fn is_kana(ch: char) -> bool {
    in_ranges(ch as u32, &KANA_RANGES)
}

pub fn is_hangul(ch: char) -> bool {
    in_ranges(ch as u32, &HANGUL_RANGES)
}

pub fn hangul_chars_in(text: &str) -> Vec<char> {
    let mut seen = HashSet::new();
    text.chars()
        .filter(|ch| is_hangul(*ch) && seen.insert(*ch))
        .collect()
}

pub fn is_kanji_candidate(ch: char) -> bool {
    let code = ch as u32;
    KANJI_PUNCTUATION.contains(&code) || in_ranges(code, &KANJI_RANGES)
}

pub fn kana_chars() -> impl Iterator<Item = char> {
    KANA_RANGES
        .into_iter()
        .flat_map(|(start, end)| start..=end)
        .filter_map(char::from_u32)
}

pub fn kanji_chars_in(text: &str) -> Vec<char> {
    let mut seen = HashSet::new();
    text.chars()
        .filter(|ch| is_kanji_candidate(*ch) && seen.insert(*ch))
        .collect()
}

fn in_ranges(code: u32, ranges: &[(u32, u32)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| (*start..=*end).contains(&code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kana_set_covers_hiragana_katakana_and_marks() {
        let kana: Vec<char> = kana_chars().collect();

        assert_eq!(kana.len(), 86 + 4 + 94);
        assert!(kana.iter().all(|ch| is_kana(*ch)));
        for ch in [
            'あ', 'ん', 'ゔ', 'ア', 'ン', 'ヴ', 'ー', '・', 'ヽ', 'ヾ', 'ゝ', 'ゞ',
        ] {
            assert!(kana.contains(&ch), "{ch}");
        }
    }

    #[test]
    fn kana_set_excludes_other_scripts() {
        for ch in ['a', '1', '漢', '、', '゠', 'ヿ', 'Ａ', '☺'] {
            assert!(!is_kana(ch), "{ch}");
        }
    }

    #[test]
    fn kanji_candidates_include_ideographs_and_ideographic_marks() {
        for ch in ['漢', '字', '々', '〆', '、', '。', '㐀', '𠮷', '﨑'] {
            assert!(is_kanji_candidate(ch), "{ch}");
        }
    }

    #[test]
    fn kanji_candidates_exclude_kana_ascii_and_symbols() {
        for ch in ['あ', 'ア', 'a', ' ', '!', 'Ａ', '☺', '「', '\n', '😀'] {
            assert!(!is_kanji_candidate(ch), "{ch}");
        }
    }

    #[test]
    fn kanji_in_text_are_unique_and_in_order_of_appearance() {
        assert_eq!(
            kanji_chars_in("漢字かな交じり文、漢字\n文"),
            vec!['漢', '字', '交', '文', '、']
        );
    }

    #[test]
    fn hangul_covers_syllables_and_compatibility_jamo() {
        for ch in ['가', '한', '힣', 'ㄱ', 'ㅎ', 'ㅏ', 'ㅣ'] {
            assert!(is_hangul(ch), "{ch}");
        }
    }

    #[test]
    fn hangul_excludes_other_scripts_and_unsupported_jamo() {
        for ch in [
            'あ', '漢', 'a', '1', '\u{1100}', '\u{3164}', '\u{d7a4}', '\u{abff}',
        ] {
            assert!(!is_hangul(ch), "{ch}");
        }
    }

    #[test]
    fn hangul_in_text_are_unique_and_in_order_of_appearance() {
        assert_eq!(
            hangul_chars_in("한글 한국어 ㄱ\n한 abc"),
            vec!['한', '글', '국', '어', 'ㄱ']
        );
        assert!(hangul_chars_in("Hello あ漢").is_empty());
    }

    #[test]
    fn text_without_kanji_yields_nothing() {
        assert!(kanji_chars_in("Hello, あいう!").is_empty());
    }
}
