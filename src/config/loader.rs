use std::fs;
use std::path::{Path, PathBuf};

use super::file::ConfigFile;
use super::merge::settings_from_file;
use super::model::Settings;
use crate::error::{AutoQvError, Result};

const DEFAULT_CONFIG_FILE_NAME: &str = "autoqv.toml";

pub fn default_config_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(DEFAULT_CONFIG_FILE_NAME))
}

pub fn load_settings(explicit: Option<&Path>) -> Result<Settings> {
    match explicit {
        Some(path) => load_from_path(path),
        None => match default_config_path() {
            Some(path) if path.is_file() => load_from_path(&path),
            _ => Ok(Settings::default()),
        },
    }
}

pub fn load_from_path(path: &Path) -> Result<Settings> {
    let text = fs::read_to_string(path)
        .map_err(|e| AutoQvError::Config(format!("failed to read {}: {e}", path.display())))?;
    parse_settings(&text).map_err(|e| match e {
        AutoQvError::Config(message) => {
            AutoQvError::Config(format!("{}: {message}", path.display()))
        }
        other => other,
    })
}

pub fn parse_settings(text: &str) -> Result<Settings> {
    let file: ConfigFile = toml::from_str(text).map_err(|e| AutoQvError::Config(e.to_string()))?;
    settings_from_file(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_settings_from_text() {
        let settings = parse_settings("[layout]\nsize = 2.0\n").unwrap();

        assert_eq!(settings.layout.size, 2.0);
    }

    #[test]
    fn reports_toml_syntax_errors() {
        assert!(parse_settings("[layout\nsize = 2.0").is_err());
    }

    #[test]
    fn loads_an_explicit_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.toml");
        fs::write(&path, "[pen]\nstep_delay_ms = 5\n").unwrap();

        let settings = load_settings(Some(&path)).unwrap();

        assert_eq!(settings.timing.step_delay_ms, 5);
    }

    #[test]
    fn missing_explicit_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();

        let error = load_settings(Some(&dir.path().join("nope.toml"))).unwrap_err();

        assert!(error.to_string().contains("nope.toml"));
    }

    #[test]
    fn invalid_file_error_mentions_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        fs::write(&path, "[pen]\nstep_delay_ms = 0\n").unwrap();

        let error = load_settings(Some(&path)).unwrap_err();

        assert!(error.to_string().contains("bad.toml"));
        assert!(error.to_string().contains("pen.step_delay_ms"));
    }
}
