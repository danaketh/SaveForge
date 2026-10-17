//! Helpers behind `cargo xtask` (spec §10).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use saveforge_core::manifest::PluginManifest;

pub mod package;

pub const WASM_TARGET: &str = "wasm32-wasip2";

/// Workspace root: parent of the xtask crate. Prefers the `CARGO_MANIFEST_DIR`
/// cargo sets at run time over the compile-time one, so a binary reused from
/// another checkout (shared target dir) still builds *this* checkout.
pub fn workspace_root(runtime_manifest_dir: Option<OsString>, compile_time: &str) -> PathBuf {
    let manifest_dir =
        runtime_manifest_dir.map_or_else(|| PathBuf::from(compile_time), PathBuf::from);
    manifest_dir
        .parent()
        .map_or_else(|| manifest_dir.clone(), Path::to_path_buf)
}

/// Cargo's target dir as the child `cargo build` (run in `root`) will see it:
/// `CARGO_TARGET_DIR` if set (relative values are relative to `root`),
/// otherwise `root/target`.
pub fn target_dir(root: &Path, cargo_target_dir: Option<OsString>) -> PathBuf {
    cargo_target_dir.map_or_else(|| root.join("target"), |dir| root.join(dir))
}

/// Where a plugin crate lives: `plugins/` ships to users, `fixtures/` feeds tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Plugin,
    Fixture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCrate {
    pub kind: PluginKind,
    /// Cargo package name; by convention equal to the directory name.
    pub package: String,
    pub dir: PathBuf,
}

/// Every crate under `plugins/` and `fixtures/` with both `Cargo.toml` and
/// `manifest.json`, sorted by package name.
pub fn find_plugin_crates(workspace: &Path) -> Vec<PluginCrate> {
    let mut found = Vec::new();
    for (sub, kind) in [
        ("plugins", PluginKind::Plugin),
        ("fixtures", PluginKind::Fixture),
    ] {
        let Ok(entries) = fs::read_dir(workspace.join(sub)) else {
            continue;
        };
        for entry in entries.flatten() {
            let dir = entry.path();
            if dir.join("Cargo.toml").is_file() && dir.join("manifest.json").is_file() {
                found.push(PluginCrate {
                    kind,
                    package: entry.file_name().to_string_lossy().into_owned(),
                    dir,
                });
            }
        }
    }
    found.sort_by(|a, b| a.package.cmp(&b.package));
    found
}

/// The crates `filter` names (by package name or plugin id), or all of them.
/// With `shippable_only`, test fixtures are left out, and naming one is an
/// error.
pub fn select_plugins(
    crates: Vec<PluginCrate>,
    filter: Option<&str>,
    shippable_only: bool,
) -> anyhow::Result<Vec<PluginCrate>> {
    let selected: Vec<PluginCrate> = crates
        .into_iter()
        .filter(|c| {
            filter.is_none_or(|f| {
                c.package == f || PluginManifest::load(&c.dir).is_ok_and(|m| m.id == f)
            })
        })
        .collect();
    if let (Some(name), true) = (filter, shippable_only)
        && selected.iter().any(|c| c.kind == PluginKind::Fixture)
    {
        bail!("{name} is a test fixture, not a plugin");
    }
    let selected: Vec<PluginCrate> = selected
        .into_iter()
        .filter(|c| !shippable_only || c.kind == PluginKind::Plugin)
        .collect();
    match (selected.is_empty(), filter) {
        (true, Some(name)) => bail!("no plugin crate matched {name}"),
        (true, None) => bail!("no plugin crate found"),
        (false, _) => Ok(selected),
    }
}

/// The component cargo writes for `package` on the wasm target.
pub fn wasm_artifact(target_dir: &Path, package: &str) -> PathBuf {
    target_dir
        .join(WASM_TARGET)
        .join("release")
        .join(format!("{}.wasm", package.replace('-', "_")))
}

/// `target/dev-plugins/{id}/{version}`, the debug app's plugin directory layout.
pub fn dev_plugin_dir(target_dir: &Path, manifest: &PluginManifest) -> PathBuf {
    target_dir
        .join("dev-plugins")
        .join(&manifest.id)
        .join(manifest.version.to_string())
}

/// `target/fixtures/{package}.wasm`, read by host tests.
pub fn fixture_wasm(target_dir: &Path, package: &str) -> PathBuf {
    target_dir.join("fixtures").join(format!("{package}.wasm"))
}

/// Copies a built crate into place and returns the destination. A plugin
/// directory is replaced wholesale so stale files never linger.
pub fn install(target_dir: &Path, krate: &PluginCrate) -> anyhow::Result<PathBuf> {
    let manifest = PluginManifest::load(&krate.dir)
        .with_context(|| format!("invalid manifest in {}", krate.dir.display()))?;
    let wasm = wasm_artifact(target_dir, &krate.package);
    ensure!(wasm.is_file(), "build output missing: {}", wasm.display());

    match krate.kind {
        PluginKind::Fixture => {
            let dest = fixture_wasm(target_dir, &krate.package);
            fs::create_dir_all(target_dir.join("fixtures"))?;
            fs::copy(&wasm, &dest)?;
            Ok(dest)
        }
        PluginKind::Plugin => {
            let dest = dev_plugin_dir(target_dir, &manifest);
            if dest.exists() {
                fs::remove_dir_all(&dest)?;
            }
            fs::create_dir_all(&dest)?;
            fs::copy(krate.dir.join("manifest.json"), dest.join("manifest.json"))?;
            copy_into(&wasm, &dest.join(&manifest.component))?;
            if let Some(icon) = &manifest.icon {
                copy_into(&krate.dir.join(icon), &dest.join(icon))?;
            }
            Ok(dest)
        }
    }
}

fn copy_into(from: &Path, to: &Path) -> anyhow::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to).with_context(|| format!("copy {} -> {}", from.display(), to.display()))?;
    Ok(())
}
