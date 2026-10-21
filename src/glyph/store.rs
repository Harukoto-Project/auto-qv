use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::Glyph;
use crate::error::{AutoQvError, Result};

const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
struct GlyphFile {
    version: u32,
    glyphs: HashMap<String, Glyph>,
}

#[derive(Serialize)]
struct GlyphFileRef<'a> {
    version: u32,
    glyphs: BTreeMap<String, &'a Glyph>,
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

    pub fn from_json_file(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path)
            .map_err(|e| AutoQvError::GlyphData(format!("{}: {e}", path.display())))?;
        Self::from_json_str(&json)
            .map_err(|e| AutoQvError::GlyphData(format!("{}: {e}", path.display())))
    }

    pub fn from_json_files(paths: &[PathBuf]) -> Result<Self> {
        let mut store = Self::default();
        for path in paths {
            store.merge(Self::from_json_file(path)?);
        }
        Ok(store)
    }

    pub fn insert(&mut self, ch: char, glyph: Glyph) -> Result<()> {
        if !glyph.is_valid() {
            return Err(AutoQvError::GlyphData(format!(
                "invalid glyph data for {ch:?} (U+{:04X})",
                ch as u32
            )));
        }
        self.glyphs.insert(ch, glyph);
        Ok(())
    }

    pub fn merge(&mut self, other: GlyphStore) {
        self.glyphs.extend(other.glyphs);
    }

    pub fn get(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.get(&ch)
    }

    pub fn iter(&self) -> impl Iterator<Item = (char, &Glyph)> {
        self.glyphs.iter().map(|(ch, glyph)| (*ch, glyph))
    }

    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    pub fn to_json_string(&self) -> Result<String> {
        let file = GlyphFileRef {
            version: SUPPORTED_VERSION,
            glyphs: self
                .glyphs
                .iter()
                .map(|(ch, glyph)| (ch.to_string(), glyph))
                .collect(),
        };
        serde_json::to_string(&file).map_err(|e| AutoQvError::GlyphData(e.to_string()))
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
    fn iterates_over_every_glyph() {
        let json = r#"{ "version": 1, "glyphs": {
            "a": { "advance": 0.5, "strokes": [] },
            "b": { "advance": 0.6, "strokes": [] }
        } }"#;
        let store = GlyphStore::from_json_str(json).unwrap();

        let mut chars: Vec<char> = store.iter().map(|(ch, _)| ch).collect();
        chars.sort_unstable();

        assert_eq!(chars, vec!['a', 'b']);
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
    fn missing_file_error_mentions_the_path() {
        let err = GlyphStore::from_json_file(Path::new("no-such-glyphs.json")).unwrap_err();

        assert!(err.to_string().contains("no-such-glyphs.json"));
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(GlyphStore::from_json_str("not json").is_err());
    }

    #[test]
    fn json_round_trip_preserves_glyphs() {
        let json = r#"{ "version": 1, "glyphs": {
            "b": { "advance": 0.5, "strokes": [] },
            "a": { "advance": 0.75, "strokes": [[[0.25, 0.5], [0.5, 0.125]]] }
        } }"#;
        let store = GlyphStore::from_json_str(json).unwrap();

        let restored = GlyphStore::from_json_str(&store.to_json_string().unwrap()).unwrap();

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.get('a'), store.get('a'));
        assert_eq!(restored.get('b'), store.get('b'));
    }

    #[test]
    fn serialized_glyphs_are_ordered_by_character() {
        let json = r#"{ "version": 1, "glyphs": {
            "b": { "advance": 1.0, "strokes": [] },
            "a": { "advance": 1.0, "strokes": [] }
        } }"#;
        let text = GlyphStore::from_json_str(json)
            .unwrap()
            .to_json_string()
            .unwrap();

        assert!(text.find("\"a\"").unwrap() < text.find("\"b\"").unwrap());
    }

    #[test]
    fn insert_rejects_invalid_glyphs() {
        let mut store = GlyphStore::default();
        let invalid = Glyph {
            advance: 0.0,
            strokes: vec![],
        };

        assert!(store.insert('a', invalid).is_err());
        assert!(store.is_empty());
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
