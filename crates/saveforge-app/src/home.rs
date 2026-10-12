//! Home screen state: which plugin is selected and where its saves are
//! (spec §5.2 Home).

use std::path::PathBuf;

use saveforge_core::discovery::SaveFileInfo;
use saveforge_core::manifest::PluginManifest;
use saveforge_core::platform::Os;
use saveforge_core::settings::AppSettings;
use saveforge_core::tokens::{Expanded, TokenEnv, expand};

#[derive(Debug, Clone, PartialEq)]
pub enum Saves {
    /// No usable folder; the reason is shown to the user.
    NotConfigured(String),
    Scanning,
    Found(Vec<SaveFileInfo>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Home {
    pub selected: Option<String>,
    /// The raw (unexpanded) save folder for the selected plugin.
    pub raw_dir: Option<String>,
    pub saves: Saves,
    /// `Some(text)` while the path editor is open.
    pub editing: Option<String>,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            selected: None,
            raw_dir: None,
            saves: Saves::NotConfigured("No plugin selected".to_owned()),
            editing: None,
        }
    }
}

/// The user's override for the plugin, else the manifest default for `os`.
pub fn raw_save_dir(settings: &AppSettings, manifest: &PluginManifest, os: Os) -> Option<String> {
    settings
        .plugin_save_directories
        .get(&manifest.id)
        .cloned()
        .or_else(|| {
            manifest
                .default_save_directories
                .for_os(os)
                .map(str::to_owned)
        })
}

/// An existing directory for `raw`, or the reason it can't be used.
pub fn resolve_save_dir(raw: Option<&str>, env: &TokenEnv) -> Result<PathBuf, String> {
    let raw = raw
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .ok_or_else(|| "No save folder set".to_owned())?;
    match expand(raw, env) {
        Expanded::Unresolved(reason) => Err(reason),
        Expanded::Resolved(path) if path.is_dir() => Ok(path),
        Expanded::Resolved(path) => Err(format!("{} does not exist", path.display())),
    }
}
