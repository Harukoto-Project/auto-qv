use super::kinsoku::is_prohibited_at_line_start;
use super::model::{Layout, LayoutConfig, MissingGlyph};
use crate::glyph::model::Point;
use crate::glyph::store::GlyphStore;

const WIDTH_EPSILON: f32 = 1e-4;

pub fn layout_text(text: &str, store: &GlyphStore, config: &LayoutConfig) -> Layout {
    let mut layout = Layout::default();
    let line_pitch = config.size * (1.0 + config.line_spacing);
    let char_gap = config.size * config.char_spacing;

    let mut row = 0usize;
    let mut cursor_x = 0.0f32;

    for (line_index, line) in text.lines().enumerate() {
        if line_index > 0 {
            row += 1;
            cursor_x = 0.0;
        }

        for (column_index, ch) in line.chars().enumerate() {
            let Some(glyph) = store.get(ch) else {
                layout.missing.push(MissingGlyph {
                    ch,
                    line: line_index + 1,
                    column: column_index + 1,
                });
                continue;
            };

            let width = glyph.advance * config.size;
            let overflows =
                cursor_x > 0.0 && cursor_x + width > config.line_width_em + WIDTH_EPSILON;
            let hangs = config.kinsoku && is_prohibited_at_line_start(ch);
            if overflows && !hangs {
                row += 1;
                cursor_x = 0.0;
            }

            let origin_y = row as f32 * line_pitch;
            for stroke in &glyph.strokes {
                layout.strokes.push(
                    stroke
                        .iter()
                        .map(|p| Point {
                            x: cursor_x + p.x * config.size,
                            y: origin_y + p.y * config.size,
                        })
                        .collect(),
                );
            }

            layout.placed += 1;
            cursor_x += width + char_gap;
        }
    }

    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLYPHS: &str = r#"{
        "version": 1,
        "glyphs": {
            "a": { "advance": 1.0, "strokes": [[[0.0, 0.0], [1.0, 1.0]]] },
            "b": { "advance": 1.0, "strokes": [[[0.5, 0.0], [0.5, 1.0]]] },
            "。": { "advance": 1.0, "strokes": [[[0.5, 0.5]]] }
        }
    }"#;

    fn store() -> GlyphStore {
        GlyphStore::from_json_str(GLYPHS).unwrap()
    }

    fn config() -> LayoutConfig {
        LayoutConfig {
            size: 1.0,
            char_spacing: 0.5,
            line_spacing: 1.0,
            line_width_em: 3.0,
            kinsoku: true,
        }
    }

    fn origin(layout: &Layout, stroke_index: usize) -> Point {
        layout.strokes[stroke_index][0]
    }

    #[test]
    fn advances_cursor_by_width_and_spacing() {
        let layout = layout_text("ab", &store(), &config());

        assert_eq!(layout.strokes.len(), 2);
        assert_eq!(origin(&layout, 0), Point { x: 0.0, y: 0.0 });
        assert_eq!(origin(&layout, 1), Point { x: 2.0, y: 0.0 });
    }

    #[test]
    fn scales_glyphs_spacing_and_rows_by_size() {
        let config = LayoutConfig {
            size: 2.0,
            line_width_em: 100.0,
            ..config()
        };
        let layout = layout_text("ab\na", &store(), &config);

        assert_eq!(layout.strokes[0][1], Point { x: 2.0, y: 2.0 });
        assert_eq!(origin(&layout, 1), Point { x: 4.0, y: 0.0 });
        assert_eq!(origin(&layout, 2), Point { x: 0.0, y: 4.0 });
    }

    #[test]
    fn starts_new_row_after_newline() {
        let layout = layout_text("a\nb", &store(), &config());

        assert_eq!(origin(&layout, 1), Point { x: 0.5, y: 2.0 });
    }

    #[test]
    fn handles_crlf_and_blank_lines() {
        let layout = layout_text("a\r\n\r\nb", &store(), &config());

        assert_eq!(origin(&layout, 1), Point { x: 0.5, y: 4.0 });
    }

    #[test]
    fn wraps_when_line_width_is_exceeded() {
        let layout = layout_text("aab", &store(), &config());

        assert_eq!(origin(&layout, 1), Point { x: 1.5, y: 0.0 });
        assert_eq!(origin(&layout, 2), Point { x: 0.5, y: 2.0 });
    }

    #[test]
    fn places_oversized_glyph_at_line_start_without_wrapping_forever() {
        let config = LayoutConfig {
            line_width_em: 0.5,
            ..config()
        };
        let layout = layout_text("ab", &store(), &config);

        assert_eq!(origin(&layout, 0), Point { x: 0.0, y: 0.0 });
        assert_eq!(origin(&layout, 1), Point { x: 0.5, y: 2.0 });
    }

    #[test]
    fn kinsoku_hangs_punctuation_at_line_end() {
        let layout = layout_text("aa。", &store(), &config());

        assert_eq!(origin(&layout, 2), Point { x: 3.5, y: 0.5 });
    }

    #[test]
    fn punctuation_wraps_when_kinsoku_is_disabled() {
        let config = LayoutConfig {
            kinsoku: false,
            ..config()
        };
        let layout = layout_text("aa。", &store(), &config);

        assert_eq!(origin(&layout, 2), Point { x: 0.5, y: 2.5 });
    }

    #[test]
    fn records_missing_glyphs_without_advancing() {
        let layout = layout_text("a\nxb?", &store(), &config());

        assert_eq!(
            layout.missing,
            vec![
                MissingGlyph {
                    ch: 'x',
                    line: 2,
                    column: 1
                },
                MissingGlyph {
                    ch: '?',
                    line: 2,
                    column: 3
                },
            ]
        );
        assert_eq!(origin(&layout, 1), Point { x: 0.5, y: 2.0 });
        assert_eq!(layout.placed, 2);
    }

    #[test]
    fn reports_extent_of_placed_strokes() {
        let layout = layout_text("ab", &store(), &config());

        assert_eq!(layout.extent_em(), (2.0, 1.0));
        assert_eq!(layout.placed, 2);
    }
}
