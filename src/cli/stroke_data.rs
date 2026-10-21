use std::path::PathBuf;

use anyhow::{Context, bail};

use crate::dataset;
use crate::glyph::store::GlyphStore;

pub fn load_glyphs(
    text: &str,
    glyph_files: &[PathBuf],
    offline: bool,
) -> anyhow::Result<GlyphStore> {
    let mut store = dataset::load_store(text, offline).with_context(|| {
        if offline {
            "failed to read the cached stroke data"
        } else {
            "failed to prepare the stroke data; check the network connection, use --offline to rely on the cache, or specify --glyph-file <PATH>"
        }
    })?;
    store.merge(GlyphStore::from_json_files(glyph_files)?);
    if store.is_empty() {
        bail!(
            "no stroke data available; run without --offline to download it, or specify --glyph-file <PATH>"
        );
    }
    Ok(store)
}
