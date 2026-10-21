use std::collections::HashMap;
use std::fmt;

use crate::layout::model::Layout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingChar {
    pub ch: char,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    pub placed: usize,
    pub missing_total: usize,
    pub missing: Vec<MissingChar>,
}

impl CheckReport {
    pub fn from_layout(layout: &Layout) -> Self {
        let mut missing: Vec<MissingChar> = Vec::new();
        let mut index_by_char: HashMap<char, usize> = HashMap::new();
        for glyph in &layout.missing {
            match index_by_char.get(&glyph.ch) {
                Some(&index) => missing[index].count += 1,
                None => {
                    index_by_char.insert(glyph.ch, missing.len());
                    missing.push(MissingChar {
                        ch: glyph.ch,
                        count: 1,
                    });
                }
            }
        }
        Self {
            placed: layout.placed,
            missing_total: layout.missing.len(),
            missing,
        }
    }
}

impl fmt::Display for CheckReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Characters checked : {}",
            self.placed + self.missing_total
        )?;
        writeln!(f, "With stroke data   : {}", self.placed)?;
        if self.missing.is_empty() {
            return writeln!(f, "Missing glyphs     : 0");
        }
        writeln!(
            f,
            "Missing glyphs     : {} ({} unique)",
            self.missing_total,
            self.missing.len()
        )?;
        for entry in &self.missing {
            writeln!(
                f,
                "  {:?} (U+{:04X}) x{}",
                entry.ch, entry.ch as u32, entry.count
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::model::MissingGlyph;

    fn layout_with(placed: usize, missing: &[(char, usize, usize)]) -> Layout {
        Layout {
            strokes: Vec::new(),
            missing: missing
                .iter()
                .map(|&(ch, line, column)| MissingGlyph { ch, line, column })
                .collect(),
            placed,
        }
    }

    #[test]
    fn groups_missing_characters_in_order_of_first_appearance() {
        let layout = layout_with(4, &[('b', 1, 2), ('a', 1, 3), ('b', 2, 1), ('b', 2, 4)]);

        let report = CheckReport::from_layout(&layout);

        assert_eq!(report.missing_total, 4);
        assert_eq!(
            report.missing,
            vec![
                MissingChar { ch: 'b', count: 3 },
                MissingChar { ch: 'a', count: 1 },
            ]
        );
    }

    #[test]
    fn formats_a_report_without_missing_glyphs() {
        let text = CheckReport::from_layout(&layout_with(5, &[])).to_string();

        assert_eq!(
            text,
            "Characters checked : 5\nWith stroke data   : 5\nMissing glyphs     : 0\n"
        );
    }

    #[test]
    fn formats_unique_missing_characters_with_counts() {
        let layout = layout_with(2, &[('漢', 1, 1), ('x', 1, 2), ('漢', 2, 1)]);

        let text = CheckReport::from_layout(&layout).to_string();

        assert!(text.contains("Characters checked : 5"));
        assert!(text.contains("With stroke data   : 2"));
        assert!(text.contains("Missing glyphs     : 3 (2 unique)"));
        assert!(text.contains("  '漢' (U+6F22) x2"));
        assert!(text.contains("  'x' (U+0078) x1"));
    }
}
