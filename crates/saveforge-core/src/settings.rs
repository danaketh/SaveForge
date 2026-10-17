//! `settings.json` (spec §5.3).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::backup::write_atomic;
use crate::paths::AppPaths;
use crate::{CoreError, Result};

/// The catalog the release workflow attaches to the newest GitHub release.
/// `YOUR_ORG/YOUR_REPO` is a placeholder until the project is published.
pub const RELEASE_CATALOG_URL: &str =
    "https://github.com/YOUR_ORG/YOUR_REPO/releases/latest/download/catalog.json";

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

    /// Never fails. A missing file means defaults. Each known key is read on its
    /// own, so a key with a wrong type falls back to its default without losing
    /// the others. Whenever the file is not fully usable it is copied to
    /// `<name>.bad` (so a later save cannot destroy it) and a warning is returned.
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
        let map = match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(map)) => map,
            Ok(_) => {
                return unusable(path, defaults, "Settings file is not a JSON object");
            }
            Err(e) => {
                return unusable(path, defaults, &format!("Settings file is invalid ({e})"));
            }
        };

        let d = defaults;
        let mut bad = Vec::new();
        let settings = Self {
            catalog_url: field(&map, "catalogUrl", d.catalog_url, &mut bad),
            plugin_directory: field(&map, "pluginDirectory", d.plugin_directory, &mut bad),
            backup_directory: field(&map, "backupDirectory", d.backup_directory, &mut bad),
            check_updates_on_startup: field(
                &map,
                "checkUpdatesOnStartup",
                d.check_updates_on_startup,
                &mut bad,
            ),
            plugin_save_directories: field(
                &map,
                "pluginSaveDirectories",
                d.plugin_save_directories,
                &mut bad,
            ),
        };
        if bad.is_empty() {
            return LoadedSettings {
                settings,
                warning: None,
            };
        }
        let reason = format!("Invalid settings ignored: {}", bad.join(", "));
        LoadedSettings {
            warning: Some(warning_with_copy(path, &reason)),
            settings,
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

/// Value of `key`, or `default` when missing; wrong types also yield `default`
/// and are recorded in `bad`.
fn field<T: DeserializeOwned>(
    map: &Map<String, Value>,
    key: &'static str,
    default: T,
    bad: &mut Vec<&'static str>,
) -> T {
    match map.get(key) {
        None => default,
        Some(value) => T::deserialize(value).unwrap_or_else(|_| {
            bad.push(key);
            default
        }),
    }
}

fn unusable(path: &Path, defaults: AppSettings, reason: &str) -> LoadedSettings {
    LoadedSettings {
        settings: defaults,
        warning: Some(warning_with_copy(path, reason)),
    }
}

/// Copies the unusable file to `<name>.bad` and builds the user-facing warning.
fn warning_with_copy(path: &Path, reason: &str) -> String {
    let file_name = path
        .file_name()
        .map_or_else(|| "settings.json".into(), |n| n.to_string_lossy());
    let bad_copy = path.with_file_name(format!("{file_name}.bad"));
    match std::fs::copy(path, &bad_copy) {
        Ok(_) => format!(
            "{reason}; using defaults. Original kept at {}.",
            bad_copy.display()
        ),
        Err(e) => {
            tracing::warn!(path = %bad_copy.display(), %e, "could not keep copy of bad settings");
            format!("{reason}; using defaults.")
        }
    }
}

/// `file://` URL for a local path (forward slashes, three slashes before a
/// drive letter, `%` escaped so [`path_from_file_url`] reverses it).
pub fn file_url(path: &Path) -> String {
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .replace('%', "%25");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

/// Inverse of [`file_url`]: the local path a `file://` URL names, with `%XX`
/// escapes decoded. `None` for any other scheme, and for URLs naming another
/// host or a UNC share (`file://host/…`, `file:////host/…`): opening those
/// would reach out over the network.
pub fn path_from_file_url(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let mut decoded = Vec::with_capacity(rest.len());
    let mut bytes = rest.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let pair = [bytes.next()?, bytes.next()?];
            let hex = std::str::from_utf8(&pair).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
        } else {
            decoded.push(b);
        }
    }
    let path = String::from_utf8(decoded).ok()?;
    let bytes = path.as_bytes();
    let path = if bytes.len() >= 3 && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        &path[1..]
    } else {
        &path
    };
    if path.starts_with(['/', '\\']) && path[1..].starts_with(['/', '\\']) {
        return None;
    }
    Some(PathBuf::from(path))
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
