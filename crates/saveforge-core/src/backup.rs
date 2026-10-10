//! Backup-then-atomic-replace for save files (spec §5.2 Save).

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::macros::format_description;

use crate::{CoreError, Result};

pub const TMP_SUFFIX: &str = ".saveforge.tmp";

/// Copies `target` to `{backup_root}/{plugin_id}/{file}.{utc-stamp}.bak`, then
/// atomically replaces `target` with `bytes`. Returns the backup path.
///
/// If the backup fails nothing is written. If the write or rename fails the
/// original file is untouched and the temp file is removed.
pub fn save_with_backup(
    target: &Path,
    bytes: &[u8],
    backup_root: &Path,
    plugin_id: &str,
) -> Result<PathBuf> {
    save_with_backup_at(
        target,
        bytes,
        backup_root,
        plugin_id,
        OffsetDateTime::now_utc(),
        |from, to| fs::rename(from, to),
    )
}

/// [`save_with_backup`] with an injectable clock and rename (for tests).
pub fn save_with_backup_at(
    target: &Path,
    bytes: &[u8],
    backup_root: &Path,
    plugin_id: &str,
    now: OffsetDateTime,
    rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<PathBuf> {
    let backup = backup_file(target, backup_root, plugin_id, now)?;
    write_atomic_with(target, bytes, rename)?;
    Ok(backup)
}

/// Writes `bytes` to a sibling temp file, fsyncs, then renames it over `target`.
pub fn write_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    write_atomic_with(target, bytes, |from, to| fs::rename(from, to))
}

fn write_atomic_with(
    target: &Path,
    bytes: &[u8],
    rename: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<()> {
    let tmp = target.with_file_name(format!("{}{TMP_SUFFIX}", file_name_of(target)?));
    let result = write_and_sync(&tmp, bytes).and_then(|()| rename(&tmp, target));
    if let Err(err) = result {
        if let Err(cleanup) = fs::remove_file(&tmp)
            && cleanup.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(tmp = %tmp.display(), %cleanup, "could not remove temp file");
        }
        return Err(CoreError::io(target, err));
    }
    Ok(())
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn backup_file(
    target: &Path,
    backup_root: &Path,
    plugin_id: &str,
    now: OffsetDateTime,
) -> Result<PathBuf> {
    let file_name = file_name_of(target)?;
    let dir = backup_root.join(plugin_id);
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    let stamp = now
        .format(format_description!(
            "[year]-[month]-[day]T[hour]-[minute]-[second]Z"
        ))
        .map_err(|e| CoreError::io(target, io::Error::other(e)))?;

    let mut candidate = dir.join(format!("{file_name}.{stamp}.bak"));
    let mut n = 1u32;
    while candidate.exists() {
        candidate = dir.join(format!("{file_name}.{stamp}-{n}.bak"));
        n += 1;
    }
    fs::copy(target, &candidate).map_err(|e| CoreError::io(target, e))?;
    Ok(candidate)
}

fn file_name_of(path: &Path) -> Result<String> {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| {
            CoreError::io(
                path,
                io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"),
            )
        })
}
