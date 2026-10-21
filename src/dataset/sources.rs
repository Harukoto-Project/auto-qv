pub const HERSHEY_COMMIT: &str = "1356bf2f83d380fcef68c887e88675eb9d445d86";
pub const HERSHEY_ROWMANS_SHA256: &str =
    "8718fb129c0f6bce89c84fe41bc467e39534d215a6f7c3220cc5789a8a7d8618";
pub const HERSHEY_CONVERTER_VERSION: u32 = 1;

const HERSHEY_RAW_BASE: &str = "https://raw.githubusercontent.com/kamalmostafa/hershey-fonts";

pub fn hershey_rowmans_url(commit: &str) -> String {
    format!("{HERSHEY_RAW_BASE}/{commit}/hershey-fonts/rowmans.jhf")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_pins_the_commit() {
        assert_eq!(
            hershey_rowmans_url("abc123"),
            "https://raw.githubusercontent.com/kamalmostafa/hershey-fonts/abc123/hershey-fonts/rowmans.jhf"
        );
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
