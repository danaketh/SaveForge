//! Locates the manifest's auxiliary files for `open` (spec §4.2, §4.5).

use saveforge_core::manifest::{AuxFileSpec, PluginManifest};
use saveforge_core::tokens::{Expanded, TokenEnv, expand};
use saveforge_plugin_api::AuxFile;

use crate::error::HostError;

/// Reads each declared aux file from its first usable candidate, in manifest
/// order. A missing optional file is omitted; a missing required one is an error.
pub fn resolve_aux_files(
    manifest: &PluginManifest,
    env: &TokenEnv,
) -> Result<Vec<AuxFile>, HostError> {
    let mut files = Vec::new();
    for spec in &manifest.aux_files {
        match read_first_candidate(spec, env) {
            Some(bytes) => files.push(AuxFile {
                id: spec.id.clone(),
                bytes,
            }),
            None if spec.optional => {
                tracing::debug!(plugin = %manifest.id, aux = %spec.id, "optional aux file not found");
            }
            None => return Err(HostError::MissingAux(spec.id.clone())),
        }
    }
    Ok(files)
}

fn read_first_candidate(spec: &AuxFileSpec, env: &TokenEnv) -> Option<Vec<u8>> {
    spec.candidates.iter().find_map(|raw| {
        let Expanded::Resolved(path) = expand(raw, env) else {
            return None;
        };
        let meta = std::fs::metadata(&path).ok()?;
        if !meta.is_file() {
            return None;
        }
        if meta.len() > spec.max_bytes {
            tracing::warn!(aux = %spec.id, path = %path.display(), size = meta.len(), "aux file too large");
            return None;
        }
        let bytes = std::fs::read(&path).ok()?;
        // The file may have grown since the metadata call.
        (u64::try_from(bytes.len()).unwrap_or(u64::MAX) <= spec.max_bytes).then_some(bytes)
    })
}
