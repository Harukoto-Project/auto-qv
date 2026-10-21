use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::{AutoQvError, Result};

const VENDOR_DIR: &str = "Harukoto Project";
const APP_DIR: &str = "AutoQV";

pub fn app_data_dir() -> Result<PathBuf> {
    app_data_dir_from(std::env::var_os("APPDATA"))
}

fn app_data_dir_from(appdata: Option<OsString>) -> Result<PathBuf> {
    let base = appdata.filter(|value| !value.is_empty()).ok_or_else(|| {
        AutoQvError::Config("the APPDATA environment variable is not set".to_string())
    })?;
    Ok(PathBuf::from(base).join(VENDOR_DIR).join(APP_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_vendor_and_app_directories() {
        let dir = app_data_dir_from(Some(OsString::from(r"C:\Users\x\AppData\Roaming"))).unwrap();

        assert_eq!(
            dir,
            PathBuf::from(r"C:\Users\x\AppData\Roaming")
                .join("Harukoto Project")
                .join("AutoQV")
        );
    }

    #[test]
    fn missing_or_empty_appdata_is_an_error() {
        assert!(app_data_dir_from(None).is_err());
        assert!(app_data_dir_from(Some(OsString::new())).is_err());
    }
}
