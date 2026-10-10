//! Layout of the app data directory.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

impl AppPaths {
    /// Platform data dir for `SaveForge`, or `None` if no home directory is known.
    pub fn detect() -> Option<Self> {
        directories::ProjectDirs::from("", "", "SaveForge").map(|d| Self {
            data_dir: d.data_dir().to_path_buf(),
        })
    }

    pub fn under(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    pub fn catalog_cache_file(&self) -> PathBuf {
        self.data_dir.join("catalog-cache.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn crash_log(&self) -> PathBuf {
        self.data_dir.join("crash.log")
    }
}
