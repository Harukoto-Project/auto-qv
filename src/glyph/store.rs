use std::collections::HashMap;

use serde::Deserialize;

use super::model::Glyph;
use crate::error::{AutoQvError, Result};

const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
struct GlyphFile {
    version: u32,
    glyphs: HashMap<String, Glyph>,
}

#[derive(Debug, Default)]
pub struct GlyphStore {
    glyphs: HashMap<char, Glyph>,
}

impl GlyphStore {
    pub fn from_json_str(json: &str) -> Result<Self> {
        let file: GlyphFile =
            serde_json::from_str(json).map_err(|e| AutoQvError::GlyphData(e.to_string()))?;

        if file.version != SUPPORTED_VERSION {
            return Err(AutoQvError::GlyphData(format!(
                "unsupported version {} (expected {})",
                file.version, SUPPORTED_VERSION
            )));
        }

        let mut glyphs = HashMap::with_capacity(file.glyphs.len());
        for (key, glyph) in file.glyphs {
            let mut chars = key.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                return Err(AutoQvError::GlyphData(format!(
                    "glyph key must be exactly one character: {key:?}"
                )));
            };
            if !glyph.is_valid() {
                return Err(AutoQvError::GlyphData(format!(
                    "invalid glyph data for {ch:?} (U+{:04X})",
                    ch as u32
                )));
            }
            glyphs.insert(ch, glyph);
        }

        Ok(Self { glyphs })
    }

    pub fn merge(&mut self, other: GlyphStore) {
        self.glyphs.extend(other.glyphs);
    }

    pub fn get(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.get(&ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::model::Point;

    #[test]
    fn loads_valid_glyphs() {
        let json = r#"{
            "version": 1,
            "glyphs": {
                "あ": { "advance": 1.0, "strokes": [[[0.2, 0.3], [0.5, 0.31]], [[0.1, 0.9]]] },
                " ": { "advance": 0.5, "strokes": [] }
            }
        }"#;
        let store = GlyphStore::from_json_str(json).unwrap();

        let glyph = store.get('あ').unwrap();
        assert_eq!(glyph.advance, 1.0);
        assert_eq!(glyph.strokes.len(), 2);
        assert_eq!(glyph.strokes[0][1], Point { x: 0.5, y: 0.31 });
        assert!(store.get(' ').unwrap().strokes.is_empty());
        assert!(store.get('い').is_none());
    }

    #[test]
    fn rejects_unsupported_version() {
        let json = r#"{ "version": 2, "glyphs": {} }"#;
        assert!(GlyphStore::from_json_str(json).is_err());
    }

    #[test]
    fn rejects_multi_character_key() {
        let json = r#"{ "version": 1, "glyphs": { "ab": { "advance": 1.0, "strokes": [] } } }"#;
        assert!(GlyphStore::from_json_str(json).is_err());
    }

    #[test]
    fn rejects_non_positive_advance() {
        let json = r#"{ "version": 1, "glyphs": { "a": { "advance": 0.0, "strokes": [] } } }"#;
        assert!(GlyphStore::from_json_str(json).is_err());
    }

    #[test]
    fn rejects_empty_stroke() {
        let json = r#"{ "version": 1, "glyphs": { "a": { "advance": 1.0, "strokes": [[]] } } }"#;
        assert!(GlyphStore::from_json_str(json).is_err());
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(GlyphStore::from_json_str("not json").is_err());
    }

    #[test]
    fn merge_overrides_existing_glyphs() {
        let base = r#"{ "version": 1, "glyphs": { "a": { "advance": 1.0, "strokes": [] } } }"#;
        let extra = r#"{ "version": 1, "glyphs": {
            "a": { "advance": 2.0, "strokes": [] },
            "b": { "advance": 1.0, "strokes": [] }
        } }"#;
        let mut store = GlyphStore::from_json_str(base).unwrap();
        store.merge(GlyphStore::from_json_str(extra).unwrap());

        assert_eq!(store.get('a').unwrap().advance, 2.0);
        assert!(store.get('b').is_some());
    }
}
