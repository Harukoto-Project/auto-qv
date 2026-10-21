use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::paths::app_data_dir;
use crate::error::{AutoQvError, Result};

const MANIFEST_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "manifest.json";
const LICENSES_FILE: &str = "LICENSES.txt";
const HERSHEY_DIR: &str = "hershey";
const HERSHEY_FILE: &str = "rowmans.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HersheyEntry {
    pub commit: String,
    pub converter_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub fetched_at: String,
    #[serde(default)]
    pub hershey: Option<HersheyEntry>,
}

impl Manifest {
    pub fn new(fetched_at: String, hershey: Option<HersheyEntry>) -> Self {
        Self {
            version: MANIFEST_VERSION,
            fetched_at,
            hershey,
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
        let manifest = Manifest::new("2026-10-22T00:00:00Z".to_string(), Some(entry()));

        cache.write_manifest(&manifest).unwrap();

        assert_eq!(cache.read_manifest(), Some(manifest));
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
}
