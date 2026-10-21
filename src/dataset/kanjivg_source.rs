use std::collections::BTreeSet;

use super::batch::{DEFAULT_WORKERS, ItemResult, fetch_batch};
use super::cache::KanjivgEntry;
use super::charset::{is_kana, kana_chars, kanji_chars_in};
use super::fetch::Fetcher;
use super::kanjivg::convert_svg;
use super::progress::Progress;
use super::provider::DatasetProvider;
use super::sources::{KANJIVG_COMMIT, KANJIVG_CONVERTER_VERSION, kanjivg_url};
use crate::error::{AutoQvError, Result};
use crate::glyph::store::GlyphStore;

const KANJIVG_LABEL: &str = "Fetching KanjiVG stroke data";
const SKIPPED_MESSAGE: &str = "skipped after an earlier failure";

pub(super) fn current_kanjivg_entry() -> KanjivgEntry {
    KanjivgEntry {
        commit: KANJIVG_COMMIT.to_string(),
        converter_version: KANJIVG_CONVERTER_VERSION,
        not_found: Vec::new(),
    }
}

fn is_current(entry: &KanjivgEntry) -> bool {
    entry.commit == KANJIVG_COMMIT && entry.converter_version == KANJIVG_CONVERTER_VERSION
}

impl<F: Fetcher> DatasetProvider<F> {
    pub(super) fn load_kanjivg(
        &self,
        text: &str,
        offline: bool,
        progress: &mut dyn Progress,
        store: &mut GlyphStore,
    ) -> Result<()> {
        let entry = self
            .cache
            .read_manifest()
            .and_then(|manifest| manifest.kanjivg)
            .filter(is_current);
        let cache_valid = entry.is_some();
        let known_not_found: BTreeSet<u32> = entry
            .map(|entry| entry.not_found.into_iter().collect())
            .unwrap_or_default();

        let mut missing = Vec::new();
        for ch in kana_chars().chain(kanji_chars_in(text)) {
            if cache_valid {
                if let Some(cached) = self.cached_kanjivg(ch) {
                    store.merge(cached);
                    continue;
                }
                if known_not_found.contains(&(ch as u32)) {
                    continue;
                }
            }
            missing.push(ch);
        }

        if missing.is_empty() {
            return Ok(());
        }
        if offline {
            if missing.iter().any(|ch| is_kana(*ch)) {
                progress.notice(&format!(
                    "warning: no usable cached kana stroke data in {} (offline mode)",
                    self.cache.root().display()
                ));
            }
            return Ok(());
        }

        let not_found = if cache_valid {
            known_not_found
        } else {
            BTreeSet::new()
        };
        self.fetch_kanjivg(&missing, cache_valid, not_found, progress, store)
    }

    fn cached_kanjivg(&self, ch: char) -> Option<GlyphStore> {
        GlyphStore::from_json_file(&self.cache.kanjivg_path(ch))
            .ok()
            .filter(|cached| cached.get(ch).is_some())
    }

    fn fetch_kanjivg(
        &self,
        missing: &[char],
        cache_valid: bool,
        mut not_found: BTreeSet<u32>,
        progress: &mut dyn Progress,
        store: &mut GlyphStore,
    ) -> Result<()> {
        let mut cache_error: Option<String> = None;

        if !cache_valid {
            note_failure(
                &mut cache_error,
                self.cache.clear_kanjivg().and_then(|()| {
                    self.update_manifest(|m| m.kanjivg = Some(current_kanjivg_entry()))
                }),
            );
        }

        let urls: Vec<String> = missing
            .iter()
            .map(|ch| kanjivg_url(KANJIVG_COMMIT, *ch))
            .collect();
        let total = missing.len() as u64;
        let mut handled = vec![false; missing.len()];
        let mut failures: Vec<(char, String)> = Vec::new();
        let mut fetched = 0usize;
        let mut done = 0u64;

        progress.begin(&format!("{KANJIVG_LABEL} ({total} characters)"));
        fetch_batch(
            &self.fetcher,
            &urls,
            DEFAULT_WORKERS,
            &mut |index, result| {
                handled[index] = true;
                let ch = missing[index];
                match result {
                    ItemResult::Fetched(bytes) => match convert_fetched(ch, &bytes) {
                        Ok(converted) => {
                            note_failure(
                                &mut cache_error,
                                converted
                                    .to_json_string()
                                    .and_then(|json| self.cache.write_kanjivg(ch, &json)),
                            );
                            store.merge(converted);
                            fetched += 1;
                        }
                        Err(message) => failures.push((ch, message)),
                    },
                    ItemResult::NotFound if is_kana(ch) => {
                        failures.push((ch, "not found at the pinned revision".to_string()));
                    }
                    ItemResult::NotFound => {
                        not_found.insert(ch as u32);
                    }
                    ItemResult::Failed(message) => failures.push((ch, message)),
                }
                done += 1;
                progress.update(done, Some(total));
            },
        );
        progress.finish();

        for (index, ch) in missing.iter().enumerate() {
            if !handled[index] {
                failures.push((*ch, SKIPPED_MESSAGE.to_string()));
            }
        }

        let entry = KanjivgEntry {
            not_found: not_found.into_iter().collect(),
            ..current_kanjivg_entry()
        };
        note_failure(
            &mut cache_error,
            self.update_manifest(|manifest| manifest.kanjivg = Some(entry)),
        );
        note_failure(&mut cache_error, self.write_licenses());

        if fetched > 0 {
            progress.notice(&format!(
                "Fetched {fetched} KanjiVG glyphs; cached in {}",
                self.cache.root().display()
            ));
        }
        if let Some(error) = cache_error {
            progress.notice(&format!(
                "warning: failed to cache the KanjiVG data in {}: {error}",
                self.cache.root().display()
            ));
        }

        report_failures(&failures, progress)
    }
}

fn convert_fetched(ch: char, bytes: &[u8]) -> std::result::Result<GlyphStore, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "the data is not valid UTF-8".to_string())?;
    let glyph = convert_svg(text)?;
    let mut store = GlyphStore::default();
    store.insert(ch, glyph).map_err(|error| error.to_string())?;
    Ok(store)
}

fn note_failure(slot: &mut Option<String>, result: Result<()>) {
    if let Err(error) = result {
        slot.get_or_insert_with(|| error.to_string());
    }
}

fn report_failures(failures: &[(char, String)], progress: &mut dyn Progress) -> Result<()> {
    if failures.is_empty() {
        return Ok(());
    }
    let reason = failures
        .iter()
        .map(|(_, message)| message.as_str())
        .find(|message| *message != SKIPPED_MESSAGE)
        .unwrap_or(SKIPPED_MESSAGE);

    let kana_count = failures.iter().filter(|(ch, _)| is_kana(*ch)).count();
    if kana_count > 0 {
        return Err(AutoQvError::Dataset(format!(
            "failed to fetch the kana stroke data ({kana_count} characters): {reason}"
        )));
    }
    progress.notice(&format!(
        "warning: failed to fetch the stroke data of {} kanji ({reason}); they are treated as missing",
        failures.len()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::dataset::cache::Cache;
    use crate::dataset::test_support::{
        FakeFetcher, RecordingProgress, SAMPLE_SVG, provider, seed_all, seed_hershey, seed_kanjivg,
    };

    fn serving_svg() -> FakeFetcher {
        FakeFetcher::serving(SAMPLE_SVG.as_bytes().to_vec())
    }

    fn manifest_of(dir: &tempfile::TempDir) -> crate::dataset::cache::Manifest {
        Cache::new(dir.path().to_path_buf())
            .read_manifest()
            .unwrap()
    }

    #[test]
    fn first_run_fetches_every_kana_and_the_kanji_in_the_text() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let fetcher = serving_svg();
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher)
            .load("漢字", false, &mut progress)
            .unwrap();

        assert_eq!(fetcher.call_count(), 184 + 2);
        assert!(store.get('あ').is_some());
        assert!(store.get('ー').is_some());
        assert!(store.get('漢').is_some());
        assert!(store.get('字').is_some());
        assert!(store.get('A').is_some());
        assert!(progress.events[0].starts_with("begin:Fetching KanjiVG stroke data (186"));
        assert!(
            progress
                .notices()
                .iter()
                .any(|notice| notice.starts_with("Fetched 186 KanjiVG glyphs"))
        );
    }

    #[test]
    fn fetched_data_is_cached_and_credited() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let fetcher = serving_svg();
        provider(&dir, &fetcher)
            .load("漢", false, &mut RecordingProgress::default())
            .unwrap();

        let second = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();
        let store = provider(&dir, &second)
            .load("漢", false, &mut progress)
            .unwrap();

        assert_eq!(second.call_count(), 0);
        assert!(progress.events.is_empty());
        assert!(store.get('漢').is_some());
        assert!(store.get('い').is_some());
        assert_eq!(manifest_of(&dir).kanjivg, Some(current_kanjivg_entry()));
        let licenses = fs::read_to_string(dir.path().join("LICENSES.txt")).unwrap();
        assert!(licenses.contains("Ulrich Apel"));
        assert!(licenses.contains("A. V. Hershey"));
    }

    #[test]
    fn requests_use_the_pinned_revision_and_code_point_names() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = serving_svg();

        provider(&dir, &fetcher)
            .load("漢", false, &mut RecordingProgress::default())
            .unwrap();

        assert_eq!(fetcher.requested(), vec![kanjivg_url(KANJIVG_COMMIT, '漢')]);
        assert!(fetcher.requested()[0].ends_with("/kanji/06f22.svg"));
    }

    #[test]
    fn only_uncached_kanji_are_requested() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        seed_kanjivg(&dir, ['漢']);
        let fetcher = serving_svg();

        provider(&dir, &fetcher)
            .load("漢字", false, &mut RecordingProgress::default())
            .unwrap();

        assert_eq!(fetcher.requested(), vec![kanjivg_url(KANJIVG_COMMIT, '字')]);
    }

    #[test]
    fn text_without_kanji_needs_only_the_kana() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::failing();

        let store = provider(&dir, &fetcher)
            .load("Hello あいう!", false, &mut RecordingProgress::default())
            .unwrap();

        assert_eq!(fetcher.call_count(), 0);
        assert!(store.get('あ').is_some());
    }

    #[test]
    fn kanji_missing_upstream_are_remembered_without_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::with(|url| {
            if url.ends_with("/06f22.svg") {
                Err(AutoQvError::NotFound(url.to_string()))
            } else {
                Ok(SAMPLE_SVG.as_bytes().to_vec())
            }
        });
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher)
            .load("漢字", false, &mut progress)
            .unwrap();

        assert!(store.get('漢').is_none());
        assert!(store.get('字').is_some());
        assert!(progress.notices().iter().all(|n| !n.starts_with("warning")));
        assert_eq!(manifest_of(&dir).kanjivg.unwrap().not_found, vec![0x6f22]);

        let second = FakeFetcher::failing();
        let again = provider(&dir, &second)
            .load("漢字", false, &mut RecordingProgress::default())
            .unwrap();
        assert_eq!(second.call_count(), 0);
        assert!(again.get('漢').is_none());
        assert!(again.get('字').is_some());
    }

    #[test]
    fn a_kanji_network_failure_warns_and_continues() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher)
            .load("漢", false, &mut progress)
            .unwrap();

        assert!(store.get('漢').is_none());
        assert!(store.get('あ').is_some());
        assert!(
            progress
                .notices()
                .iter()
                .any(|n| n.starts_with("warning: failed to fetch the stroke data of 1 kanji"))
        );
        assert_eq!(progress.events.iter().filter(|e| *e == "finish").count(), 1);
        assert!(manifest_of(&dir).kanjivg.unwrap().not_found.is_empty());
    }

    #[test]
    fn an_invalid_download_is_a_warning_and_is_not_cached() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::serving(b"<html>rate limited</html>".to_vec());
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher)
            .load("漢", false, &mut progress)
            .unwrap();

        assert!(store.get('漢').is_none());
        assert!(progress.notices().iter().any(|n| n.starts_with("warning")));
        assert!(
            !Cache::new(dir.path().to_path_buf())
                .kanjivg_path('漢')
                .exists()
        );
    }

    #[test]
    fn a_kana_failure_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let fetcher = FakeFetcher::failing();

        let error = provider(&dir, &fetcher)
            .load("", false, &mut RecordingProgress::default())
            .unwrap_err();

        assert!(error.to_string().contains("kana stroke data"));
        assert!(error.to_string().contains("network down"));
    }

    #[test]
    fn a_failure_stops_the_remaining_requests() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let fetcher = FakeFetcher::failing();

        let _ = provider(&dir, &fetcher).load("", false, &mut RecordingProgress::default());

        assert!(fetcher.call_count() <= DEFAULT_WORKERS);
    }

    #[test]
    fn a_missing_kana_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let fetcher = FakeFetcher::with(|url| {
            if url.ends_with("/03042.svg") {
                Err(AutoQvError::NotFound(url.to_string()))
            } else {
                Ok(SAMPLE_SVG.as_bytes().to_vec())
            }
        });

        let result = provider(&dir, &fetcher).load("", false, &mut RecordingProgress::default());

        assert!(result.is_err());
    }

    #[test]
    fn a_partially_fetched_kana_set_is_resumed_on_the_next_run() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let failing = FakeFetcher::with(|url| {
            if url.ends_with("/030a2.svg") {
                Err(AutoQvError::Dataset("boom".to_string()))
            } else {
                Ok(SAMPLE_SVG.as_bytes().to_vec())
            }
        });
        assert!(
            provider(&dir, &failing)
                .load("", false, &mut RecordingProgress::default())
                .is_err()
        );
        let cached_before = fs::read_dir(dir.path().join("kanjivg")).unwrap().count();
        assert!(cached_before > 0);

        let fetcher = serving_svg();
        let store = provider(&dir, &fetcher)
            .load("", false, &mut RecordingProgress::default())
            .unwrap();

        assert_eq!(fetcher.call_count(), 184 - cached_before);
        assert!(store.get('ア').is_some());
    }

    #[test]
    fn offline_never_fetches_and_uses_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher)
            .load("漢", true, &mut progress)
            .unwrap();

        assert_eq!(fetcher.call_count(), 0);
        assert!(store.get('あ').is_some());
        assert!(store.get('漢').is_none());
        assert!(progress.events.is_empty());
    }

    #[test]
    fn offline_without_kana_cache_warns() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let fetcher = FakeFetcher::failing();
        let mut progress = RecordingProgress::default();

        let store = provider(&dir, &fetcher)
            .load("", true, &mut progress)
            .unwrap();

        assert_eq!(fetcher.call_count(), 0);
        assert!(store.get('あ').is_none());
        assert!(progress.notices()[0].contains("kana stroke data"));
    }

    #[test]
    fn an_outdated_revision_discards_the_cached_files() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        seed_kanjivg(&dir, ['漢']);
        let cache = Cache::new(dir.path().to_path_buf());
        let mut manifest = cache.read_manifest().unwrap();
        manifest.kanjivg = Some(KanjivgEntry {
            commit: "old".to_string(),
            converter_version: KANJIVG_CONVERTER_VERSION,
            not_found: vec![0x5b57],
        });
        cache.write_manifest(&manifest).unwrap();
        let fetcher = serving_svg();

        let store = provider(&dir, &fetcher)
            .load("漢字", false, &mut RecordingProgress::default())
            .unwrap();

        assert_eq!(fetcher.call_count(), 184 + 2);
        assert!(store.get('字').is_some());
        assert_eq!(manifest_of(&dir).kanjivg, Some(current_kanjivg_entry()));
    }

    #[test]
    fn an_outdated_revision_is_ignored_when_offline() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let cache = Cache::new(dir.path().to_path_buf());
        let mut manifest = cache.read_manifest().unwrap();
        manifest.kanjivg.as_mut().unwrap().commit = "old".to_string();
        cache.write_manifest(&manifest).unwrap();
        let fetcher = FakeFetcher::failing();

        let store = provider(&dir, &fetcher)
            .load("", true, &mut RecordingProgress::default())
            .unwrap();

        assert!(store.get('あ').is_none());
        assert!(cache.kanjivg_path('あ').exists());
    }

    #[test]
    fn a_corrupt_cached_file_is_fetched_again() {
        let dir = tempfile::tempdir().unwrap();
        seed_all(&dir);
        let cache = Cache::new(dir.path().to_path_buf());
        fs::write(cache.kanjivg_path('あ'), "broken").unwrap();
        let fetcher = serving_svg();

        let store = provider(&dir, &fetcher)
            .load("", false, &mut RecordingProgress::default())
            .unwrap();

        assert_eq!(fetcher.requested(), vec![kanjivg_url(KANJIVG_COMMIT, 'あ')]);
        assert!(store.get('あ').is_some());
    }

    #[test]
    fn a_cache_write_failure_still_returns_the_fetched_data() {
        let dir = tempfile::tempdir().unwrap();
        seed_hershey(&dir);
        let cache = Cache::new(dir.path().to_path_buf());
        fs::write(dir.path().join("kanjivg"), "a file, not a directory").unwrap();
        let fetcher = serving_svg();
        let mut progress = RecordingProgress::default();

        let result = provider(&dir, &fetcher).load("漢", false, &mut progress);

        let store = result.unwrap();
        assert!(store.get('漢').is_some());
        assert!(
            progress
                .notices()
                .iter()
                .any(|n| n.starts_with("warning: failed to cache the KanjiVG data"))
        );
        assert!(!cache.kanjivg_path('漢').exists());
    }
}
