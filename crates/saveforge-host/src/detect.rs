//! Which plugin opens a file (spec §5.2 Open).

use std::path::Path;
use std::sync::Arc;

use crate::engine::{LoadedPlugin, PluginEngine};
use crate::error::HostError;
use crate::registry::PluginRegistry;

/// Plugins see at most this many leading bytes of the file.
pub const HEADER_LEN: usize = 512;

#[derive(Debug, Clone)]
pub enum Detection {
    NoMatch,
    Winner(Arc<LoadedPlugin>),
    /// Several plugins share the highest priority; the user must pick.
    Tie(Vec<Arc<LoadedPlugin>>),
}

#[derive(Debug)]
pub struct DetectReport {
    pub detection: Detection,
    /// Plugins whose `detect` failed (id, error). Feed these to
    /// `PluginRegistry::record_fault`.
    pub faults: Vec<(String, HostError)>,
}

/// Asks every loaded plugin that handles the file's extension. Highest
/// priority wins; equal top priorities are a tie (in registry order).
pub fn detect(
    engine: &PluginEngine,
    registry: &PluginRegistry,
    path: &Path,
    header: &[u8],
) -> DetectReport {
    let header = &header[..header.len().min(HEADER_LEN)];
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut matches = Vec::new();
    let mut faults = Vec::new();
    for plugin in registry
        .loaded()
        .filter(|p| p.manifest.handles_extension(path))
    {
        match engine.detect(plugin, &file_name, header) {
            Ok(Some(priority)) => matches.push((priority, Arc::clone(plugin))),
            Ok(None) => {}
            Err(e) => faults.push((plugin.manifest.id.clone(), e)),
        }
    }

    let Some(top) = matches.iter().map(|(priority, _)| *priority).max() else {
        return DetectReport {
            detection: Detection::NoMatch,
            faults,
        };
    };
    let mut winners: Vec<_> = matches
        .into_iter()
        .filter(|(priority, _)| *priority == top)
        .map(|(_, plugin)| plugin)
        .collect();
    let detection = if winners.len() == 1 {
        Detection::Winner(winners.remove(0))
    } else {
        Detection::Tie(winners)
    };
    DetectReport { detection, faults }
}
