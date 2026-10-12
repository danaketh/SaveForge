//! Blocking document operations. The app runs these on worker threads
//! (`spawn_blocking`) so the UI never waits on file I/O or plugin parsing
//! (spec §3.4, §5.2 Open/Save).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use saveforge_core::backup::save_with_backup;
use saveforge_core::tokens::TokenEnv;
use saveforge_host::{
    DocumentSession, HEADER_LEN, HostError, LoadedPlugin, PluginEngine, PluginRegistry, UiTree,
    detect, resolve_aux_files,
};

/// A failed operation, as shown to the user. `plugin_fault` marks a crash or
/// timeout that counts toward disabling the plugin (spec §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    pub plugin_fault: bool,
}

impl Failure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            plugin_fault: false,
        }
    }

    pub fn from_host(error: &HostError) -> Self {
        Self {
            message: error.to_string(),
            plugin_fault: matches!(error, HostError::Fault(_)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DetectOutcome {
    Open(Arc<LoadedPlugin>),
    /// Several plugins share the top priority; the user picks one.
    Choose(Vec<Arc<LoadedPlugin>>),
    NoMatch,
}

#[derive(Debug, Clone)]
pub struct Detected {
    pub outcome: DetectOutcome,
    /// Ids of plugins whose `detect` crashed or timed out.
    pub faulted: Vec<String>,
}

/// Reads the file's header and asks the matching plugins (spec §5.2 Open).
pub fn detect_file(
    engine: &PluginEngine,
    registry: &PluginRegistry,
    path: &Path,
) -> Result<Detected, Failure> {
    let mut header = Vec::with_capacity(HEADER_LEN);
    File::open(path)
        .and_then(|f| f.take(HEADER_LEN as u64).read_to_end(&mut header))
        .map_err(|e| Failure::new(format!("Could not read {}: {e}", path.display())))?;
    let report = detect(engine, registry, path, &header);
    let faulted = report
        .faults
        .iter()
        .filter(|(_, e)| matches!(e, HostError::Fault(_)))
        .map(|(id, _)| id.clone())
        .collect();
    let outcome = match report.detection {
        saveforge_host::Detection::NoMatch => DetectOutcome::NoMatch,
        saveforge_host::Detection::Winner(plugin) => DetectOutcome::Open(plugin),
        saveforge_host::Detection::Tie(plugins) => DetectOutcome::Choose(plugins),
    };
    Ok(Detected { outcome, faulted })
}

/// A document opened by a plugin, with its first UI tree.
#[derive(Debug, Clone)]
pub struct OpenedDocument {
    pub session: Arc<Mutex<DocumentSession>>,
    pub plugin_id: String,
    pub path: PathBuf,
    pub tree: UiTree,
}

/// Reads the file and its aux files and opens it in `plugin`.
pub fn open_document(
    engine: &PluginEngine,
    plugin: &LoadedPlugin,
    path: &Path,
    tokens: &TokenEnv,
) -> Result<OpenedDocument, Failure> {
    let bytes = std::fs::read(path)
        .map_err(|e| Failure::new(format!("Could not read {}: {e}", path.display())))?;
    let aux = resolve_aux_files(&plugin.manifest, tokens).map_err(|e| Failure::from_host(&e))?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut session = engine
        .open(plugin, &file_name, &bytes, &aux)
        .map_err(|e| Failure::from_host(&e))?;
    let tree = session.view().map_err(|e| Failure::from_host(&e))?;
    Ok(OpenedDocument {
        session: Arc::new(Mutex::new(session)),
        plugin_id: plugin.manifest.id.clone(),
        path: path.to_path_buf(),
        tree,
    })
}

/// Serializes, backs up the original, writes atomically, then tells the
/// plugin the bytes are on disk (spec §5.2 Save). Returns the backup path.
pub fn save_document(
    session: &Mutex<DocumentSession>,
    path: &Path,
    backup_root: &Path,
) -> Result<PathBuf, Failure> {
    let mut session = lock(session);
    let bytes = session.serialize().map_err(|e| Failure::from_host(&e))?;
    let backup = save_with_backup(path, &bytes, backup_root, session.plugin_id())
        .map_err(|e| Failure::new(format!("Save failed: {e}")))?;
    session.mark_saved().map_err(|e| Failure::from_host(&e))?;
    Ok(backup)
}

/// Locks a session; a poisoned lock (a panic while held) still yields the
/// session, whose own poisoning state is authoritative.
pub fn lock(session: &Mutex<DocumentSession>) -> MutexGuard<'_, DocumentSession> {
    session.lock().unwrap_or_else(PoisonError::into_inner)
}
