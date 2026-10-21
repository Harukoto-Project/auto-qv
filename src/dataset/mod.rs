pub mod cache;
pub mod fetch;
pub mod hershey;
pub mod progress;
pub mod provider;
pub mod sources;

use crate::error::Result;
use crate::glyph::store::GlyphStore;
use cache::Cache;
use fetch::HttpFetcher;
use progress::StderrProgress;
use provider::DatasetProvider;

pub fn load_store(offline: bool) -> Result<GlyphStore> {
    let provider = DatasetProvider::new(Cache::at_default_location()?, HttpFetcher::new());
    provider.load(offline, &mut StderrProgress::new())
}
