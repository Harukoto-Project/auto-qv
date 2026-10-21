use std::fs;
use std::path::Path;

use anyhow::{Context, bail};

use super::args::TextSourceArgs;

pub fn read_text(source: &TextSourceArgs) -> anyhow::Result<String> {
    match (&source.text, &source.file) {
        (Some(text), _) => Ok(text.clone()),
        (None, Some(path)) => read_text_file(path),
        (None, None) => bail!("specify the text or --file <PATH>"),
    }
}

fn read_text_file(path: &Path) -> anyhow::Result<String> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read {} as UTF-8 text", path.display()))?;
    Ok(match text.strip_prefix('\u{feff}') {
        Some(stripped) => stripped.to_string(),
        None => text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_utf8_text_and_strips_a_byte_order_mark() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text.txt");
        fs::write(&path, "\u{feff}こんにちは\nabc").unwrap();

        assert_eq!(read_text_file(&path).unwrap(), "こんにちは\nabc");
    }

    #[test]
    fn missing_text_file_reports_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("none.txt");

        let error = read_text_file(&path).unwrap_err();

        assert!(error.to_string().contains("none.txt"));
    }

    #[test]
    fn non_utf8_text_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sjis.txt");
        fs::write(&path, [0x82, 0xA0]).unwrap();

        assert!(read_text_file(&path).is_err());
    }

    #[test]
    fn inline_text_takes_effect_without_a_file() {
        let source = TextSourceArgs {
            text: Some("abc".to_string()),
            file: None,
        };

        assert_eq!(read_text(&source).unwrap(), "abc");
    }
}
