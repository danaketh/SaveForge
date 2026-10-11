//! Installed plugins: `{plugins}/{id}/{version}/` scanning and session fault
//! tracking (spec §5.2 Startup, §7).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use semver::Version;

use crate::engine::{LoadedPlugin, PluginEngine};

/// Faults in one session after which a plugin is disabled until restart.
pub const MAX_FAULTS_PER_SESSION: u8 = 3;

#[derive(Debug, Clone)]
pub enum PluginEntry {
    Loaded(Arc<LoadedPlugin>),
    Failed {
        id: String,
        dir: PathBuf,
        reason: String,
    },
}

impl PluginEntry {
    pub fn id(&self) -> &str {
        match self {
            Self::Loaded(plugin) => &plugin.manifest.id,
            Self::Failed { id, .. } => id,
        }
    }
}

#[derive(Debug, Default)]
pub struct PluginRegistry {
    entries: Vec<PluginEntry>,
    faults: HashMap<String, u8>,
}

impl PluginRegistry {
    /// Loads the newest version of every plugin under `plugins_dir`. Never
    /// fails: problems become [`PluginEntry::Failed`] with a reason.
    pub fn scan(engine: &PluginEngine, plugins_dir: &Path) -> Self {
        let Ok(id_dirs) = fs::read_dir(plugins_dir) else {
            return Self::default();
        };
        let mut entries = Vec::new();
        for id_dir in id_dirs.flatten() {
            let id = id_dir.file_name().to_string_lossy().into_owned();
            let path = id_dir.path();
            if id.starts_with('.') || !path.is_dir() {
                continue;
            }
            entries.push(load_newest(engine, id, &path));
        }
        entries.sort_by(|a, b| a.id().cmp(b.id()));
        Self {
            entries,
            faults: HashMap::new(),
        }
    }

    pub fn entries(&self) -> &[PluginEntry] {
        &self.entries
    }

    pub fn loaded(&self) -> impl Iterator<Item = &Arc<LoadedPlugin>> {
        self.entries.iter().filter_map(|entry| match entry {
            PluginEntry::Loaded(plugin) => Some(plugin),
            PluginEntry::Failed { .. } => None,
        })
    }

    pub fn get(&self, id: &str) -> Option<&PluginEntry> {
        self.entries.iter().find(|entry| entry.id() == id)
    }

    /// Counts a fault. Returns `true` when this fault disabled the plugin.
    pub fn record_fault(&mut self, id: &str) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id() == id) else {
            return false;
        };
        let PluginEntry::Loaded(plugin) = entry else {
            return false;
        };
        let count = self.faults.entry(id.to_owned()).or_insert(0);
        *count = count.saturating_add(1);
        if *count < MAX_FAULTS_PER_SESSION {
            return false;
        }
        let dir = plugin.dir.clone();
        *entry = PluginEntry::Failed {
            id: id.to_owned(),
            dir,
            reason: format!("disabled after {MAX_FAULTS_PER_SESSION} crashes this session"),
        };
        true
    }
}

fn load_newest(engine: &PluginEngine, id: String, id_dir: &Path) -> PluginEntry {
    let Some((version, dir)) = newest_version(id_dir) else {
        return PluginEntry::Failed {
            id,
            dir: id_dir.to_path_buf(),
            reason: "no installed version".to_owned(),
        };
    };
    let reason = match engine.load(&dir) {
        Ok(plugin) if plugin.manifest.id != id => format!(
            "manifest id '{}' does not match folder '{id}'",
            plugin.manifest.id
        ),
        Ok(plugin) if plugin.manifest.version != version => format!(
            "manifest version {} does not match folder {version}",
            plugin.manifest.version
        ),
        Ok(plugin) => return PluginEntry::Loaded(Arc::new(plugin)),
        Err(e) => e.to_string(),
    };
    tracing::warn!(plugin = %id, %reason, "plugin failed to load");
    PluginEntry::Failed { id, dir, reason }
}

fn newest_version(id_dir: &Path) -> Option<(Version, PathBuf)> {
    fs::read_dir(id_dir)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let version = Version::parse(&entry.file_name().to_string_lossy()).ok()?;
            Some((version, entry.path()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
}
