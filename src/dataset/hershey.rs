use crate::error::{AutoQvError, Result};
use crate::glyph::model::{Glyph, Point};
use crate::glyph::store::GlyphStore;

const FIRST_CHAR: u32 = 0x20;
const LAST_CHAR: u32 = 0x7e;
const GLYPH_COUNT: usize = (LAST_CHAR - FIRST_CHAR + 1) as usize;
const COORD_ORIGIN: i32 = b'R' as i32;
const UNITS_PER_EM: f32 = 32.0;
const Y_OFFSET_UNITS: i32 = 16;
const PEN_UP_MARKER: [u8; 2] = *b" R";
const HEADER_LEN: usize = 8;

pub fn convert_rowmans(text: &str) -> Result<GlyphStore> {
    let mut store = GlyphStore::default();
    let mut count = 0;

    for (index, line) in text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        if index >= GLYPH_COUNT {
            break;
        }
        let ch = char::from_u32(FIRST_CHAR + index as u32).expect("ASCII range");
        let glyph = parse_glyph_line(line).map_err(|reason| {
            AutoQvError::Dataset(format!(
                "Hershey data line {} ({ch:?}): {reason}",
                index + 1
            ))
        })?;
        store.insert(ch, glyph)?;
        count += 1;
    }

    if count < GLYPH_COUNT {
        return Err(AutoQvError::Dataset(format!(
            "Hershey data has {count} glyphs (expected at least {GLYPH_COUNT})"
        )));
    }
    Ok(store)
}

fn parse_glyph_line(line: &str) -> std::result::Result<Glyph, String> {
    if !line.is_ascii() || line.len() < HEADER_LEN + 2 {
        return Err("malformed line".to_string());
    }
    let vertex_count: usize = line[5..HEADER_LEN]
        .trim()
        .parse()
        .map_err(|_| "invalid vertex count".to_string())?;
    let body = &line.as_bytes()[HEADER_LEN..];
    if vertex_count < 1 || body.len() != vertex_count * 2 {
        return Err(format!(
            "vertex count {vertex_count} does not match the data length"
        ));
    }

    let left = coord(body[0]);
    let right = coord(body[1]);
    if right <= left {
        return Err("invalid glyph width".to_string());
    }

    let mut strokes = Vec::new();
    let mut current = Vec::new();
    let (pairs, _) = body[2..].as_chunks::<2>();
    for pair in pairs {
        if *pair == PEN_UP_MARKER {
            if !current.is_empty() {
                strokes.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(Point {
            x: (coord(pair[0]) - left) as f32 / UNITS_PER_EM,
            y: (coord(pair[1]) + Y_OFFSET_UNITS) as f32 / UNITS_PER_EM,
        });
    }
    if !current.is_empty() {
        strokes.push(current);
    }

    Ok(Glyph {
        advance: (right - left) as f32 / UNITS_PER_EM,
        strokes,
    })
}

fn coord(byte: u8) -> i32 {
    i32::from(byte) - COORD_ORIGIN
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACE: &str = "  699  1JZ";
    const LETTER_A: &str = "  501  9I[RFJ[ RRFZ[ RMTWT";

    fn full_text(replacement: Option<(usize, &str)>) -> String {
        (0..GLYPH_COUNT)
            .map(|index| match replacement {
                Some((target, line)) if target == index => line,
                _ => SPACE,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn space_has_width_and_no_strokes() {
        let glyph = parse_glyph_line(SPACE).unwrap();

        assert_eq!(glyph.advance, 0.5);
        assert!(glyph.strokes.is_empty());
    }

    #[test]
    fn letter_is_scaled_into_the_em_box() {
        let glyph = parse_glyph_line(LETTER_A).unwrap();

        assert_eq!(glyph.advance, 18.0 / 32.0);
        assert_eq!(glyph.strokes.len(), 3);
        assert_eq!(
            glyph.strokes[0],
            vec![
                Point {
                    x: 9.0 / 32.0,
                    y: 4.0 / 32.0
                },
                Point {
                    x: 1.0 / 32.0,
                    y: 25.0 / 32.0
                },
            ]
        );
        assert!(glyph.is_valid());
    }

    #[test]
    fn pen_up_marker_splits_strokes() {
        let glyph = parse_glyph_line("  725  6E_RIR[ RIR[R").unwrap();

        assert_eq!(glyph.strokes.len(), 2);
        assert!(glyph.strokes.iter().all(|stroke| stroke.len() == 2));
    }

    #[test]
    fn rejects_mismatched_vertex_count() {
        assert!(parse_glyph_line("  501 10I[RFJ[ RRFZ[ RMTWT").is_err());
    }

    #[test]
    fn rejects_non_ascii_and_short_lines() {
        assert!(parse_glyph_line("  699  1JあZ").is_err());
        assert!(parse_glyph_line("  699").is_err());
    }

    #[test]
    fn converts_ascii_glyphs_in_order() {
        let store = convert_rowmans(&full_text(Some((33, LETTER_A)))).unwrap();

        assert_eq!(store.len(), GLYPH_COUNT);
        assert_eq!(store.get('A').unwrap().strokes.len(), 3);
        assert!(store.get(' ').unwrap().strokes.is_empty());
        assert!(store.get('~').is_some());
        assert!(store.get('\u{7f}').is_none());
    }

    #[test]
    fn ignores_glyphs_after_the_ascii_range() {
        let text = format!("{}\n{LETTER_A}\n", full_text(None));

        let store = convert_rowmans(&text).unwrap();

        assert_eq!(store.len(), GLYPH_COUNT);
    }

    #[test]
    fn accepts_crlf_line_endings() {
        let text = full_text(None).replace('\n', "\r\n");

        assert_eq!(convert_rowmans(&text).unwrap().len(), GLYPH_COUNT);
    }

    #[test]
    fn too_few_glyphs_is_an_error() {
        assert!(convert_rowmans("  699  1JZ\n").is_err());
    }

    #[test]
    fn broken_line_error_mentions_the_character() {
        let error = convert_rowmans(&full_text(Some((33, "garbage")))).unwrap_err();

        assert!(error.to_string().contains("'A'"));
    }
}
