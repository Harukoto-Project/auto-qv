use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::paths::app_data_dir;
use crate::error::{AutoQvError, Result};

const MANIFEST_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "manifest.json";
const LICENSES_FILE: &str = "LICENSES.txt";
const HERSHEY_DIR: &str = "hershey";
const HERSHEY_FILE: &str = "rowmans.json";
const KANJIVG_DIR: &str = "kanjivg";
const HANGUL_DIR: &str = "hangul";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HersheyEntry {
    pub commit: String,
    pub converter_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanjivgEntry {
    pub commit: String,
    pub converter_version: u32,
    #[serde(default)]
    pub not_found: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HangulEntry {
    pub commit: String,
    pub converter_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub fetched_at: String,
    #[serde(default)]
    pub hershey: Option<HersheyEntry>,
    #[serde(default)]
    pub kanjivg: Option<KanjivgEntry>,
    #[serde(default)]
    pub hangul: Option<HangulEntry>,
}

impl Manifest {
    pub fn new(fetched_at: String, hershey: Option<HersheyEntry>) -> Self {
        Self {
            version: MANIFEST_VERSION,
            fetched_at,
            hershey,
            kanjivg: None,
            hangul: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn at_default_location() -> Result<Self> {
        Ok(Self::new(app_data_dir()?))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn hershey_path(&self) -> PathBuf {
        self.root.join(HERSHEY_DIR).join(HERSHEY_FILE)
    }

    pub fn kanjivg_path(&self, ch: char) -> PathBuf {
        self.root
            .join(KANJIVG_DIR)
            .join(format!("{:05x}.json", ch as u32))
    }

    pub fn hangul_path(&self, ch: char) -> PathBuf {
        self.root
            .join(HANGUL_DIR)
            .join(format!("{:04x}.json", ch as u32))
    }

    pub fn read_manifest(&self) -> Option<Manifest> {
        let text = fs::read_to_string(self.root.join(MANIFEST_FILE)).ok()?;
        let manifest: Manifest = serde_json::from_str(&text).ok()?;
        (manifest.version == MANIFEST_VERSION).then_some(manifest)
    }

    pub fn write_manifest(&self, manifest: &Manifest) -> Result<()> {
        let text = serde_json::to_string_pretty(manifest)
            .map_err(|e| AutoQvError::Dataset(e.to_string()))?;
        self.write_atomic(&self.root.join(MANIFEST_FILE), text.as_bytes())
    }

    pub fn write_hershey(&self, json: &str) -> Result<()> {
        self.write_atomic(&self.hershey_path(), json.as_bytes())
    }

    pub fn write_kanjivg(&self, ch: char, json: &str) -> Result<()> {
        self.write_atomic(&self.kanjivg_path(ch), json.as_bytes())
    }

    pub fn write_hangul(&self, ch: char, json: &str) -> Result<()> {
        self.write_atomic(&self.hangul_path(ch), json.as_bytes())
    }

    pub fn clear_kanjivg(&self) -> Result<()> {
        self.remove_dir(KANJIVG_DIR)
    }

    pub fn clear_hangul(&self) -> Result<()> {
        self.remove_dir(HANGUL_DIR)
    }

    fn remove_dir(&self, name: &str) -> Result<()> {
        match fs::remove_dir_all(self.root.join(name)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn write_licenses(&self, text: &str) -> Result<()> {
        self.write_atomic(&self.root.join(LICENSES_FILE), text.as_bytes())
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("tmp");
        fs::write(&temp, bytes)?;
        fs::rename(&temp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> HersheyEntry {
        HersheyEntry {
            commit: "abc".to_string(),
            converter_version: 1,
        }
    }

    #[test]
    fn manifest_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().join("nested").join("cache"));
        let mut manifest = Manifest::new("2026-10-22T00:00:00Z".to_string(), Some(entry()));
        manifest.kanjivg = Some(KanjivgEntry {
            commit: "def".to_string(),
            converter_version: 2,
            not_found: vec![0x20bb7, 0x4e00],
        });

        cache.write_manifest(&manifest).unwrap();

        assert_eq!(cache.read_manifest(), Some(manifest));
    }

    #[test]
    fn manifest_written_before_kanjivg_support_still_reads() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(MANIFEST_FILE),
            r#"{ "version": 1, "fetched_at": "x", "hershey": { "commit": "abc", "converter_version": 1 } }"#,
        )
        .unwrap();

        let manifest = Cache::new(dir.path().to_path_buf())
            .read_manifest()
            .unwrap();

        assert_eq!(manifest.hershey, Some(entry()));
        assert_eq!(manifest.kanjivg, None);
    }

    #[test]
    fn missing_manifest_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(Cache::new(dir.path().to_path_buf()).read_manifest(), None);
    }

    #[test]
    fn corrupt_manifest_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MANIFEST_FILE), "{ not json").unwrap();

        assert_eq!(Cache::new(dir.path().to_path_buf()).read_manifest(), None);
    }

    #[test]
    fn manifest_with_another_schema_version_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(MANIFEST_FILE),
            r#"{ "version": 99, "fetched_at": "x" }"#,
        )
        .unwrap();

        assert_eq!(Cache::new(dir.path().to_path_buf()).read_manifest(), None);
    }

    #[test]
    fn writing_replaces_existing_files_without_leaving_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().to_path_buf());

        cache.write_hershey("first").unwrap();
        cache.write_hershey("second").unwrap();

        assert_eq!(fs::read_to_string(cache.hershey_path()).unwrap(), "second");
        let leftovers = fs::read_dir(cache.hershey_path().parent().unwrap())
            .unwrap()
            .count();
        assert_eq!(leftovers, 1);
    }

    #[test]
    fn kanjivg_files_are_named_by_code_point() {
        let cache = Cache::new(PathBuf::from("root"));

        assert!(cache.kanjivg_path('あ').ends_with("kanjivg/03042.json"));
        assert!(cache.kanjivg_path('𠮷').ends_with("20bb7.json"));
    }

    #[test]
    fn hangul_files_are_named_by_code_point() {
        let cache = Cache::new(PathBuf::from("root"));

        assert!(cache.hangul_path('가').ends_with("hangul/ac00.json"));
        assert!(cache.hangul_path('ㄱ').ends_with("hangul/3131.json"));
    }

    #[test]
    fn manifest_with_a_hangul_entry_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().to_path_buf());
        let mut manifest = Manifest::new("x".to_string(), None);
        manifest.hangul = Some(HangulEntry {
            commit: "abc".to_string(),
            converter_version: 1,
        });

        cache.write_manifest(&manifest).unwrap();

        assert_eq!(cache.read_manifest(), Some(manifest));
    }

    #[test]
    fn clearing_hangul_leaves_the_other_data_alone() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().to_path_buf());
        cache.write_kanjivg('あ', "k").unwrap();
        cache.write_hangul('가', "h").unwrap();

        cache.clear_hangul().unwrap();

        assert!(!cache.hangul_path('가').exists());
        assert!(cache.kanjivg_path('あ').exists());
        assert!(cache.clear_hangul().is_ok());
    }

    #[test]
    fn clearing_removes_only_the_kanjivg_directory() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().to_path_buf());
        cache.write_hershey("h").unwrap();
        cache.write_kanjivg('あ', "k").unwrap();

        cache.clear_kanjivg().unwrap();

        assert!(!cache.kanjivg_path('あ').exists());
        assert!(cache.hershey_path().exists());
    }

    #[test]
    fn clearing_without_a_directory_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();

        assert!(Cache::new(dir.path().to_path_buf()).clear_kanjivg().is_ok());
    }
}
