//! Recursive save-file discovery for a plugin's save directory (spec §5.2 Home).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::manifest::{GroupingSource, SaveGrouping, matches_extension};

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

/// A save as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedSave {
    pub label: String,
    pub file: SaveFileInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveGroup {
    /// `None` for the saves no grouping applies to.
    pub name: Option<String>,
    pub saves: Vec<ListedSave>,
}

/// Groups `saves` (as [`discover`] orders them) by `grouping`.
///
/// Groups and the saves in them keep that order, so the group with the newest
/// save comes first. A save is labelled by the pattern's `name` capture, else
/// by its file name. Saves the pattern does not match (or whose `group` is
/// empty), or every save when there is no grouping, form one unnamed group at
/// the end.
pub fn group_saves(saves: Vec<SaveFileInfo>, grouping: Option<&SaveGrouping>) -> Vec<SaveGroup> {
    let rule = grouping.and_then(|g| match g.regex() {
        Ok(regex) => Some((g.source, regex)),
        Err(err) => {
            tracing::warn!(%err, "ignoring save grouping");
            None
        }
    });
    let mut groups: Vec<SaveGroup> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut ungrouped = Vec::new();
    for file in saves {
        let captures = rule.as_ref().and_then(|(source, regex)| {
            let text = match source {
                GroupingSource::Directory => file.path.parent()?.file_name()?,
                GroupingSource::File => file.path.file_stem()?,
            };
            let text = text.to_str()?;
            let caps = regex.captures(text)?;
            let group = caps
                .name("group")
                .map(|m| m.as_str().to_owned())
                .filter(|group| !group.is_empty())?;
            let label = caps
                .name("name")
                .map(|m| m.as_str().to_owned())
                .filter(|label| !label.is_empty());
            Some((group, label))
        });
        let Some((group, label)) = captures else {
            ungrouped.push(ListedSave {
                label: file.file_name.clone(),
                file,
            });
            continue;
        };
        let save = ListedSave {
            label: label.unwrap_or_else(|| file.file_name.clone()),
            file,
        };
        let at = *index.entry(group.clone()).or_insert_with(|| {
            groups.push(SaveGroup {
                name: Some(group),
                saves: Vec::new(),
            });
            groups.len() - 1
        });
        groups[at].saves.push(save);
    }
    if !ungrouped.is_empty() {
        groups.push(SaveGroup {
            name: None,
            saves: ungrouped,
        });
    }
    groups
}
