use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::cache::{Cache, Manifest};
use super::charset::kana_chars;
use super::fetch::Fetcher;
use super::hershey::convert_rowmans;
use super::kanjivg_source::current_kanjivg_entry;
use super::progress::Progress;
use super::provider::DatasetProvider;
use crate::error::{AutoQvError, Result};
use crate::glyph::model::{Glyph, Point};
use crate::glyph::store::GlyphStore;

type Handler = Box<dyn Fn(&str) -> Result<Vec<u8>> + Send + Sync>;

pub struct FakeFetcher {
    handler: Handler,
    calls: AtomicUsize,
    urls: Mutex<Vec<String>>,
}

impl FakeFetcher {
    pub fn with(handler: impl Fn(&str) -> Result<Vec<u8>> + Send + Sync + 'static) -> Self {
        Self {
            handler: Box::new(handler),
            calls: AtomicUsize::new(0),
            urls: Mutex::new(Vec::new()),
        }
    }

    pub fn serving(body: Vec<u8>) -> Self {
        Self::with(move |_| Ok(body.clone()))
    }

    pub fn failing() -> Self {
        Self::with(|_| Err(AutoQvError::Dataset("network down".to_string())))
    }

    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn requested(&self) -> Vec<String> {
        self.urls.lock().unwrap().clone()
    }
}

impl Fetcher for &FakeFetcher {
    fn get(&self, url: &str, on_progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.urls.lock().unwrap().push(url.to_string());
        let body = (self.handler)(url)?;
        on_progress(body.len() as u64, Some(body.len() as u64));
        Ok(body)
    }
}

#[derive(Default)]
pub struct RecordingProgress {
    pub events: Vec<String>,
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

impl RecordingProgress {
    pub fn notices(&self) -> Vec<&str> {
        self.events
            .iter()
            .filter_map(|event| event.strip_prefix("notice:"))
            .collect()
    }
}

pub fn rowmans_text() -> Vec<u8> {
    vec!["  699  1JZ"; 95].join("\n").into_bytes()
}

pub fn hangul_font_text() -> Vec<u8> {
    let glyph = |code: u32| {
        format!(
            r#"<glyph unicode="&#x{code:X};" horiz-adv-x="1000" d="M100 100 L900 100 L900 900" />"#
        )
    };
    let glyphs = [0x3131, 0xac00, 0xd55c, 0xae00].map(glyph).concat();
    format!(r#"<svg><font-face units-per-em="1000" />{glyphs}</svg>"#).into_bytes()
}

pub fn provider<'a>(
    dir: &tempfile::TempDir,
    fetcher: &'a FakeFetcher,
) -> DatasetProvider<&'a FakeFetcher> {
    DatasetProvider::new(Cache::new(dir.path().to_path_buf()), fetcher)
}

pub fn seed_hershey(dir: &tempfile::TempDir) {
    let cache = Cache::new(dir.path().to_path_buf());
    let store = convert_rowmans(&String::from_utf8(rowmans_text()).unwrap()).unwrap();
    let fetcher = FakeFetcher::failing();
    let provider = DatasetProvider::new(cache, &fetcher);
    provider.persist_hershey(&store).unwrap();
}

pub fn seed_kanjivg(dir: &tempfile::TempDir, chars: impl IntoIterator<Item = char>) {
    let cache = Cache::new(dir.path().to_path_buf());
    for ch in chars {
        cache.write_kanjivg(ch, &glyph_json(ch)).unwrap();
    }
    let mut manifest = cache
        .read_manifest()
        .unwrap_or_else(|| Manifest::new("seed".to_string(), None));
    manifest.kanjivg = Some(current_kanjivg_entry());
    cache.write_manifest(&manifest).unwrap();
    let fetcher = FakeFetcher::failing();
    DatasetProvider::new(cache, &fetcher)
        .write_licenses()
        .unwrap();
}

pub fn seed_all(dir: &tempfile::TempDir) {
    seed_hershey(dir);
    seed_kanjivg(dir, kana_chars());
}

pub fn glyph_json(ch: char) -> String {
    let mut store = GlyphStore::default();
    store
        .insert(
            ch,
            Glyph {
                advance: 1.0,
                strokes: vec![vec![Point { x: 0.1, y: 0.1 }, Point { x: 0.9, y: 0.9 }]],
            },
        )
        .unwrap();
    store.to_json_string().unwrap()
}

pub const SAMPLE_SVG: &str = r#"<svg viewBox="0 0 109 109"><g id="kvg:StrokePaths_x"><path id="kvg:x-s1" d="M10,10c10,0,20,0,30,0"/><path id="kvg:x-s2" d="M50,20L50,90"/></g></svg>"#;
