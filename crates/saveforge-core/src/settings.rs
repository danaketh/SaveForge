//! `settings.json` (spec §5.3).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backup::write_atomic;
use crate::paths::AppPaths;
use crate::{CoreError, Result};

pub const RELEASE_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/YOUR_ORG/YOUR_REPO/main/catalog.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub catalog_url: String,
    pub plugin_directory: PathBuf,
    pub backup_directory: PathBuf,
    pub check_updates_on_startup: bool,
    /// Plugin id -> raw (unexpanded) save directory chosen by the user.
    pub plugin_save_directories: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct LoadedSettings {
    pub settings: AppSettings,
    /// Set when the file existed but could not be used; shown in the status bar.
    pub warning: Option<String>,
}

/// On-disk shape: every key optional so older/partial files merge over defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StoredSettings {
    catalog_url: Option<String>,
    plugin_directory: Option<PathBuf>,
    backup_directory: Option<PathBuf>,
    check_updates_on_startup: Option<bool>,
    plugin_save_directories: Option<BTreeMap<String, String>>,
}

impl StoredSettings {
    fn apply_to(self, d: AppSettings) -> AppSettings {
        AppSettings {
            catalog_url: self.catalog_url.unwrap_or(d.catalog_url),
            plugin_directory: self.plugin_directory.unwrap_or(d.plugin_directory),
            backup_directory: self.backup_directory.unwrap_or(d.backup_directory),
            check_updates_on_startup: self
                .check_updates_on_startup
                .unwrap_or(d.check_updates_on_startup),
            plugin_save_directories: self
                .plugin_save_directories
                .unwrap_or(d.plugin_save_directories),
        }
    }
}

impl AppSettings {
    pub fn defaults(paths: &AppPaths) -> Self {
        Self {
            catalog_url: default_catalog_url(paths),
            plugin_directory: default_plugin_directory(paths),
            backup_directory: paths.data_dir.join("backups"),
            check_updates_on_startup: true,
            plugin_save_directories: BTreeMap::new(),
        }
    }

    /// Never fails: missing file means defaults; unreadable or invalid file means
    /// defaults plus a warning.
    pub fn load(path: &Path, defaults: AppSettings) -> LoadedSettings {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return LoadedSettings {
                    settings: defaults,
                    warning: None,
                };
            }
            Err(e) => {
                return LoadedSettings {
                    settings: defaults,
                    warning: Some(format!("Could not read settings ({e}); using defaults.")),
                };
            }
        };
        match serde_json::from_str::<StoredSettings>(&text) {
            Ok(stored) => LoadedSettings {
                settings: stored.apply_to(defaults),
                warning: None,
            },
            Err(e) => LoadedSettings {
                settings: defaults,
                warning: Some(format!("Settings file is invalid ({e}); using defaults.")),
            },
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(|source| CoreError::Json {
            path: path.to_path_buf(),
            source,
        })?;
        write_atomic(path, &json)
    }
}

/// `file://` URL for a local path (forward slashes, three slashes before a drive letter).
pub fn file_url(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

fn default_catalog_url(paths: &AppPaths) -> String {
    if cfg!(debug_assertions) {
        file_url(&paths.data_dir.join("catalog.json"))
    } else {
        RELEASE_CATALOG_URL.to_owned()
    }
}

fn default_plugin_directory(paths: &AppPaths) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dev-plugins")
    } else {
        paths.data_dir.join("plugins")
    }
}
