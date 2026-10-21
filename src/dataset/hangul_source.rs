use super::cache::HangulEntry;
use super::charset::hangul_chars_in;
use super::fetch::Fetcher;
use super::hangul::convert_font;
use super::progress::Progress;
use super::provider::{DatasetProvider, verify_sha256};
use super::sources::{HANGUL_COMMIT, HANGUL_CONVERTER_VERSION, hangul_font_url};
use crate::error::{AutoQvError, Result};
use crate::glyph::model::Glyph;
use crate::glyph::store::GlyphStore;

const FETCH_LABEL: &str = "Fetching Hangul stroke data";
const CACHE_LABEL: &str = "Caching Hangul stroke data";

pub(super) fn current_hangul_entry() -> HangulEntry {
    HangulEntry {
        commit: HANGUL_COMMIT.to_string(),
        converter_version: HANGUL_CONVERTER_VERSION,
    }
}

impl<F: Fetcher> DatasetProvider<F> {
    pub(super) fn load_hangul(
        &self,
        text: &str,
        offline: bool,
        progress: &mut dyn Progress,
        store: &mut GlyphStore,
    ) {
        let needed = hangul_chars_in(text);
        if needed.is_empty() {
            return;
        }
        if let Some(cached) = self.cached_hangul(&needed) {
            store.merge(cached);
            return;
        }
        if offline {
            progress.notice(&format!(
                "warning: no usable cached Hangul stroke data in {} (offline mode)",
                self.cache.root().display()
            ));
            return;
        }

        let font = match self.fetch_hangul(progress) {
            Ok(font) => font,
            Err(error) => {
                progress.notice(&format!(
                    "warning: failed to fetch the Hangul stroke data ({error}); Hangul characters are treated as missing"
                ));
                return;
            }
        };

        match self.persist_hangul(&font, progress) {
            Ok(()) => progress.notice(&format!(
                "Fetched {} Hangul glyphs; cached in {}",
                font.len(),
                self.cache.root().display()
            )),
            Err(error) => progress.notice(&format!(
                "warning: failed to cache the Hangul data in {}: {error}",
                self.cache.root().display()
            )),
        }
        store.merge(subset(&font, &needed));
    }

    fn cached_hangul(&self, needed: &[char]) -> Option<GlyphStore> {
        let entry = self.cache.read_manifest()?.hangul?;
        if entry != current_hangul_entry() {
            return None;
        }
        let mut cached = GlyphStore::default();
        for ch in needed {
            cached.merge(
                GlyphStore::from_json_file(&self.cache.hangul_path(*ch))
                    .ok()
                    .filter(|loaded| loaded.get(*ch).is_some())?,
            );
        }
        Some(cached)
    }

    fn fetch_hangul(&self, progress: &mut dyn Progress) -> Result<GlyphStore> {
        let url = hangul_font_url(HANGUL_COMMIT);

        progress.begin(FETCH_LABEL);
        let fetched = self.fetcher.get(&url, &mut |received, total| {
            progress.update(received, total)
        });
        progress.finish();
        let bytes = fetched?;

        verify_sha256(&bytes, &self.hangul_sha256, "Hangul")?;
        let text = String::from_utf8(bytes)
            .map_err(|_| AutoQvError::Dataset("Hangul data is not valid UTF-8".to_string()))?;
        convert_font(&text).map_err(|message| {
            AutoQvError::Dataset(format!("failed to convert the Hangul data: {message}"))
        })
    }

    fn persist_hangul(&self, font: &GlyphStore, progress: &mut dyn Progress) -> Result<()> {
        self.cache.clear_hangul()?;

        let total = font.len() as u64;
        let mut written = Ok(());
        progress.begin(CACHE_LABEL);
        for (done, (ch, glyph)) in font.iter().enumerate() {
            written =
                single_glyph_json(ch, glyph).and_then(|json| self.cache.write_hangul(ch, &json));
            if written.is_err() {
                break;
            }
            progress.update(done as u64 + 1, Some(total));
        }
        progress.finish();
        written?;

        self.update_manifest(|manifest| manifest.hangul = Some(current_hangul_entry()))?;
        self.write_licenses()
    }
}

fn single_glyph_json(ch: char, glyph: &Glyph) -> Result<String> {
    let mut single = GlyphStore::default();
    single.insert(ch, glyph.clone())?;
    single.to_json_string()
}

fn subset(font: &GlyphStore, needed: &[char]) -> GlyphStore {
    let mut picked = GlyphStore::default();
    for ch in needed {
        if let Some(glyph) = font.get(*ch) {
            let _ = picked.insert(*ch, glyph.clone());
        }
    }
    picked
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::dataset::cache::Cache;
    use crate::dataset::provider::sha256_hex;
    use crate::dataset::test_support::{
        FakeFetcher, RecordingProgress, hangul_font_text, provider, seed_all,
    };

    fn serving_font() -> FakeFetcher {
        FakeFetcher::serving(hangul_font_text())
    }

    fn load_with(
        dir: &tempfile::TempDir,
        fetcher: &FakeFetcher,
        text: &str,
        offline: bool,
        progress: &mut RecordingProgress,
    ) -> GlyphStore {
        let mut target = provider(dir, fetcher);
        target.hangul_sha256 = sha256_hex(&hangul_font_text());
        target.load(text, offline, progress).unwrap()
    }

    fn manifest_of(dir: &tempfile::TempDir) -> crate::dataset::cache::Manifest {
        Cache::new(dir.path().to_path_buf())
            .read_manifest()
            .unwrap()
    }

    #[test]
    fn hangul_in_the_text_triggers_one_font_download() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = serving_font();
        let mut progress = RecordingProgress::default();

        let store = load_with(&dir, &fetcher, "한글 abc", false, &mut progress);

        assert_eq!(fetcher.requested(), vec![hangul_font_url(HANGUL_COMMIT)]);
        assert!(store.get('한').is_some());
        assert!(store.get('글').is_some());
        assert!(store.get('A').is_some());
        assert!(progress.events.contains(&format!("begin:{FETCH_LABEL}")));
        assert!(progress.events.contains(&format!("begin:{CACHE_LABEL}")));
        assert!(
            progress
                .notices()
                .iter()
                .any(|n| n.starts_with("Fetched 4 Hangul glyphs"))
        );
    }

    #[test]
    fn only_the_glyphs_in_the_text_are_returned() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = serving_font();

        let store = load_with(
            &dir,
            &fetcher,
            "한",
            false,
            &mut RecordingProgress::default(),
        );

        assert!(store.get('한').is_some());
        assert!(store.get('글').is_none());
    }

    #[test]
    fn every_glyph_of_the_font_is_cached_and_credited() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = serving_font();
        load_with(
            &dir,
            &fetcher,
            "한",
            false,
            &mut RecordingProgress::default(),
        );

        let cache = Cache::new(dir.path().to_path_buf());
        for ch in ['ㄱ', '가', '한', '글'] {
            assert!(cache.hangul_path(ch).exists(), "{ch}");
        }
        assert_eq!(manifest_of(&dir).hangul, Some(current_hangul_entry()));
        let licenses = fs::read_to_string(dir.path().join("LICENSES.txt")).unwrap();
        assert!(licenses.contains("Choi Samrak"));
        assert!(licenses.contains("Ulrich Apel"));
        assert!(licenses.contains("A. V. Hershey"));
    }

    #[test]
    fn a_cached_font_is_reused_without_network_access() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        load_with(
            &dir,
            &serving_font(),
            "한",
            false,
            &mut RecordingProgress::default(),
        );

        let second = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();
        let store = load_with(&dir, &second, "글 한", false, &mut progress);

        assert_eq!(second.call_count(), 0);
        assert!(progress.events.is_empty());
        assert!(store.get('한').is_some());
        assert!(store.get('글').is_some());
    }

    #[test]
    fn text_without_hangul_never_touches_the_font() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::failing();

        let store = load_with(
            &dir,
            &fetcher,
            "Hello あ",
            false,
            &mut RecordingProgress::default(),
        );

        assert_eq!(fetcher.call_count(), 0);
        assert!(store.get('한').is_none());
        assert!(manifest_of(&dir).hangul.is_none());
    }

    #[test]
    fn offline_without_a_cache_warns_and_does_not_fetch() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = load_with(&dir, &fetcher, "한", true, &mut progress);

        assert_eq!(fetcher.call_count(), 0);
        assert!(store.get('한').is_none());
        assert!(progress.notices()[0].contains("Hangul stroke data"));
        assert!(progress.notices()[0].starts_with("warning"));
    }

    #[test]
    fn offline_uses_a_valid_cache() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        load_with(
            &dir,
            &serving_font(),
            "한",
            false,
            &mut RecordingProgress::default(),
        );
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = load_with(&dir, &fetcher, "한", true, &mut progress);

        assert_eq!(fetcher.call_count(), 0);
        assert!(store.get('한').is_some());
        assert!(progress.events.is_empty());
    }

    #[test]
    fn a_download_failure_warns_and_continues() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = load_with(&dir, &fetcher, "한 abc", false, &mut progress);

        assert!(store.get('한').is_none());
        assert!(store.get('A').is_some());
        assert!(progress.notices().iter().any(|n| {
            n.starts_with("warning: failed to fetch the Hangul stroke data")
                && n.contains("network down")
        }));
        assert_eq!(progress.events.iter().filter(|e| *e == "finish").count(), 1);
        assert!(manifest_of(&dir).hangul.is_none());
    }

    #[test]
    fn a_checksum_mismatch_is_a_warning_and_nothing_is_cached() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = serving_font();
        let mut progress = RecordingProgress::default();

        let mut target = provider(&dir, &fetcher);
        target.hangul_sha256 = "0".repeat(64);
        let store = target.load("한", false, &mut progress).unwrap();

        assert!(store.get('한').is_none());
        assert!(
            progress
                .notices()
                .iter()
                .any(|n| n.contains("checksum mismatch"))
        );
        assert!(manifest_of(&dir).hangul.is_none());
        assert!(
            !Cache::new(dir.path().to_path_buf())
                .hangul_path('한')
                .exists()
        );
    }

    #[test]
    fn an_unusable_font_is_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let body = b"<html>rate limited</html>".to_vec();
        let fetcher = FakeFetcher::serving(body.clone());
        let mut progress = RecordingProgress::default();

        let mut target = provider(&dir, &fetcher);
        target.hangul_sha256 = sha256_hex(&body);
        let store = target.load("한", false, &mut progress).unwrap();

        assert!(store.get('한').is_none());
        assert!(
            progress
                .notices()
                .iter()
                .any(|n| n.contains("failed to convert the Hangul data"))
        );
    }

    #[test]
    fn a_corrupt_cached_file_triggers_a_new_download() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        load_with(
            &dir,
            &serving_font(),
            "한",
            false,
            &mut RecordingProgress::default(),
        );
        let cache = Cache::new(dir.path().to_path_buf());
        fs::write(cache.hangul_path('한'), "broken").unwrap();
        let fetcher = serving_font();

        let store = load_with(
            &dir,
            &fetcher,
            "한",
            false,
            &mut RecordingProgress::default(),
        );

        assert_eq!(fetcher.call_count(), 1);
        assert!(store.get('한').is_some());
        assert!(
            GlyphStore::from_json_file(&cache.hangul_path('한'))
                .unwrap()
                .get('한')
                .is_some()
        );
    }

    #[test]
    fn a_missing_cached_file_triggers_a_new_download() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        load_with(
            &dir,
            &serving_font(),
            "한",
            false,
            &mut RecordingProgress::default(),
        );
        let cache = Cache::new(dir.path().to_path_buf());
        fs::remove_file(cache.hangul_path('글')).unwrap();
        let fetcher = serving_font();

        load_with(
            &dir,
            &fetcher,
            "한글",
            false,
            &mut RecordingProgress::default(),
        );

        assert_eq!(fetcher.call_count(), 1);
        assert!(cache.hangul_path('글').exists());
    }

    #[test]
    fn an_outdated_entry_discards_the_cached_files() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        load_with(
            &dir,
            &serving_font(),
            "한",
            false,
            &mut RecordingProgress::default(),
        );
        let cache = Cache::new(dir.path().to_path_buf());
        fs::write(dir.path().join("hangul").join("stale.json"), "x").unwrap();
        let mut manifest = cache.read_manifest().unwrap();
        manifest.hangul.as_mut().unwrap().converter_version += 1;
        cache.write_manifest(&manifest).unwrap();
        let fetcher = serving_font();

        load_with(
            &dir,
            &fetcher,
            "한",
            false,
            &mut RecordingProgress::default(),
        );

        assert_eq!(fetcher.call_count(), 1);
        assert!(!dir.path().join("hangul").join("stale.json").exists());
        assert_eq!(manifest_of(&dir).hangul, Some(current_hangul_entry()));
    }

    #[test]
    fn a_cache_write_failure_still_returns_the_fetched_data() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        fs::write(dir.path().join("hangul"), "a file, not a directory").unwrap();
        let fetcher = serving_font();
        let mut progress = RecordingProgress::default();

        let store = load_with(&dir, &fetcher, "한", false, &mut progress);

        assert!(store.get('한').is_some());
        assert!(
            progress
                .notices()
                .iter()
                .any(|n| n.starts_with("warning: failed to cache the Hangul data"))
        );
        assert!(manifest_of(&dir).hangul.is_none());
    }
}
