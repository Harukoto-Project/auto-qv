use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::cache::{Cache, HersheyEntry, Manifest};
use super::fetch::Fetcher;
use super::hershey::convert_rowmans;
use super::progress::Progress;
use super::sources::{
    HERSHEY_COMMIT, HERSHEY_CONVERTER_VERSION, HERSHEY_ROWMANS_SHA256, hershey_rowmans_url,
};
use crate::error::{AutoQvError, Result};
use crate::glyph::store::GlyphStore;

const HERSHEY_LABEL: &str = "Fetching Hershey Roman Simplex";
const LICENSES_HEADER: &str = "Licenses and credits for the stroke data cached by AutoQV.\n";
const HERSHEY_NOTICE: &str = include_str!("assets/hershey-notice.txt");

pub struct DatasetProvider<F: Fetcher> {
    cache: Cache,
    fetcher: F,
    hershey_sha256: String,
}

impl<F: Fetcher> DatasetProvider<F> {
    pub fn new(cache: Cache, fetcher: F) -> Self {
        Self {
            cache,
            fetcher,
            hershey_sha256: HERSHEY_ROWMANS_SHA256.to_string(),
        }
    }

    pub fn load(&self, offline: bool, progress: &mut dyn Progress) -> Result<GlyphStore> {
        if let Some(store) = self.cached_hershey() {
            return Ok(store);
        }
        if offline {
            progress.notice(&format!(
                "warning: no usable cached stroke data in {} (offline mode)",
                self.cache.root().display()
            ));
            return Ok(GlyphStore::default());
        }
        self.fetch_hershey(progress)
    }

    fn cached_hershey(&self) -> Option<GlyphStore> {
        let manifest = self.cache.read_manifest()?;
        if manifest.hershey? != current_hershey_entry() {
            return None;
        }
        GlyphStore::from_json_file(&self.cache.hershey_path()).ok()
    }

    fn fetch_hershey(&self, progress: &mut dyn Progress) -> Result<GlyphStore> {
        let url = hershey_rowmans_url(HERSHEY_COMMIT);

        progress.begin(HERSHEY_LABEL);
        let fetched = self.fetcher.get(&url, &mut |received, total| {
            progress.update(received, total)
        });
        progress.finish();
        let bytes = fetched?;

        verify_sha256(&bytes, &self.hershey_sha256)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| AutoQvError::Dataset("Hershey data is not valid UTF-8".to_string()))?;
        let store = convert_rowmans(&text)?;

        match self.persist_hershey(&store) {
            Ok(()) => progress.notice(&format!(
                "Fetched {} glyphs; cached in {}",
                store.len(),
                self.cache.root().display()
            )),
            Err(error) => progress.notice(&format!(
                "warning: failed to cache the stroke data in {}: {error}",
                self.cache.root().display()
            )),
        }
        Ok(store)
    }

    fn persist_hershey(&self, store: &GlyphStore) -> Result<()> {
        self.cache.write_hershey(&store.to_json_string()?)?;
        self.cache
            .write_licenses(&format!("{LICENSES_HEADER}\n{HERSHEY_NOTICE}"))?;
        let fetched_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(std::io::Error::other)?;
        self.cache
            .write_manifest(&Manifest::new(fetched_at, Some(current_hershey_entry())))
    }
}

fn current_hershey_entry() -> HersheyEntry {
    HersheyEntry {
        commit: HERSHEY_COMMIT.to_string(),
        converter_version: HERSHEY_CONVERTER_VERSION,
    }
}

fn verify_sha256(bytes: &[u8], expected: &str) -> Result<()> {
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(AutoQvError::Dataset(format!(
            "checksum mismatch for the downloaded Hershey data (expected {expected}, got {actual})"
        )))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::fs;

    use super::*;

    struct FakeFetcher {
        body: Vec<u8>,
        fail: bool,
        calls: Cell<usize>,
        urls: RefCell<Vec<String>>,
    }

    impl FakeFetcher {
        fn serving(body: Vec<u8>) -> Self {
            Self {
                body,
                fail: false,
                calls: Cell::new(0),
                urls: RefCell::new(Vec::new()),
            }
        }

        fn failing() -> Self {
            Self {
                fail: true,
                ..Self::serving(Vec::new())
            }
        }
    }

    impl Fetcher for &FakeFetcher {
        fn get(&self, url: &str, on_progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>> {
            self.calls.set(self.calls.get() + 1);
            self.urls.borrow_mut().push(url.to_string());
            if self.fail {
                return Err(AutoQvError::Dataset("network down".to_string()));
            }
            on_progress(self.body.len() as u64, Some(self.body.len() as u64));
            Ok(self.body.clone())
        }
    }

    #[derive(Default)]
    struct RecordingProgress {
        events: Vec<String>,
    }

    impl Progress for RecordingProgress {
        fn begin(&mut self, label: &str) {
            self.events.push(format!("begin:{label}"));
        }
        fn update(&mut self, _received: u64, _total: Option<u64>) {
            self.events.push("update".to_string());
        }
        fn finish(&mut self) {
            self.events.push("finish".to_string());
        }
        fn notice(&mut self, message: &str) {
            self.events.push(format!("notice:{message}"));
        }
    }

    fn rowmans_text() -> Vec<u8> {
        vec!["  699  1JZ"; 95].join("\n").into_bytes()
    }

    fn provider<'a>(
        dir: &tempfile::TempDir,
        fetcher: &'a FakeFetcher,
    ) -> DatasetProvider<&'a FakeFetcher> {
        DatasetProvider::new(Cache::new(dir.path().to_path_buf()), fetcher)
    }

    fn seed_cache(dir: &tempfile::TempDir) {
        let cache = Cache::new(dir.path().to_path_buf());
        let store = convert_rowmans(&String::from_utf8(rowmans_text()).unwrap()).unwrap();
        let fetcher = FakeFetcher::failing();
        let provider = DatasetProvider::new(cache, &fetcher);
        provider.persist_hershey(&store).unwrap();
    }

    #[test]
    fn sha256_matches_a_known_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_rejects_a_different_digest() {
        assert!(verify_sha256(b"abc", &"0".repeat(64)).is_err());
        assert!(
            verify_sha256(
                b"abc",
                "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
            )
            .is_ok()
        );
    }

    #[test]
    fn fresh_cache_is_reused_without_network_access() {
        let dir = tempfile::tempdir().unwrap();
        seed_cache(&dir);
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher).load(false, &mut progress).unwrap();

        assert_eq!(store.len(), 95);
        assert_eq!(fetcher.calls.get(), 0);
        assert!(progress.events.is_empty());
    }

    #[test]
    fn offline_without_cache_returns_an_empty_store_and_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher).load(true, &mut progress).unwrap();

        assert!(store.is_empty());
        assert_eq!(fetcher.calls.get(), 0);
        assert!(progress.events[0].starts_with("notice:warning"));
    }

    #[test]
    fn offline_uses_a_valid_cache() {
        let dir = tempfile::tempdir().unwrap();
        seed_cache(&dir);
        let fetcher = FakeFetcher::failing();

        let store = provider(&dir, &fetcher)
            .load(true, &mut RecordingProgress::default())
            .unwrap();

        assert_eq!(store.len(), 95);
    }

    #[test]
    fn corrupt_glyph_file_triggers_a_refetch_attempt() {
        let dir = tempfile::tempdir().unwrap();
        seed_cache(&dir);
        fs::write(
            Cache::new(dir.path().to_path_buf()).hershey_path(),
            "broken",
        )
        .unwrap();
        let fetcher = FakeFetcher::failing();

        let result = provider(&dir, &fetcher).load(false, &mut RecordingProgress::default());

        assert!(result.is_err());
        assert_eq!(fetcher.calls.get(), 1);
    }

    #[test]
    fn outdated_manifest_triggers_a_refetch_attempt() {
        let dir = tempfile::tempdir().unwrap();
        seed_cache(&dir);
        let cache = Cache::new(dir.path().to_path_buf());
        cache
            .write_manifest(&Manifest::new(
                "x".to_string(),
                Some(HersheyEntry {
                    commit: "old".to_string(),
                    converter_version: HERSHEY_CONVERTER_VERSION,
                }),
            ))
            .unwrap();
        let fetcher = FakeFetcher::failing();

        let result = provider(&dir, &fetcher).load(false, &mut RecordingProgress::default());

        assert!(result.is_err());
        assert_eq!(fetcher.calls.get(), 1);
    }

    #[test]
    fn fetch_failure_is_an_error_and_finishes_the_progress() {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let result = provider(&dir, &fetcher).load(false, &mut progress);

        assert!(result.is_err());
        assert_eq!(progress.events.last().map(String::as_str), Some("finish"));
        assert!(fetcher.urls.borrow()[0].contains(HERSHEY_COMMIT));
    }

    #[test]
    fn checksum_mismatch_is_an_error_and_nothing_is_cached() {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = FakeFetcher::serving(rowmans_text());

        let result = provider(&dir, &fetcher).load(false, &mut RecordingProgress::default());

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("checksum mismatch")
        );
        assert!(
            Cache::new(dir.path().to_path_buf())
                .read_manifest()
                .is_none()
        );
    }

    #[test]
    fn persist_writes_glyphs_manifest_and_licenses() {
        let dir = tempfile::tempdir().unwrap();
        seed_cache(&dir);
        let cache = Cache::new(dir.path().to_path_buf());

        assert_eq!(
            cache.read_manifest().unwrap().hershey,
            Some(current_hershey_entry())
        );
        let licenses = fs::read_to_string(dir.path().join("LICENSES.txt")).unwrap();
        assert!(licenses.contains("A. V. Hershey"));
        assert!(licenses.contains("James Hurt"));
    }

    #[test]
    fn successful_fetch_converts_caches_and_reports_the_result() {
        let dir = tempfile::tempdir().unwrap();
        let body = rowmans_text();
        let fetcher = FakeFetcher::serving(body.clone());
        let mut target = provider(&dir, &fetcher);
        target.hershey_sha256 = sha256_hex(&body);
        let mut progress = RecordingProgress::default();

        let store = target.load(false, &mut progress).unwrap();

        assert_eq!(store.len(), 95);
        assert_eq!(progress.events[0], format!("begin:{HERSHEY_LABEL}"));
        assert!(progress.events.contains(&"finish".to_string()));
        assert!(
            progress
                .events
                .last()
                .unwrap()
                .starts_with("notice:Fetched 95 glyphs")
        );

        let second = FakeFetcher::failing();
        let cached = provider(&dir, &second)
            .load(false, &mut RecordingProgress::default())
            .unwrap();
        assert_eq!(cached.len(), 95);
        assert_eq!(second.calls.get(), 0);
    }

    #[test]
    fn cache_write_failure_still_returns_the_fetched_data() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocked");
        fs::write(&blocker, "file, not a directory").unwrap();
        let body = rowmans_text();
        let fetcher = FakeFetcher::serving(body.clone());
        let mut target = DatasetProvider::new(Cache::new(blocker), &fetcher);
        target.hershey_sha256 = sha256_hex(&body);
        let mut progress = RecordingProgress::default();

        let store = target.load(false, &mut progress).unwrap();

        assert_eq!(store.len(), 95);
        assert!(
            progress
                .events
                .last()
                .unwrap()
                .starts_with("notice:warning: failed to cache")
        );
    }
}
