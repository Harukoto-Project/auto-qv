pub const HERSHEY_COMMIT: &str = "1356bf2f83d380fcef68c887e88675eb9d445d86";
pub const HERSHEY_ROWMANS_SHA256: &str =
    "8718fb129c0f6bce89c84fe41bc467e39534d215a6f7c3220cc5789a8a7d8618";
pub const HERSHEY_CONVERTER_VERSION: u32 = 1;

pub const KANJIVG_COMMIT: &str = "d95a97627fd9fe5b2c8d06ca81e38149609c0c1e";
pub const KANJIVG_CONVERTER_VERSION: u32 = 2;

pub const HANGUL_COMMIT: &str = "3b3082d3269548ae0032e51975bf0a3be9b67317";
pub const HANGUL_FONT_SHA256: &str =
    "22d507523da03a0798a95f7d9a1e346221e960d8b71a2b95223b6f990aded829";
pub const HANGUL_CONVERTER_VERSION: u32 = 1;

const HERSHEY_RAW_BASE: &str = "https://raw.githubusercontent.com/kamalmostafa/hershey-fonts";
const KANJIVG_RAW_BASE: &str = "https://raw.githubusercontent.com/KanjiVG/kanjivg";
const HANGUL_RAW_BASE: &str =
    "https://raw.githubusercontent.com/golanlevin/p5-single-line-font-resources";

pub fn hershey_rowmans_url(commit: &str) -> String {
    format!("{HERSHEY_RAW_BASE}/{commit}/hershey-fonts/rowmans.jhf")
}

pub fn kanjivg_url(commit: &str, ch: char) -> String {
    format!("{KANJIVG_RAW_BASE}/{commit}/kanji/{:05x}.svg", ch as u32)
}

pub fn hangul_font_url(commit: &str) -> String {
    format!(
        "{HANGUL_RAW_BASE}/{commit}/hangeul_monoline_fonts/p5_hangeul_svgfonts/svg_fonts/KOSDICHS_johab_svgfont.svg"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hangul_url_pins_the_commit() {
        assert_eq!(
            hangul_font_url("abc123"),
            "https://raw.githubusercontent.com/golanlevin/p5-single-line-font-resources/abc123/hangeul_monoline_fonts/p5_hangeul_svgfonts/svg_fonts/KOSDICHS_johab_svgfont.svg"
        );
    }

    #[test]
    fn hangul_pinned_values_have_the_expected_shape() {
        assert_eq!(HANGUL_COMMIT.len(), 40);
        assert_eq!(HANGUL_FONT_SHA256.len(), 64);
        assert!(HANGUL_COMMIT.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(
            HANGUL_FONT_SHA256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
    }

    #[test]
    fn url_pins_the_commit() {
        assert_eq!(
            hershey_rowmans_url("abc123"),
            "https://raw.githubusercontent.com/kamalmostafa/hershey-fonts/abc123/hershey-fonts/rowmans.jhf"
        );
    }

    #[test]
    fn kanjivg_url_uses_five_digit_lowercase_hex() {
        assert_eq!(
            kanjivg_url("abc123", 'あ'),
            "https://raw.githubusercontent.com/KanjiVG/kanjivg/abc123/kanji/03042.svg"
        );
        assert_eq!(
            kanjivg_url("abc123", '漢'),
            "https://raw.githubusercontent.com/KanjiVG/kanjivg/abc123/kanji/06f22.svg"
        );
        assert_eq!(
            kanjivg_url("abc123", '𠮷'),
            "https://raw.githubusercontent.com/KanjiVG/kanjivg/abc123/kanji/20bb7.svg"
        );
    }

    #[test]
    fn kanjivg_commit_has_the_expected_shape() {
        assert_eq!(KANJIVG_COMMIT.len(), 40);
        assert!(KANJIVG_COMMIT.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn pinned_values_have_the_expected_shape() {
        assert_eq!(HERSHEY_COMMIT.len(), 40);
        assert_eq!(HERSHEY_ROWMANS_SHA256.len(), 64);
        assert!(HERSHEY_COMMIT.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(
            HERSHEY_ROWMANS_SHA256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
    }
}
