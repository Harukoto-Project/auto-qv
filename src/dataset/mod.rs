pub mod batch;
pub mod cache;
pub mod charset;
pub mod fetch;
pub mod hershey;
pub mod kanjivg;
pub mod kanjivg_source;
pub mod progress;
pub mod provider;
pub mod sources;
pub mod svg_path;
#[cfg(test)]
mod test_support;

use crate::error::Result;
use crate::glyph::store::GlyphStore;
use cache::Cache;
use fetch::HttpFetcher;
use progress::StderrProgress;
use provider::DatasetProvider;

pub fn load_store(text: &str, offline: bool) -> Result<GlyphStore> {
    let provider = DatasetProvider::new(Cache::at_default_location()?, HttpFetcher::new());
    provider.load(text, offline, &mut StderrProgress::new())
}
