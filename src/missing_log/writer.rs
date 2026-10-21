use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::record::to_json_lines;
use crate::config::paths::app_data_dir;
use crate::error::Result;
use crate::layout::model::MissingGlyph;

pub fn record_missing_glyphs(
    configured: &Path,
    glyphs: &[MissingGlyph],
) -> Result<Option<PathBuf>> {
    if glyphs.is_empty() {
        return Ok(None);
    }
    let path = resolve_log_path(configured, &app_data_dir()?);
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(std::io::Error::other)?;
    append_missing_glyphs(&path, glyphs, &timestamp)?;
    Ok(Some(path))
}

pub fn resolve_log_path(configured: &Path, base: &Path) -> PathBuf {
    if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        base.join(configured)
    }
}

pub fn append_missing_glyphs(path: &Path, glyphs: &[MissingGlyph], timestamp: &str) -> Result<()> {
    let text = to_json_lines(glyphs, timestamp)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(ch: char, column: usize) -> MissingGlyph {
        MissingGlyph {
            ch,
            line: 1,
            column,
        }
    }

    #[test]
    fn relative_paths_are_placed_under_the_base_directory() {
        let base = PathBuf::from(r"C:\data");

        assert_eq!(
            resolve_log_path(Path::new("missing.log"), &base),
            base.join("missing.log")
        );
    }

    #[test]
    fn absolute_paths_are_kept() {
        let base = PathBuf::from(r"C:\data");

        assert_eq!(
            resolve_log_path(Path::new(r"D:\logs\missing.log"), &base),
            PathBuf::from(r"D:\logs\missing.log")
        );
    }

    #[test]
    fn appends_records_and_creates_missing_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("missing.log");

        append_missing_glyphs(&path, &[glyph('ん', 1)], "t1").unwrap();
        append_missing_glyphs(&path, &[glyph('に', 2), glyph('ち', 3)], "t2").unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains(r#""timestamp":"t1""#) && lines[0].contains("ん"));
        assert!(lines[2].contains(r#""timestamp":"t2""#) && lines[2].contains("ち"));
    }

    #[test]
    fn nothing_is_written_when_no_glyph_is_missing() {
        assert_eq!(
            record_missing_glyphs(Path::new("x.log"), &[]).unwrap(),
            None
        );
    }
}
