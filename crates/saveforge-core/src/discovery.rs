//! Recursive save-file discovery for a plugin's save directory (spec §5.2 Home).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::manifest::matches_extension;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFileInfo {
    pub path: PathBuf,
    pub file_name: String,
    pub modified: SystemTime,
    pub size_bytes: u64,
}

/// All files under `dir` whose extension is in `extensions`, newest first.
///
/// Never fails. Unreadable directories and entries are skipped. Symlinks are
/// not followed, so link loops cannot hang the scan.
pub fn discover(dir: &Path, extensions: &[String]) -> Vec<SaveFileInfo> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(err) => {
                tracing::debug!(dir = %current.display(), %err, "skipping unreadable directory");
                continue;
            }
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() && matches_extension(&path, extensions) {
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                found.push(SaveFileInfo {
                    file_name: entry.file_name().to_string_lossy().into_owned(),
                    modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    size_bytes: meta.len(),
                    path,
                });
            }
        }
    }
    found.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    found
}
