use serde::Serialize;

use crate::error::Result;
use crate::layout::model::MissingGlyph;

#[derive(Debug, Serialize)]
struct MissingGlyphRecord<'a> {
    timestamp: &'a str,
    #[serde(rename = "char")]
    ch: String,
    codepoint: String,
    line: usize,
    column: usize,
}

pub fn to_json_lines(glyphs: &[MissingGlyph], timestamp: &str) -> Result<String> {
    let mut output = String::new();
    for glyph in glyphs {
        let record = MissingGlyphRecord {
            timestamp,
            ch: glyph.ch.to_string(),
            codepoint: format!("U+{:04X}", glyph.ch as u32),
            line: glyph.line,
            column: glyph.column,
        };
        output.push_str(&serde_json::to_string(&record).map_err(std::io::Error::from)?);
        output.push('\n');
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(ch: char, line: usize, column: usize) -> MissingGlyph {
        MissingGlyph { ch, line, column }
    }

    #[test]
    fn writes_one_json_object_per_line() {
        let text = to_json_lines(
            &[glyph('ん', 1, 2), glyph('𠮷', 3, 4)],
            "2026-10-22T00:00:00Z",
        )
        .unwrap();

        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            r#"{"timestamp":"2026-10-22T00:00:00Z","char":"ん","codepoint":"U+3093","line":1,"column":2}"#
        );
        assert!(lines[1].contains(r#""codepoint":"U+20BB7""#));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn escapes_control_characters_so_each_record_stays_on_one_line() {
        let text = to_json_lines(&[glyph('\u{1}', 1, 1)], "t").unwrap();

        assert_eq!(text.lines().count(), 1);
        assert!(text.contains(r#""char":"\u0001""#));
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert_eq!(to_json_lines(&[], "t").unwrap(), "");
    }
}
