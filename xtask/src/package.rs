//! `cargo xtask package`: plugin zips (spec §4.1) and the catalog that lists
//! them (`docs/CATALOG_SCHEMA.md`).

use std::fmt::Write as _;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use saveforge_core::manifest::{MANIFEST_FILE_NAME, PluginManifest};
use saveforge_core::settings::file_url;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

/// A built plugin zip.
#[derive(Debug, Clone)]
pub struct Package {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    /// Lowercase hex SHA-256 of the zip.
    pub sha256: String,
}

/// `{id}-{version}.zip`.
pub fn package_file_name(manifest: &PluginManifest) -> String {
    format!("{}-{}.zip", manifest.id, manifest.version)
}

/// Zips `manifest.json`, the component (read from `wasm`) and the icon of the
/// plugin crate at `crate_dir` into `out_dir`. Every entry has a fixed
/// timestamp and permissions, so the same inputs give the same bytes.
pub fn build_package(crate_dir: &Path, wasm: &Path, out_dir: &Path) -> anyhow::Result<Package> {
    let manifest = PluginManifest::load(crate_dir)
        .with_context(|| format!("invalid manifest in {}", crate_dir.display()))?;
    let mut files = vec![
        (
            MANIFEST_FILE_NAME.to_owned(),
            crate_dir.join(MANIFEST_FILE_NAME),
        ),
        (manifest.component.clone(), wasm.to_path_buf()),
    ];
    if let Some(icon) = &manifest.icon {
        files.push((icon.clone(), crate_dir.join(icon)));
    }

    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::default())
        .unix_permissions(0o644);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, source) in &files {
        let bytes =
            fs::read(source).with_context(|| format!("cannot read {}", source.display()))?;
        zip.start_file(name.replace('\\', "/"), options)?;
        zip.write_all(&bytes)?;
    }
    let bytes = zip.finish()?.into_inner();

    fs::create_dir_all(out_dir).with_context(|| format!("cannot create {}", out_dir.display()))?;
    let path = out_dir.join(package_file_name(&manifest));
    // Written aside and renamed, so a failure never leaves a partial zip.
    let mut staged = tempfile::NamedTempFile::new_in(out_dir)
        .with_context(|| format!("cannot write in {}", out_dir.display()))?;
    staged.write_all(&bytes)?;
    staged
        .persist(&path)
        .with_context(|| format!("cannot write {}", path.display()))?;

    Ok(Package {
        manifest,
        path,
        sha256: hex(&Sha256::digest(&bytes)),
    })
}

/// A catalog listing `packages`, each downloadable from `base_url` plus its
/// file name.
pub fn catalog_json(packages: &[Package], base_url: &str) -> anyhow::Result<String> {
    if !(base_url.starts_with("https://") || base_url.starts_with("file://")) {
        bail!("base URL must start with https:// or file://: {base_url}");
    }
    let base = if base_url.ends_with('/') {
        base_url.to_owned()
    } else {
        format!("{base_url}/")
    };
    let plugins: Vec<serde_json::Value> = packages
        .iter()
        .map(|p| {
            let m = &p.manifest;
            serde_json::json!({
                "id": m.id,
                "name": m.name,
                "description": m.description,
                "author": m.author,
                "latestVersion": m.version.to_string(),
                "minHostVersion": m.min_host_version.to_string(),
                "maxHostVersion": m.max_host_version.as_ref().map(ToString::to_string),
                "packageUrl": format!("{base}{}", package_file_name(m)),
                "sha256": p.sha256,
                "tags": [],
            })
        })
        .collect();
    let catalog = serde_json::json!({ "schemaVersion": "1.0", "plugins": plugins });
    Ok(serde_json::to_string_pretty(&catalog)? + "\n")
}

/// Packages every `(crate_dir, wasm)` pair and writes `catalog.json` into
/// `out`. Everything is built in `{out}.tmp` first and only replaces `out`
/// when it all succeeded, so a failure keeps the previous output. Without
/// `base_url`, package URLs point at `out` itself (`file://`).
pub fn write_packages(
    out: &Path,
    plugins: &[(PathBuf, PathBuf)],
    base_url: Option<&str>,
) -> anyhow::Result<(Vec<Package>, String)> {
    let mut staging = out.as_os_str().to_owned();
    staging.push(".tmp");
    let staging = PathBuf::from(staging);
    let result = build_all(&staging, out, plugins, base_url);
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
        return result;
    }
    if out.exists() {
        fs::remove_dir_all(out).with_context(|| format!("cannot clear {}", out.display()))?;
    }
    fs::rename(&staging, out)
        .with_context(|| format!("cannot move {} to {}", staging.display(), out.display()))?;
    result
}

fn build_all(
    staging: &Path,
    out: &Path,
    plugins: &[(PathBuf, PathBuf)],
    base_url: Option<&str>,
) -> anyhow::Result<(Vec<Package>, String)> {
    if staging.exists() {
        fs::remove_dir_all(staging)
            .with_context(|| format!("cannot clear {}", staging.display()))?;
    }
    let mut packages = Vec::new();
    for (crate_dir, wasm) in plugins {
        let mut package = build_package(crate_dir, wasm, staging)?;
        package.path = out.join(package_file_name(&package.manifest));
        packages.push(package);
    }
    let default_base = format!("{}/", file_url(out));
    let catalog = catalog_json(&packages, base_url.unwrap_or(&default_base))?;
    fs::write(staging.join("catalog.json"), &catalog)
        .with_context(|| format!("cannot write {}", staging.display()))?;
    Ok((packages, catalog))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}
