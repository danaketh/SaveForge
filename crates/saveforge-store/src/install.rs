//! Installs, updates and removes plugin packages (spec §5.2 Store step 4).
//!
//! Every install stages under `{plugins}/.staging` in temp files and dirs that
//! clean up on drop, so a failure leaves `.staging` empty and any existing
//! install untouched.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use saveforge_core::manifest::{PluginManifest, is_safe_relative_path, is_valid_id};
use saveforge_host::{API_VERSION, HOST_VERSION};
use semver::Version;
use sha2::{Digest, Sha256};

use crate::transport::{MAX_PACKAGE_BYTES, read_url};
use crate::{CatalogEntry, StoreError};

pub const MAX_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024;

/// Most entries a package may hold.
pub const MAX_ZIP_ENTRIES: usize = 10_000;

const STAGING: &str = ".staging";

/// Downloads, verifies and unpacks `entry` into `{plugins_dir}/{id}/{version}`,
/// replacing that version if it is already installed. Returns the new dir.
pub fn install(
    entry: &CatalogEntry,
    plugins_dir: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<PathBuf, StoreError> {
    check_id(&entry.id)?;
    let staging = plugins_dir.join(STAGING);
    fs::create_dir_all(&staging).map_err(|e| StoreError::io(&staging, e))?;

    let mut download =
        tempfile::NamedTempFile::new_in(&staging).map_err(|e| StoreError::io(&staging, e))?;
    let mut writer = HashingWriter {
        inner: download.as_file_mut(),
        hasher: Sha256::new(),
    };
    read_url(&entry.package_url, MAX_PACKAGE_BYTES, &mut writer, progress)?;
    let actual = hex(&writer.hasher.finalize());
    let expected = entry.sha256.to_ascii_lowercase();
    if actual != expected {
        return Err(StoreError::HashMismatch { expected, actual });
    }

    let staged = tempfile::Builder::new()
        .prefix("pkg-")
        .tempdir_in(&staging)
        .map_err(|e| StoreError::io(&staging, e))?;
    download
        .as_file_mut()
        .seek(SeekFrom::Start(0))
        .map_err(|e| StoreError::io(download.path(), e))?;
    extract_zip(
        BufReader::new(download.as_file()),
        staged.path(),
        MAX_EXTRACTED_BYTES,
    )?;

    let manifest = PluginManifest::load(staged.path())?;
    let expected = format!("{} {}", entry.id, entry.latest_version);
    let found = format!("{} {}", manifest.id, manifest.version);
    if found != expected {
        return Err(StoreError::ManifestMismatch { expected, found });
    }
    manifest.check_compat(&HOST_VERSION, &API_VERSION)?;

    let target = plugins_dir
        .join(&entry.id)
        .join(entry.latest_version.to_string());
    move_into_place(staged.path(), &target, &staging, &mut retry_rename)?;
    // Moved into place: nothing left for the temp dir to clean up.
    let _ = staged.keep();
    Ok(target)
}

/// Renames the staged package dir to `target`. An existing `target` waits in
/// `staging` until the new one is in place and is moved back if that fails.
/// If even that fails, the old copy is kept in `staging` (and logged) rather
/// than deleted, and an `{id}` dir left empty is removed.
pub fn move_into_place(
    staged: &Path,
    target: &Path,
    staging: &Path,
    rename: &mut dyn FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<(), StoreError> {
    let parent = target.parent().unwrap_or(target);
    fs::create_dir_all(parent).map_err(|e| StoreError::io(parent, e))?;
    let previous = if target.exists() {
        let holder = tempfile::Builder::new()
            .prefix("old-")
            .tempdir_in(staging)
            .map_err(|e| StoreError::io(staging, e))?;
        let moved = holder.path().join("old");
        rename(target, &moved).map_err(|e| StoreError::io(target, e))?;
        Some((holder, moved))
    } else {
        None
    };
    let Err(e) = rename(staged, target) else {
        return Ok(());
    };
    if let Some((holder, moved)) = previous
        && let Err(restore) = rename(&moved, target)
    {
        let kept = holder.keep();
        tracing::error!(
            target = %target.display(),
            kept = %kept.display(),
            %restore,
            "could not restore the previous install; it was kept"
        );
    }
    // Only removes the dir when empty, i.e. this was a first install.
    let _ = fs::remove_dir(parent);
    Err(StoreError::io(target, e))
}

/// `fs::rename`, retried while the OS reports the files as locked (an
/// antivirus scanner holding a just-extracted file, on Windows).
fn retry_rename(from: &Path, to: &Path) -> io::Result<()> {
    retry(5, Duration::from_millis(200), || fs::rename(from, to))
}

/// Runs `op` up to `attempts` times, `delay` apart, while it fails with
/// `PermissionDenied`.
pub fn retry<T>(
    attempts: u32,
    delay: Duration,
    mut op: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let mut attempt = 1;
    loop {
        match op() {
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && attempt < attempts => {
                attempt += 1;
                std::thread::sleep(delay);
            }
            result => return result,
        }
    }
}

/// Unpacks a zip into `dest`, rejecting entries that would escape it,
/// symlinks, and archives that expand past `limit` bytes.
pub fn extract_zip(reader: impl Read + Seek, dest: &Path, limit: u64) -> Result<(), StoreError> {
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| StoreError::BadPackage(e.to_string()))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(StoreError::BadPackage(format!(
            "more than {MAX_ZIP_ENTRIES} entries"
        )));
    }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| StoreError::BadPackage(e.to_string()))?;
        let name = file
            .name()
            .map_err(|e| StoreError::BadPackage(e.to_string()))?
            .into_owned();
        // `enclosed_name` strips a leading `/` or drive instead of failing,
        // so the raw name must also be a safe relative path.
        let relative = match file.enclosed_name() {
            Some(relative) if is_safe_relative_path(&name) && !file.is_symlink() => relative,
            _ => return Err(StoreError::UnsafePackage(name)),
        };
        let path = dest.join(relative);
        if file.is_dir() {
            fs::create_dir_all(&path).map_err(|e| StoreError::io(&path, e))?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| StoreError::io(parent, e))?;
        }
        let mut out = File::create(&path).map_err(|e| StoreError::io(&path, e))?;
        let written = io::copy(&mut (&mut file).take(limit - total + 1), &mut out)
            .map_err(|e| StoreError::BadPackage(format!("{name}: {e}")))?;
        total += written;
        if total > limit {
            return Err(StoreError::PackageTooLarge);
        }
    }
    Ok(())
}

/// Deletes every installed version of plugin `id`. A missing plugin is fine.
pub fn remove(plugins_dir: &Path, id: &str) -> Result<(), StoreError> {
    check_id(id)?;
    remove_dir(&plugins_dir.join(id))
}

/// Deletes one installed version of plugin `id`.
pub fn remove_version(plugins_dir: &Path, id: &str, version: &Version) -> Result<(), StoreError> {
    check_id(id)?;
    remove_dir(&plugins_dir.join(id).join(version.to_string()))
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

fn check_id(id: &str) -> Result<(), StoreError> {
    if is_valid_id(id) {
        Ok(())
    } else {
        Err(StoreError::InvalidId(id.to_owned()))
    }
}

fn remove_dir(path: &Path) -> Result<(), StoreError> {
    match fs::remove_dir_all(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(StoreError::io(path, e)),
        _ => Ok(()),
    }
}

/// Writes through to `inner` while hashing.
struct HashingWriter<'a> {
    inner: &'a mut File,
    hasher: Sha256,
}

impl Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
