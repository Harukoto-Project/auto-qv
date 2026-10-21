use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use super::fetch::Fetcher;
use crate::error::AutoQvError;

pub const DEFAULT_WORKERS: usize = 4;

#[derive(Debug, PartialEq, Eq)]
pub enum ItemResult {
    Fetched(Vec<u8>),
    NotFound,
    Failed(String),
}

pub fn fetch_batch<F: Fetcher>(
    fetcher: &F,
    urls: &[String],
    workers: usize,
    on_result: &mut dyn FnMut(usize, ItemResult),
) {
    if urls.is_empty() {
        return;
    }
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        for _ in 0..workers.clamp(1, urls.len()) {
            let sender = sender.clone();
            let (next, stop) = (&next, &stop);
            scope.spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(url) = urls.get(index) else {
                        break;
                    };
                    let result = match fetcher.get(url, &mut |_, _| {}) {
                        Ok(bytes) => ItemResult::Fetched(bytes),
                        Err(AutoQvError::NotFound(_)) => ItemResult::NotFound,
                        Err(error) => {
                            stop.store(true, Ordering::Relaxed);
                            ItemResult::Failed(error.to_string())
                        }
                    };
                    if sender.send((index, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);

        for (index, result) in receiver {
            on_result(index, result);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::error::Result;

    struct ScriptedFetcher {
        calls: AtomicUsize,
        fail_on: Option<&'static str>,
        missing: Option<&'static str>,
    }

    impl ScriptedFetcher {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                fail_on: None,
                missing: None,
            }
        }
    }

    impl Fetcher for ScriptedFetcher {
        fn get(
            &self,
            url: &str,
            _on_progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_on == Some(url) {
                return Err(AutoQvError::Dataset("boom".to_string()));
            }
            if self.missing == Some(url) {
                return Err(AutoQvError::NotFound(url.to_string()));
            }
            Ok(url.as_bytes().to_vec())
        }
    }

    fn urls(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("u{index}")).collect()
    }

    fn collect(
        fetcher: &ScriptedFetcher,
        urls: &[String],
        workers: usize,
    ) -> BTreeMap<usize, ItemResult> {
        let mut results = BTreeMap::new();
        fetch_batch(fetcher, urls, workers, &mut |index, result| {
            results.insert(index, result);
        });
        results
    }

    #[test]
    fn every_url_is_fetched_exactly_once() {
        let fetcher = ScriptedFetcher::new();
        let urls = urls(25);

        let results = collect(&fetcher, &urls, 4);

        assert_eq!(results.len(), 25);
        assert_eq!(fetcher.calls.load(Ordering::SeqCst), 25);
        for (index, result) in &results {
            assert_eq!(
                *result,
                ItemResult::Fetched(urls[*index].as_bytes().to_vec())
            );
        }
    }

    #[test]
    fn not_found_is_reported_without_stopping() {
        let fetcher = ScriptedFetcher {
            missing: Some("u1"),
            ..ScriptedFetcher::new()
        };

        let results = collect(&fetcher, &urls(6), 2);

        assert_eq!(results.len(), 6);
        assert_eq!(results[&1], ItemResult::NotFound);
    }

    #[test]
    fn a_failure_stops_further_requests() {
        let fetcher = ScriptedFetcher {
            fail_on: Some("u0"),
            ..ScriptedFetcher::new()
        };

        let results = collect(&fetcher, &urls(200), 1);

        assert_eq!(results.len(), 1);
        assert!(matches!(results[&0], ItemResult::Failed(_)));
        assert_eq!(fetcher.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn empty_input_does_nothing() {
        let fetcher = ScriptedFetcher::new();

        assert!(collect(&fetcher, &[], 4).is_empty());
        assert_eq!(fetcher.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn worker_count_is_bounded_by_the_work() {
        let fetcher = ScriptedFetcher::new();

        assert_eq!(collect(&fetcher, &urls(2), 100).len(), 2);
    }
}
