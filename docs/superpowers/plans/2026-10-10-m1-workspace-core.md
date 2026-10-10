# Milestone 1: Workspace + `saveforge-core` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the cargo workspace with its lint gate and CI, plus the `saveforge-core` crate. The crate covers plugin manifests, path tokens, Steam library lookup, save discovery, backup with atomic write, and settings. Every piece is fully tested.

**Architecture:** `saveforge-core` is a plain library with no UI, async, or wasm. Its functions are synchronous and do their own file I/O. Inputs that depend on the machine are injectable for tests: clock, rename, token environment, Steam root. Later crates (`host`, `store`, `app`) depend on it.

**Tech Stack:** Rust 1.99 (edition 2024), `serde`/`serde_json`, `semver`, `thiserror`, `directories`, `time`, `tracing`, `winreg` (Windows only). Tests use `tempfile` and `filetime`.

**Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md` (§3.1–3.3, §4.2, §4.3, §5.2 Home/Save, §5.3, §7, §8 `saveforge-core` row, §9)

## Global Constraints

- Workspace root is `rust/` (this repo). Edition `2024`, `resolver = "3"`, `rust-version = "1.90"`, all crates `publish = false`.
- Workspace `[lints]`: `unsafe_code = "forbid"`, `clippy::all` + `clippy::pedantic` at warn. Allow-list only `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`. CI runs `cargo clippy --workspace --all-targets -- -D warnings`.
- Release profile: `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`, `opt-level = 3`.
- Library errors are a `thiserror` enum (`CoreError`). No `anyhow` in library crates.
- Plugin id pattern: `^[a-z0-9]+(-[a-z0-9]+)*$`.
- `auxFiles[].maxBytes` defaults to 8 MB (8388608), with a hard cap of 64 MB (67108864).
- `apiVersion` before 1.0 is compatible only with the same `major.minor`. From 1.0 on, it needs the same major and plugin minor ≤ host minor.
- Backup file name: `{backupDir}/{pluginId}/{fileName}.{yyyy-MM-ddTHH-mm-ssZ}.bak` (UTC). Temp file: `{fileName}.saveforge.tmp` in the target's directory.
- Unknown path tokens are left verbatim. If a known token can't be resolved, the whole path is unresolved.
- Settings JSON keys are camelCase: `catalogUrl`, `pluginDirectory`, `backupDirectory`, `checkUpdatesOnStartup`, `pluginSaveDirectories`.
- Release catalog URL default: `https://raw.githubusercontent.com/YOUR_ORG/YOUR_REPO/main/catalog.json`. Debug default: `file://<data_dir>/catalog.json`. Debug plugin dir: `<workspace>/target/dev-plugins`.
- Tests live in `crates/<crate>/tests/*.rs`, not inline modules.
- Work on branch `feat/m1-workspace-core`, never on `main`.

## Review Focus

These five inputs are implied by the spec but easy to miss. Each one is pinned by a test in the task listed.

1. **Two saves of one file within the same second.** .NET used `File.Copy(overwrite: false)`, so the second save failed. Expected: the second backup gets a `-1` suffix and the save succeeds. Covered in Task 6 (`second_backup_in_same_second_gets_suffix`).
2. **Save files with upper-case extensions** (`SAVE.JSON`) on case-sensitive filesystems. Expected: they are discovered and detected. Covered in Task 2 (`handles_extension_is_case_insensitive`) and Task 5 (`extension_filter_is_case_insensitive`).
3. **User types a lower-case token** (`%appdata%/...`) in the path editor. Expected: it expands like `%APPDATA%`. Covered in Task 4 (`percent_tokens_are_case_insensitive`).
4. **First run with no app data dir.** Expected: saving settings creates the parent directories. Covered in Task 7 (`save_creates_missing_parent_dirs`).
5. **Windows-style traversal (`..\evil.wasm`, `C:\x.wasm`) in a manifest** from a zip built on Windows. Expected: rejected on every OS. Covered in Task 2 (`rejects_unsafe_relative_paths`).

---

## File Structure

```
rust/
  Cargo.toml                         # workspace, shared deps, lints, release profile
  rustfmt.toml
  deny.toml
  .github/workflows/ci.yml
  crates/saveforge-core/
    Cargo.toml
    src/lib.rs                       # module wiring, re-exports
    src/error.rs                     # CoreError, Result
    src/platform.rs                  # Os enum
    src/manifest.rs                  # PluginManifest parse/validate/compat, extension matching
    src/steam/mod.rs                 # SteamLibraries, find_steam_root, library_paths_from_vdf
    src/steam/vdf.rs                 # minimal Valve KeyValues parser
    src/tokens.rs                    # TokenEnv, expand()
    src/discovery.rs                 # SaveFileInfo, discover()
    src/backup.rs                    # save_with_backup(), write_atomic()
    src/paths.rs                     # AppPaths (data dir layout)
    src/settings.rs                  # AppSettings load/save/defaults, file_url()
    tests/platform.rs
    tests/manifest.rs
    tests/steam.rs
    tests/tokens.rs
    tests/discovery.rs
    tests/backup.rs
    tests/settings.rs
```

---

### Task 1: Workspace scaffold, lint gate, CI, `Os` + `CoreError`

**Files:**
- Create: `Cargo.toml`, `rustfmt.toml`, `deny.toml`, `.github/workflows/ci.yml`
- Create: `crates/saveforge-core/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/platform.rs`
- Test: `crates/saveforge-core/tests/platform.rs`

**Interfaces:**
- Produces: `saveforge_core::platform::Os { Windows, Linux, MacOs }` with `const fn current() -> Os`. `saveforge_core::CoreError` with variants `Io { path, source }`, `Json { path, source }`, `Manifest(String)`, `Incompatible(String)`. `saveforge_core::Result<T>`.

- [ ] **Step 1: Create the branch**

```bash
git checkout -b feat/m1-workspace-core
```

- [ ] **Step 2: Write workspace `Cargo.toml`**

```toml
[workspace]
resolver = "3"
members = ["crates/saveforge-core"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.90"
publish = false

[workspace.dependencies]
directories = "6.0.0"
filetime = "0.2.29"
semver = { version = "1.0.28", features = ["serde"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
tempfile = "3.27.0"
thiserror = "2.0.21"
time = { version = "0.3.55", features = ["formatting", "macros"] }
tracing = "0.1.44"
winreg = "0.56.0"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"

[profile.release]
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
opt-level = 3
```

- [ ] **Step 3: Write `rustfmt.toml`**

```toml
edition = "2024"
max_width = 100
```

- [ ] **Step 4: Write `deny.toml`**

```toml
[graph]
all-features = true

[advisories]
yanked = "deny"

[licenses]
confidence-threshold = 0.9
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Zlib",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"
allow-wildcard-paths = true

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

- [ ] **Step 5: Write `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  lint:
    name: fmt / deny / machete
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-deny,cargo-machete
      - run: cargo deny check
      - run: cargo machete

  test:
    name: clippy + test (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

- [ ] **Step 6: Write `crates/saveforge-core/Cargo.toml`**

```toml
[package]
name = "saveforge-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
directories.workspace = true
semver.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
time.workspace = true
tracing.workspace = true

[target.'cfg(windows)'.dependencies]
winreg.workspace = true

[dev-dependencies]
filetime.workspace = true
tempfile.workspace = true

[lints]
workspace = true
```

- [ ] **Step 7: Write the failing test `crates/saveforge-core/tests/platform.rs`**

```rust
use saveforge_core::platform::Os;

#[test]
fn current_os_matches_compile_target() {
    let expected = if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else {
        Os::Linux
    };
    assert_eq!(Os::current(), expected);
}
```

- [ ] **Step 8: Run test to verify it fails**

Run: `cargo test -p saveforge-core --test platform`
Expected: compile error (`src/lib.rs` missing / unresolved import `saveforge_core::platform`).

- [ ] **Step 9: Write `src/error.rs`**

```rust
use std::path::{Path, PathBuf};

/// Errors produced by `saveforge-core`.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("I/O error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON error at {}: {source}", path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid manifest: {0}")]
    Manifest(String),
    #[error("incompatible plugin: {0}")]
    Incompatible(String),
}

impl CoreError {
    pub(crate) fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

pub type Result<T, E = CoreError> = std::result::Result<T, E>;
```

- [ ] **Step 10: Write `src/platform.rs`**

```rust
/// Operating system families the host supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    Windows,
    Linux,
    MacOs,
}

impl Os {
    /// The OS this binary was compiled for.
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}
```

- [ ] **Step 11: Write `src/lib.rs`**

```rust
//! Domain logic for the `SaveForge` host: plugin manifests, path tokens, Steam
//! library lookup, save discovery, safe file writes and settings.

mod error;
pub mod platform;

pub use error::{CoreError, Result};
```

- [ ] **Step 12: Run test and the lint gate**

Run: `cargo test -p saveforge-core --test platform`
Expected: `test current_os_matches_compile_target ... ok`

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no output from fmt, clippy finishes with no warnings. Clippy may flag the unused `CoreError::io` helper as dead code. If it does, add `#[allow(dead_code)] // used from Task 2 on` above `pub(crate) fn io` and remove that attribute in Task 2.

- [ ] **Step 13: Commit**

```bash
git add Cargo.toml rustfmt.toml deny.toml .github crates
git commit -m "chore: scaffold cargo workspace, lint gate and CI"
```

---

### Task 2: Plugin manifest parsing, validation, compatibility

**Files:**
- Create: `crates/saveforge-core/src/manifest.rs`
- Modify: `crates/saveforge-core/src/lib.rs` (add `pub mod manifest;`)
- Test: `crates/saveforge-core/tests/manifest.rs`

**Interfaces:**
- Consumes: `CoreError::{Io, Manifest, Incompatible}`, `CoreError::io(&Path, io::Error)`, `Os`.
- Produces:
  - `PluginManifest { id, name, version: semver::Version, author, description, api_version: String, min_host_version: Version, max_host_version: Option<Version>, component: String, icon: Option<String>, file_extensions: Vec<String>, default_save_directories: DefaultSaveDirectories, aux_files: Vec<AuxFileSpec> }`
  - `PluginManifest::parse(&str) -> Result<Self>`, `PluginManifest::load(plugin_dir: &Path) -> Result<Self>`, `PluginManifest::check_compat(&self, host_version: &Version, host_api: &Version) -> Result<()>`, `PluginManifest::handles_extension(&self, path: &Path) -> bool`
  - `DefaultSaveDirectories { windows, linux, macos: Option<String> }` with `for_os(Os) -> Option<&str>`
  - `AuxFileSpec { id: String, optional: bool, max_bytes: u64, candidates: Vec<String> }`
  - free fns `is_valid_id(&str) -> bool`, `is_safe_relative_path(&str) -> bool`, `matches_extension(path: &Path, extensions: &[String]) -> bool`
  - consts `MANIFEST_FILE_NAME = "manifest.json"`, `DEFAULT_AUX_MAX_BYTES = 8_388_608`, `AUX_MAX_BYTES_CAP = 67_108_864`

- [ ] **Step 1: Write the failing tests `tests/manifest.rs`**

```rust
use std::path::Path;

use saveforge_core::CoreError;
use saveforge_core::manifest::{
    AUX_MAX_BYTES_CAP, DEFAULT_AUX_MAX_BYTES, PluginManifest, is_safe_relative_path, is_valid_id,
};
use saveforge_core::platform::Os;
use semver::Version;
use serde_json::{Value, json};

const VALID: &str = r#"{
  "id": "planet-crafter",
  "name": "Planet Crafter Save Editor",
  "version": "1.0.0",
  "author": "SaveForge Team",
  "description": "Edit saves for Planet Crafter.",
  "apiVersion": "0.1",
  "minHostVersion": "1.0.0",
  "maxHostVersion": null,
  "component": "plugin.wasm",
  "icon": "icon.png",
  "fileExtensions": [".json"],
  "defaultSaveDirectories": {
    "windows": "%APPDATA%/MijuGames/The Planet Crafter/Saves",
    "linux": "{steam_compat:1284190}/AppData/Roaming/MijuGames/The Planet Crafter/Saves",
    "macos": null
  },
  "auxFiles": [
    { "id": "localization", "optional": true,
      "candidates": ["{steam_app:1284190}/Planet Crafter_Data/Resources/LocalizationSpaceCrafter.txt"] }
  ]
}"#;

fn with(field: &str, value: Value) -> String {
    let mut v: Value = serde_json::from_str(VALID).unwrap();
    v[field] = value;
    v.to_string()
}

fn without(field: &str) -> String {
    let mut v: Value = serde_json::from_str(VALID).unwrap();
    v.as_object_mut().unwrap().remove(field);
    v.to_string()
}

fn assert_manifest_err(json: &str) {
    match PluginManifest::parse(json) {
        Err(CoreError::Manifest(_)) => {}
        other => panic!("expected Manifest error, got {other:?}"),
    }
}

fn v(s: &str) -> Version {
    Version::parse(s).unwrap()
}

#[test]
fn parses_valid_manifest() {
    let m = PluginManifest::parse(VALID).unwrap();
    assert_eq!(m.id, "planet-crafter");
    assert_eq!(m.version, v("1.0.0"));
    assert_eq!(m.api_version, "0.1");
    assert_eq!(m.max_host_version, None);
    assert_eq!(m.component, "plugin.wasm");
    assert_eq!(m.icon.as_deref(), Some("icon.png"));
    assert_eq!(
        m.default_save_directories.for_os(Os::Windows),
        Some("%APPDATA%/MijuGames/The Planet Crafter/Saves")
    );
    assert_eq!(m.default_save_directories.for_os(Os::MacOs), None);
    assert_eq!(m.aux_files.len(), 1);
    assert!(m.aux_files[0].optional);
    assert_eq!(m.aux_files[0].max_bytes, DEFAULT_AUX_MAX_BYTES);
}

#[test]
fn optional_fields_default() {
    let mut v: Value = serde_json::from_str(VALID).unwrap();
    let obj = v.as_object_mut().unwrap();
    for key in ["description", "maxHostVersion", "icon", "defaultSaveDirectories", "auxFiles"] {
        obj.remove(key);
    }
    let m = PluginManifest::parse(&v.to_string()).unwrap();
    assert_eq!(m.description, "");
    assert_eq!(m.icon, None);
    assert_eq!(m.default_save_directories.for_os(Os::Linux), None);
    assert!(m.aux_files.is_empty());
}

#[test]
fn unknown_fields_are_ignored() {
    PluginManifest::parse(&with("futureField", json!(42))).unwrap();
}

#[test]
fn missing_required_field_is_manifest_error() {
    assert_manifest_err(&without("component"));
    assert_manifest_err(&with("version", json!("not-semver")));
}

#[test]
fn id_pattern() {
    for ok in ["planet-crafter", "a1", "x"] {
        assert!(is_valid_id(ok), "{ok}");
    }
    for bad in ["", "Planet", "planet--crafter", "-x", "x-", "planet_crafter", "pl anet"] {
        assert!(!is_valid_id(bad), "{bad}");
        assert_manifest_err(&with("id", json!(bad)));
    }
}

#[test]
fn rejects_unsafe_relative_paths() {
    for ok in ["plugin.wasm", "bin/plugin.wasm", "bin\\plugin.wasm"] {
        assert!(is_safe_relative_path(ok), "{ok}");
    }
    for bad in [
        "",
        "../evil.wasm",
        "..\\evil.wasm",
        "sub/../../x.wasm",
        "/abs.wasm",
        "\\abs.wasm",
        "C:\\evil.wasm",
        "C:evil.wasm",
    ] {
        assert!(!is_safe_relative_path(bad), "{bad}");
        assert_manifest_err(&with("component", json!(bad)));
        assert_manifest_err(&with("icon", json!(bad)));
    }
}

#[test]
fn file_extensions_rules() {
    assert_manifest_err(&with("fileExtensions", json!([])));
    assert_manifest_err(&with("fileExtensions", json!(["json"])));
    assert_manifest_err(&with("fileExtensions", json!(["."])));
}

#[test]
fn aux_file_rules() {
    let aux = |items: Value| with("auxFiles", items);
    let one = |extra: Value| {
        let mut base = json!({ "id": "a", "candidates": ["x.txt"] });
        for (k, val) in extra.as_object().unwrap() {
            base[k] = val.clone();
        }
        base
    };
    assert_manifest_err(&aux(json!([one(json!({})), one(json!({}))])));
    assert_manifest_err(&aux(json!([one(json!({ "maxBytes": 0 }))])));
    assert_manifest_err(&aux(json!([one(json!({ "maxBytes": AUX_MAX_BYTES_CAP + 1 }))])));
    assert_manifest_err(&aux(json!([one(json!({ "candidates": [] }))])));
    PluginManifest::parse(&aux(json!([one(json!({ "maxBytes": AUX_MAX_BYTES_CAP }))]))).unwrap();
}

#[test]
fn api_version_must_be_major_minor() {
    for bad in ["1", "0.1.0", "x.y", ""] {
        assert_manifest_err(&with("apiVersion", json!(bad)));
    }
}

#[test]
fn compat_checks_host_range() {
    let api = v("0.1.0");
    let m = PluginManifest::parse(&with("maxHostVersion", json!("1.5.0"))).unwrap();
    m.check_compat(&v("1.0.0"), &api).unwrap();
    m.check_compat(&v("1.5.0"), &api).unwrap();
    assert!(matches!(m.check_compat(&v("1.6.0"), &api), Err(CoreError::Incompatible(_))));
    assert!(matches!(m.check_compat(&v("0.9.0"), &api), Err(CoreError::Incompatible(_))));
}

#[test]
fn compat_checks_api_version() {
    let host = v("1.0.0");
    let pre = PluginManifest::parse(&with("apiVersion", json!("0.2"))).unwrap();
    assert!(matches!(pre.check_compat(&host, &v("0.1.0")), Err(CoreError::Incompatible(_))));
    assert!(matches!(pre.check_compat(&host, &v("0.3.0")), Err(CoreError::Incompatible(_))));
    pre.check_compat(&host, &v("0.2.5")).unwrap();

    let stable = PluginManifest::parse(&with("apiVersion", json!("1.2"))).unwrap();
    stable.check_compat(&host, &v("1.2.0")).unwrap();
    stable.check_compat(&host, &v("1.3.0")).unwrap();
    assert!(matches!(stable.check_compat(&host, &v("1.1.0")), Err(CoreError::Incompatible(_))));
    assert!(matches!(stable.check_compat(&host, &v("2.0.0")), Err(CoreError::Incompatible(_))));
}

#[test]
fn handles_extension_is_case_insensitive() {
    let m = PluginManifest::parse(VALID).unwrap();
    assert!(m.handles_extension(Path::new("Standard-1.json")));
    assert!(m.handles_extension(Path::new("SAVE.JSON")));
    assert!(!m.handles_extension(Path::new("save.json.bak")));
    assert!(!m.handles_extension(Path::new("noext")));
}

#[test]
fn load_reads_manifest_from_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("manifest.json"), VALID).unwrap();
    assert_eq!(PluginManifest::load(dir.path()).unwrap().id, "planet-crafter");

    let empty = tempfile::tempdir().unwrap();
    assert!(matches!(PluginManifest::load(empty.path()), Err(CoreError::Io { .. })));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p saveforge-core --test manifest`
Expected: compile error, unresolved import `saveforge_core::manifest`.

- [ ] **Step 3: Write `src/manifest.rs`**

```rust
//! `manifest.json` shipped inside every plugin package (spec §4.2).

use std::collections::HashSet;
use std::path::Path;

use semver::Version;
use serde::Deserialize;

use crate::platform::Os;
use crate::{CoreError, Result};

pub const MANIFEST_FILE_NAME: &str = "manifest.json";
pub const DEFAULT_AUX_MAX_BYTES: u64 = 8 * 1024 * 1024;
pub const AUX_MAX_BYTES_CAP: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub api_version: String,
    pub min_host_version: Version,
    #[serde(default)]
    pub max_host_version: Option<Version>,
    pub component: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub file_extensions: Vec<String>,
    #[serde(default)]
    pub default_save_directories: DefaultSaveDirectories,
    #[serde(default)]
    pub aux_files: Vec<AuxFileSpec>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct DefaultSaveDirectories {
    #[serde(default)]
    pub windows: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
    #[serde(default)]
    pub macos: Option<String>,
}

impl DefaultSaveDirectories {
    pub fn for_os(&self, os: Os) -> Option<&str> {
        match os {
            Os::Windows => self.windows.as_deref(),
            Os::Linux => self.linux.as_deref(),
            Os::MacOs => self.macos.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuxFileSpec {
    pub id: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default = "default_aux_max_bytes")]
    pub max_bytes: u64,
    pub candidates: Vec<String>,
}

fn default_aux_max_bytes() -> u64 {
    DEFAULT_AUX_MAX_BYTES
}

impl PluginManifest {
    /// Parses and validates the manifest shape. Host compatibility is checked
    /// separately by [`PluginManifest::check_compat`].
    pub fn parse(json: &str) -> Result<Self> {
        let manifest: Self =
            serde_json::from_str(json).map_err(|e| CoreError::Manifest(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load(plugin_dir: &Path) -> Result<Self> {
        let path = plugin_dir.join(MANIFEST_FILE_NAME);
        let json = std::fs::read_to_string(&path).map_err(|e| CoreError::io(&path, e))?;
        Self::parse(&json)
    }

    pub fn check_compat(&self, host_version: &Version, host_api: &Version) -> Result<()> {
        let api = parse_api_version(&self.api_version)
            .ok_or_else(|| invalid(format!("apiVersion '{}' is not 'major.minor'", self.api_version)))?;
        if !api_compatible(api, host_api) {
            return Err(CoreError::Incompatible(format!(
                "plugin API {} is not supported by host API {}.{}",
                self.api_version, host_api.major, host_api.minor
            )));
        }
        if *host_version < self.min_host_version {
            return Err(CoreError::Incompatible(format!(
                "requires SaveForge {} or newer (running {host_version})",
                self.min_host_version
            )));
        }
        if let Some(max) = &self.max_host_version
            && host_version > max
        {
            return Err(CoreError::Incompatible(format!(
                "supports SaveForge up to {max} (running {host_version})"
            )));
        }
        Ok(())
    }

    pub fn handles_extension(&self, path: &Path) -> bool {
        matches_extension(path, &self.file_extensions)
    }

    fn validate(&self) -> Result<()> {
        if !is_valid_id(&self.id) {
            return Err(invalid(format!("id '{}' must match ^[a-z0-9]+(-[a-z0-9]+)*$", self.id)));
        }
        if parse_api_version(&self.api_version).is_none() {
            return Err(invalid(format!("apiVersion '{}' is not 'major.minor'", self.api_version)));
        }
        if !is_safe_relative_path(&self.component) {
            return Err(invalid(format!("component '{}' is not a safe relative path", self.component)));
        }
        if let Some(icon) = &self.icon
            && !is_safe_relative_path(icon)
        {
            return Err(invalid(format!("icon '{icon}' is not a safe relative path")));
        }
        if self.file_extensions.is_empty() {
            return Err(invalid("fileExtensions must not be empty"));
        }
        if let Some(bad) = self.file_extensions.iter().find(|e| e.len() < 2 || !e.starts_with('.')) {
            return Err(invalid(format!("file extension '{bad}' must start with '.'")));
        }
        let mut seen = HashSet::new();
        for aux in &self.aux_files {
            if !seen.insert(aux.id.as_str()) {
                return Err(invalid(format!("duplicate auxFiles id '{}'", aux.id)));
            }
            if aux.max_bytes == 0 || aux.max_bytes > AUX_MAX_BYTES_CAP {
                return Err(invalid(format!(
                    "auxFiles '{}' maxBytes must be 1..={AUX_MAX_BYTES_CAP}",
                    aux.id
                )));
            }
            if aux.candidates.is_empty() {
                return Err(invalid(format!("auxFiles '{}' has no candidates", aux.id)));
            }
        }
        Ok(())
    }
}

/// `^[a-z0-9]+(-[a-z0-9]+)*$`
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
}

/// Relative path that cannot escape its base directory on any OS.
pub fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && path.split(['/', '\\']).all(|part| part != "..")
}

/// True when `path`'s extension equals one of `extensions` (each written as `.ext`),
/// ignoring ASCII case.
pub fn matches_extension(path: &Path, extensions: &[String]) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    extensions
        .iter()
        .any(|allowed| allowed.strip_prefix('.').is_some_and(|a| a.eq_ignore_ascii_case(ext)))
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Manifest(message.into())
}

fn parse_api_version(s: &str) -> Option<(u64, u64)> {
    let (major, minor) = s.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn api_compatible((major, minor): (u64, u64), host: &Version) -> bool {
    if host.major == 0 {
        major == 0 && minor == host.minor
    } else {
        major == host.major && minor <= host.minor
    }
}
```

- [ ] **Step 4: Register the module in `src/lib.rs`**

```rust
mod error;
pub mod manifest;
pub mod platform;
```

If Task 1 added `#[allow(dead_code)]` on `CoreError::io`, remove it now.

- [ ] **Step 5: Run tests**

Run: `cargo test -p saveforge-core --test manifest`
Expected: all 13 tests pass.

- [ ] **Step 6: Lint gate**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/saveforge-core
git commit -m "feat(core): plugin manifest parsing, validation and compat checks"
```

---

### Task 3: VDF parser + Steam library lookup

**Files:**
- Create: `crates/saveforge-core/src/steam/mod.rs`, `crates/saveforge-core/src/steam/vdf.rs`
- Modify: `crates/saveforge-core/src/lib.rs` (add `pub mod steam;`)
- Test: `crates/saveforge-core/tests/steam.rs`

**Interfaces:**
- Consumes: `Os`.
- Produces:
  - `steam::Vdf { Str(String), Obj(Vec<(String, Vdf)>) }` with `get(&self, key: &str) -> Option<&Vdf>` (key match ignores case) and `as_str(&self) -> Option<&str>`
  - `steam::parse_vdf(&str) -> Option<Vdf>` (root is `Vdf::Obj`)
  - `steam::library_paths_from_vdf(&str) -> Vec<PathBuf>`
  - `steam::SteamLibraries` with `new(Vec<PathBuf>)`, `load(steam_root: &Path) -> Option<Self>`, `libraries() -> &[PathBuf]`, `app_dir(app_id: u32) -> Option<PathBuf>`, `compat_user_dir(app_id: u32) -> Option<PathBuf>`
  - `steam::find_steam_root(os: Os, home: &Path) -> Option<PathBuf>`

- [ ] **Step 1: Write the failing tests `tests/steam.rs`**

```rust
use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::platform::Os;
use saveforge_core::steam::{
    SteamLibraries, Vdf, find_steam_root, library_paths_from_vdf, parse_vdf,
};

fn vdf_escape(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "\\\\")
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Creates `<lib>/steamapps/appmanifest_<id>.acf` and `<lib>/steamapps/common/<dir>/`.
fn install_app(lib: &Path, app_id: u32, install_dir: &str) -> PathBuf {
    write(
        &lib.join("steamapps").join(format!("appmanifest_{app_id}.acf")),
        &format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"installdir\"\t\t\"{install_dir}\"\n}}\n"),
    );
    let dir = lib.join("steamapps").join("common").join(install_dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn parses_nested_vdf_with_escapes_comments_and_conditionals() {
    let text = r#"
        // comment
        "Root"
        {
            "Name"   "a \"quoted\" C:\\path"
            "Child" { "Inner" "1" }
            "Flag"   "1" [$WIN32]
            Unquoted value
        }
    "#;
    let root = parse_vdf(text).unwrap();
    let r = root.get("root").unwrap();
    assert_eq!(r.get("name").and_then(Vdf::as_str), Some("a \"quoted\" C:\\path"));
    assert_eq!(r.get("Child").and_then(|c| c.get("Inner")).and_then(Vdf::as_str), Some("1"));
    assert_eq!(r.get("Flag").and_then(Vdf::as_str), Some("1"));
    assert_eq!(r.get("Unquoted").and_then(Vdf::as_str), Some("value"));
}

#[test]
fn rejects_malformed_vdf() {
    assert!(parse_vdf("\"a\" { \"b\" \"c\"").is_none());
    assert!(parse_vdf("\"a\" }").is_none());
    assert!(parse_vdf("\"unterminated").is_none());
}

#[test]
fn library_paths_new_and_old_format() {
    let new = r#""libraryfolders" {
        "0" { "path" "C:\\Program Files (x86)\\Steam" "apps" { "228980" "1" } }
        "1" { "path" "D:\\SteamLibrary" }
        "contentstatsid" "123"
    }"#;
    assert_eq!(
        library_paths_from_vdf(new),
        vec![PathBuf::from("C:\\Program Files (x86)\\Steam"), PathBuf::from("D:\\SteamLibrary")]
    );
    let old = r#""LibraryFolders" { "TimeNextStatsReport" "1" "1" "/mnt/games/Steam" }"#;
    assert_eq!(library_paths_from_vdf(old), vec![PathBuf::from("/mnt/games/Steam")]);
    assert!(library_paths_from_vdf("garbage {").is_empty());
}

#[test]
fn load_includes_root_and_listed_libraries_without_duplicates() {
    let root = tempfile::tempdir().unwrap();
    let extra = tempfile::tempdir().unwrap();
    write(
        &root.path().join("steamapps/libraryfolders.vdf"),
        &format!(
            "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
            vdf_escape(root.path()),
            vdf_escape(extra.path())
        ),
    );
    let libs = SteamLibraries::load(root.path()).unwrap();
    assert_eq!(libs.libraries(), &[root.path().to_path_buf(), extra.path().to_path_buf()]);
}

#[test]
fn load_without_vdf_uses_root_only_and_missing_root_is_none() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(SteamLibraries::load(root.path()).unwrap().libraries(), &[root.path().to_path_buf()]);
    assert!(SteamLibraries::load(&root.path().join("nope")).is_none());
}

#[test]
fn app_dir_searches_all_libraries() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let installed = install_app(b.path(), 1_284_190, "Planet Crafter");
    let libs = SteamLibraries::new(vec![a.path().to_path_buf(), b.path().to_path_buf()]);
    assert_eq!(libs.app_dir(1_284_190), Some(installed));
    assert_eq!(libs.app_dir(42), None);
}

#[test]
fn app_dir_ignores_manifest_without_install_dir_on_disk() {
    let lib = tempfile::tempdir().unwrap();
    let dir = install_app(lib.path(), 7, "Gone");
    fs::remove_dir(&dir).unwrap();
    assert_eq!(SteamLibraries::new(vec![lib.path().to_path_buf()]).app_dir(7), None);
}

#[test]
fn compat_user_dir_finds_proton_prefix() {
    let lib = tempfile::tempdir().unwrap();
    let user = lib.path().join("steamapps/compatdata/1284190/pfx/drive_c/users/steamuser");
    fs::create_dir_all(&user).unwrap();
    let libs = SteamLibraries::new(vec![lib.path().to_path_buf()]);
    assert_eq!(libs.compat_user_dir(1_284_190), Some(user));
    assert_eq!(libs.compat_user_dir(1), None);
}

#[test]
fn find_steam_root_linux_prefers_dot_steam() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(find_steam_root(Os::Linux, home.path()), None);
    let share = home.path().join(".local/share/Steam");
    fs::create_dir_all(&share).unwrap();
    assert_eq!(find_steam_root(Os::Linux, home.path()), Some(share));
    let dot = home.path().join(".steam/steam");
    fs::create_dir_all(&dot).unwrap();
    assert_eq!(find_steam_root(Os::Linux, home.path()), Some(dot));
}

#[test]
fn find_steam_root_macos() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("Library/Application Support/Steam");
    fs::create_dir_all(&root).unwrap();
    assert_eq!(find_steam_root(Os::MacOs, home.path()), Some(root));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p saveforge-core --test steam`
Expected: compile error, unresolved import `saveforge_core::steam`.

- [ ] **Step 3: Write `src/steam/vdf.rs`**

```rust
//! Minimal parser for Valve's text `KeyValues` format (`.vdf` / `.acf`).

/// A `KeyValues` node. Objects keep insertion order and duplicate keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    /// First child whose key matches ignoring ASCII case.
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Self::Obj(entries) => entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Self::Str(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            Self::Obj(_) => None,
        }
    }
}

/// Parses a document into a root `Vdf::Obj`. Returns `None` when malformed.
pub fn parse(input: &str) -> Option<Vdf> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let pairs = parse_pairs(&tokens, &mut pos, false)?;
    Some(Vdf::Obj(pairs))
}

enum Tok {
    Str(String),
    Open,
    Close,
}

fn tokenize(input: &str) -> Option<Vec<Tok>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '{' => tokens.push(Tok::Open),
            '}' => tokens.push(Tok::Close),
            // Platform conditionals like [$WIN32] are ignored.
            '[' => {
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            other => s.push(other),
                        },
                        other => s.push(other),
                    }
                }
                tokens.push(Tok::Str(s));
            }
            other => {
                let mut s = String::from(other);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || matches!(next, '{' | '}' | '"') {
                        break;
                    }
                    s.push(next);
                    chars.next();
                }
                tokens.push(Tok::Str(s));
            }
        }
    }
    Some(tokens)
}

fn parse_pairs(tokens: &[Tok], pos: &mut usize, nested: bool) -> Option<Vec<(String, Vdf)>> {
    let mut pairs = Vec::new();
    loop {
        match tokens.get(*pos) {
            None => return (!nested).then_some(pairs),
            Some(Tok::Close) => {
                *pos += 1;
                return nested.then_some(pairs);
            }
            Some(Tok::Open) => return None,
            Some(Tok::Str(key)) => {
                *pos += 1;
                let value = match tokens.get(*pos)? {
                    Tok::Str(v) => {
                        *pos += 1;
                        Vdf::Str(v.clone())
                    }
                    Tok::Open => {
                        *pos += 1;
                        Vdf::Obj(parse_pairs(tokens, pos, true)?)
                    }
                    Tok::Close => return None,
                };
                pairs.push((key.clone(), value));
            }
        }
    }
}
```

- [ ] **Step 4: Write `src/steam/mod.rs`**

```rust
//! Steam library discovery used by the `{steam_app:ID}` and `{steam_compat:ID}`
//! path tokens (spec §4.3).

mod vdf;

use std::fs;
use std::path::{Path, PathBuf};

pub use vdf::{Vdf, parse as parse_vdf};

use crate::platform::Os;

/// All Steam library folders on this machine, Steam root first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamLibraries {
    libraries: Vec<PathBuf>,
}

impl SteamLibraries {
    pub fn new(libraries: Vec<PathBuf>) -> Self {
        Self { libraries }
    }

    /// Reads `<root>/steamapps/libraryfolders.vdf`. Returns `None` if `steam_root`
    /// is not a directory. A missing or broken VDF yields just the root.
    pub fn load(steam_root: &Path) -> Option<Self> {
        if !steam_root.is_dir() {
            return None;
        }
        let mut libraries = vec![steam_root.to_path_buf()];
        let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(&vdf_path) {
            for path in library_paths_from_vdf(&text) {
                if !libraries.iter().any(|known| same_path(known, &path)) {
                    libraries.push(path);
                }
            }
        }
        Some(Self { libraries })
    }

    pub fn libraries(&self) -> &[PathBuf] {
        &self.libraries
    }

    /// `<lib>/steamapps/common/<installdir>` of the first library whose
    /// `appmanifest_<id>.acf` names an install dir that exists.
    pub fn app_dir(&self, app_id: u32) -> Option<PathBuf> {
        self.libraries.iter().find_map(|lib| {
            let steamapps = lib.join("steamapps");
            let text = fs::read_to_string(steamapps.join(format!("appmanifest_{app_id}.acf"))).ok()?;
            let root = vdf::parse(&text)?;
            let install_dir = root.get("AppState")?.get("installdir")?.as_str()?;
            let dir = steamapps.join("common").join(install_dir);
            dir.is_dir().then_some(dir)
        })
    }

    /// Proton prefix user dir: `<lib>/steamapps/compatdata/<id>/pfx/drive_c/users/steamuser`.
    pub fn compat_user_dir(&self, app_id: u32) -> Option<PathBuf> {
        self.libraries
            .iter()
            .map(|lib| {
                lib.join("steamapps")
                    .join("compatdata")
                    .join(app_id.to_string())
                    .join("pfx")
                    .join("drive_c")
                    .join("users")
                    .join("steamuser")
            })
            .find(|p| p.is_dir())
    }
}

/// Library paths listed in `libraryfolders.vdf`, in file order. Handles both
/// the current format (`"0" { "path" "..." }`) and the legacy one (`"1" "..."`).
pub fn library_paths_from_vdf(text: &str) -> Vec<PathBuf> {
    let Some(root) = vdf::parse(text) else {
        return Vec::new();
    };
    let Some(Vdf::Obj(entries)) = root.get("libraryfolders") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|(key, _)| !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|(_, value)| match value {
            Vdf::Str(path) => Some(PathBuf::from(path)),
            Vdf::Obj(_) => value.get("path").and_then(Vdf::as_str).map(PathBuf::from),
        })
        .collect()
}

/// First existing Steam root for `os`.
pub fn find_steam_root(os: Os, home: &Path) -> Option<PathBuf> {
    root_candidates(os, home).into_iter().find(|p| p.is_dir())
}

fn root_candidates(os: Os, home: &Path) -> Vec<PathBuf> {
    match os {
        Os::Windows => {
            let mut candidates = Vec::new();
            if let Some(path) = registry_steam_path() {
                candidates.push(path);
            }
            if let Some(pf) = std::env::var_os("ProgramFiles(x86)") {
                candidates.push(PathBuf::from(pf).join("Steam"));
            }
            candidates
        }
        Os::Linux => vec![home.join(".steam").join("steam"), home.join(".local/share/Steam")],
        Os::MacOs => vec![home.join("Library/Application Support/Steam")],
    }
}

#[cfg(windows)]
fn registry_steam_path() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam").ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(not(windows))]
fn registry_steam_path() -> Option<PathBuf> {
    None
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        p.to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase()
    };
    norm(a) == norm(b)
}
```

- [ ] **Step 5: Register the module in `src/lib.rs`**

```rust
mod error;
pub mod manifest;
pub mod platform;
pub mod steam;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p saveforge-core --test steam`
Expected: all 10 tests pass.

- [ ] **Step 7: Lint gate**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/saveforge-core
git commit -m "feat(core): Steam library discovery with VDF parser"
```

---

### Task 4: Path token expansion

**Files:**
- Create: `crates/saveforge-core/src/tokens.rs`
- Modify: `crates/saveforge-core/src/lib.rs` (add `pub mod tokens;`)
- Test: `crates/saveforge-core/tests/tokens.rs`

**Interfaces:**
- Consumes: `Os`, `SteamLibraries::{load, app_dir, compat_user_dir}`, `find_steam_root`.
- Produces:
  - `tokens::TokenEnv { os: Os, appdata: Option<PathBuf>, local_appdata: Option<PathBuf>, home: Option<PathBuf>, temp: PathBuf, steam: Option<SteamLibraries> }` with `TokenEnv::detect() -> Self`
  - `tokens::Expanded { Resolved(PathBuf), Unresolved(String) }`
  - `tokens::expand(raw: &str, env: &TokenEnv) -> Expanded`

- [ ] **Step 1: Write the failing tests `tests/tokens.rs`**

```rust
use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::platform::Os;
use saveforge_core::steam::SteamLibraries;
use saveforge_core::tokens::{Expanded, TokenEnv, expand};

fn env(os: Os, steam: Option<SteamLibraries>) -> TokenEnv {
    TokenEnv {
        os,
        appdata: Some(PathBuf::from("/A")),
        local_appdata: Some(PathBuf::from("/L")),
        home: Some(PathBuf::from("/H")),
        temp: PathBuf::from("/T"),
        steam,
    }
}

fn resolved(s: &str) -> Expanded {
    Expanded::Resolved(PathBuf::from(s))
}

fn is_unresolved(e: &Expanded) -> bool {
    matches!(e, Expanded::Unresolved(_))
}

fn steam_with_app(lib: &Path, app_id: u32, install_dir: &str) -> (SteamLibraries, PathBuf) {
    let steamapps = lib.join("steamapps");
    fs::create_dir_all(steamapps.join("common").join(install_dir)).unwrap();
    fs::write(
        steamapps.join(format!("appmanifest_{app_id}.acf")),
        format!("\"AppState\" {{ \"installdir\" \"{install_dir}\" }}"),
    )
    .unwrap();
    (
        SteamLibraries::new(vec![lib.to_path_buf()]),
        steamapps.join("common").join(install_dir),
    )
}

#[test]
fn expands_known_percent_tokens() {
    let e = env(Os::Windows, None);
    assert_eq!(expand("%APPDATA%/MijuGames/Saves", &e), resolved("/A/MijuGames/Saves"));
    assert_eq!(expand("%LOCALAPPDATA%/x", &e), resolved("/L/x"));
    assert_eq!(expand("%USERPROFILE%/x", &e), resolved("/H/x"));
    assert_eq!(expand("%TEMP%/x", &e), resolved("/T/x"));
}

#[test]
fn percent_tokens_are_case_insensitive() {
    let e = env(Os::Windows, None);
    assert_eq!(expand("%appdata%/x", &e), resolved("/A/x"));
    assert_eq!(expand("%AppData%/x", &e), resolved("/A/x"));
}

#[test]
fn leading_tilde_expands_to_home_only_as_whole_segment() {
    let e = env(Os::Linux, None);
    assert_eq!(expand("~", &e), resolved("/H"));
    assert_eq!(expand("~/saves", &e), resolved("/H/saves"));
    assert_eq!(expand("~\\saves", &e), resolved("/H\\saves"));
    assert_eq!(expand("~user/saves", &e), resolved("~user/saves"));
    assert_eq!(expand("a/~/b", &e), resolved("a/~/b"));
}

#[test]
fn unknown_tokens_are_left_verbatim_and_later_tokens_still_expand() {
    let e = env(Os::Windows, None);
    // "%FOO%/" kept verbatim, then appdata "/A", then "/x".
    assert_eq!(expand("%FOO%/%APPDATA%/x", &e), resolved("%FOO%//A/x"));
    assert_eq!(expand("{foo}/x", &e), resolved("{foo}/x"));
    assert_eq!(expand("{steam_app:abc}/x", &e), resolved("{steam_app:abc}/x"));
    assert_eq!(expand("100% sure", &e), resolved("100% sure"));
    assert_eq!(expand("%APPDATA", &e), resolved("%APPDATA"));
}

#[test]
fn trims_and_rejects_empty() {
    let e = env(Os::Windows, None);
    assert_eq!(expand("  %APPDATA%/x  ", &e), resolved("/A/x"));
    assert!(is_unresolved(&expand("", &e)));
    assert!(is_unresolved(&expand("   ", &e)));
}

#[test]
fn missing_base_dir_makes_path_unresolved() {
    let mut e = env(Os::Windows, None);
    e.appdata = None;
    e.home = None;
    assert!(is_unresolved(&expand("%APPDATA%/x", &e)));
    assert!(is_unresolved(&expand("~/x", &e)));
    assert_eq!(expand("%TEMP%/x", &e), resolved("/T/x"));
}

#[test]
fn steam_app_token() {
    let lib = tempfile::tempdir().unwrap();
    let (steam, dir) = steam_with_app(lib.path(), 1_284_190, "Planet Crafter");
    let e = env(Os::Windows, Some(steam));
    assert_eq!(
        expand("{steam_app:1284190}/Data/loc.txt", &e),
        Expanded::Resolved(PathBuf::from(format!("{}/Data/loc.txt", dir.display())))
    );
    assert!(is_unresolved(&expand("{steam_app:42}/x", &e)));
    assert!(is_unresolved(&expand("{steam_app:1284190}/x", &env(Os::Windows, None))));
}

#[test]
fn steam_compat_token_only_on_linux() {
    let lib = tempfile::tempdir().unwrap();
    let user = lib.path().join("steamapps/compatdata/1284190/pfx/drive_c/users/steamuser");
    fs::create_dir_all(&user).unwrap();
    let steam = SteamLibraries::new(vec![lib.path().to_path_buf()]);

    let linux = env(Os::Linux, Some(steam.clone()));
    assert_eq!(
        expand("{steam_compat:1284190}/AppData", &linux),
        Expanded::Resolved(PathBuf::from(format!("{}/AppData", user.display())))
    );
    assert!(is_unresolved(&expand("{steam_compat:1284190}/AppData", &env(Os::Windows, Some(steam)))));
}

#[test]
fn detect_produces_usable_env() {
    let e = TokenEnv::detect();
    assert_eq!(e.os, Os::current());
    assert!(e.home.is_some());
    assert!(matches!(expand("%TEMP%", &e), Expanded::Resolved(_)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p saveforge-core --test tokens`
Expected: compile error, unresolved import `saveforge_core::tokens`.

- [ ] **Step 3: Write `src/tokens.rs`**

```rust
//! Path token expansion (spec §4.3).
//!
//! Supported: `%APPDATA%`, `%LOCALAPPDATA%`, `%USERPROFILE%`, `%TEMP%` (case-insensitive),
//! a leading `~`, `{steam_app:ID}` and `{steam_compat:ID}` (Linux only). Unknown tokens
//! are left verbatim; a known token that cannot be resolved makes the whole path
//! unresolved.

use std::path::PathBuf;

use crate::platform::Os;
use crate::steam::{SteamLibraries, find_steam_root};

/// Machine facts used for expansion. Build with [`TokenEnv::detect`] or by hand in tests.
#[derive(Debug, Clone)]
pub struct TokenEnv {
    pub os: Os,
    pub appdata: Option<PathBuf>,
    pub local_appdata: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub temp: PathBuf,
    pub steam: Option<SteamLibraries>,
}

impl TokenEnv {
    pub fn detect() -> Self {
        let os = Os::current();
        let base = directories::BaseDirs::new();
        let home = base.as_ref().map(|b| b.home_dir().to_path_buf());
        let steam = home
            .as_deref()
            .and_then(|h| find_steam_root(os, h))
            .and_then(|root| SteamLibraries::load(&root));
        Self {
            os,
            appdata: base.as_ref().map(|b| b.config_dir().to_path_buf()),
            local_appdata: base.as_ref().map(|b| b.data_local_dir().to_path_buf()),
            home,
            temp: std::env::temp_dir(),
            steam,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expanded {
    Resolved(PathBuf),
    /// Human-readable reason, shown in the "path not set" state.
    Unresolved(String),
}

enum Token {
    Value(PathBuf),
    Unknown,
    Missing(String),
}

pub fn expand(raw: &str, env: &TokenEnv) -> Expanded {
    let raw = raw.trim();
    if raw.is_empty() {
        return Expanded::Unresolved("path is empty".to_owned());
    }

    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;

    if let Some(after) = rest.strip_prefix('~')
        && (after.is_empty() || after.starts_with(['/', '\\']))
    {
        let Some(home) = &env.home else {
            return Expanded::Unresolved("home directory is unknown".to_owned());
        };
        out.push_str(&home.to_string_lossy());
        rest = after;
    }

    while let Some(i) = rest.find(['%', '{']) {
        out.push_str(&rest[..i]);
        let percent = rest[i..].starts_with('%');
        let close = if percent { '%' } else { '}' };
        let tail = &rest[i + 1..];
        let token = tail.find(close).map(|j| {
            let name = &tail[..j];
            let token = if percent { resolve_percent(name, env) } else { resolve_brace(name, env) };
            (token, &tail[j + 1..])
        });
        match token {
            Some((Token::Value(path), after)) => {
                out.push_str(&path.to_string_lossy());
                rest = after;
            }
            Some((Token::Missing(reason), _)) => return Expanded::Unresolved(reason),
            Some((Token::Unknown, _)) | None => {
                // Keep the opening char and keep scanning right after it, so
                // "%FOO%/%APPDATA%" still expands the second token.
                out.push(if percent { '%' } else { '{' });
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    Expanded::Resolved(PathBuf::from(out))
}

fn resolve_percent(name: &str, env: &TokenEnv) -> Token {
    let dir = match name.to_ascii_uppercase().as_str() {
        "APPDATA" => env.appdata.as_ref(),
        "LOCALAPPDATA" => env.local_appdata.as_ref(),
        "USERPROFILE" => env.home.as_ref(),
        "TEMP" => Some(&env.temp),
        _ => return Token::Unknown,
    };
    dir.map_or_else(
        || Token::Missing(format!("%{name}% is not available on this system")),
        |d| Token::Value(d.clone()),
    )
}

fn resolve_brace(name: &str, env: &TokenEnv) -> Token {
    let Some((kind, id)) = name.split_once(':') else {
        return Token::Unknown;
    };
    let Ok(app_id) = id.parse::<u32>() else {
        return Token::Unknown;
    };
    match kind {
        "steam_app" => env
            .steam
            .as_ref()
            .and_then(|s| s.app_dir(app_id))
            .map_or_else(
                || Token::Missing(format!("Steam app {app_id} is not installed")),
                Token::Value,
            ),
        "steam_compat" if env.os != Os::Linux => {
            Token::Missing("Proton prefixes only exist on Linux".to_owned())
        }
        "steam_compat" => env
            .steam
            .as_ref()
            .and_then(|s| s.compat_user_dir(app_id))
            .map_or_else(
                || Token::Missing(format!("no Proton prefix for Steam app {app_id}")),
                Token::Value,
            ),
        _ => Token::Unknown,
    }
}
```

- [ ] **Step 4: Register the module in `src/lib.rs`**

```rust
mod error;
pub mod manifest;
pub mod platform;
pub mod steam;
pub mod tokens;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p saveforge-core --test tokens`
Expected: all 9 tests pass.

- [ ] **Step 6: Lint gate**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/saveforge-core
git commit -m "feat(core): path token expansion incl. Steam tokens"
```

---

### Task 5: Save discovery

**Files:**
- Create: `crates/saveforge-core/src/discovery.rs`
- Modify: `crates/saveforge-core/src/lib.rs` (add `pub mod discovery;`)
- Test: `crates/saveforge-core/tests/discovery.rs`

**Interfaces:**
- Consumes: `manifest::matches_extension(&Path, &[String]) -> bool`.
- Produces: `discovery::SaveFileInfo { path: PathBuf, file_name: String, modified: SystemTime, size_bytes: u64 }` and `discovery::discover(dir: &Path, extensions: &[String]) -> Vec<SaveFileInfo>`. Results are sorted newest first, with ties broken by path ascending. The function never fails.

- [ ] **Step 1: Write the failing tests `tests/discovery.rs`**

```rust
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use filetime::{FileTime, set_file_mtime};
use saveforge_core::discovery::discover;

fn json() -> Vec<String> {
    vec![".json".to_owned()]
}

fn touch(path: &Path, bytes: &[u8], secs_after_epoch: u64) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs_after_epoch);
    set_file_mtime(path, FileTime::from_system_time(t)).unwrap();
}

#[test]
fn finds_matching_files_recursively_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("old.json"), b"1", 1_000);
    touch(&dir.path().join("nested/deeper/new.json"), b"22", 3_000);
    touch(&dir.path().join("mid.json"), b"333", 2_000);
    touch(&dir.path().join("ignored.txt"), b"x", 4_000);

    let found = discover(dir.path(), &json());
    let names: Vec<_> = found.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, ["new.json", "mid.json", "old.json"]);
    assert_eq!(found[0].path, dir.path().join("nested/deeper/new.json"));
    assert_eq!(found[0].size_bytes, 2);
    assert_eq!(found[0].modified, SystemTime::UNIX_EPOCH + Duration::from_secs(3_000));
}

#[test]
fn extension_filter_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("SAVE.JSON"), b"{}", 1);
    touch(&dir.path().join("save.json.bak"), b"{}", 2);
    let found = discover(dir.path(), &json());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].file_name, "SAVE.JSON");
}

#[test]
fn equal_mtimes_sort_by_path() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("b.json"), b"", 5);
    touch(&dir.path().join("a.json"), b"", 5);
    let names: Vec<_> = discover(dir.path(), &json()).into_iter().map(|f| f.file_name).collect();
    assert_eq!(names, ["a.json", "b.json"]);
}

#[test]
fn missing_dir_or_file_path_yields_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert!(discover(&dir.path().join("nope"), &json()).is_empty());
    touch(&dir.path().join("f.json"), b"", 1);
    assert!(discover(&dir.path().join("f.json"), &json()).is_empty());
}

#[test]
fn multiple_extensions() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("a.sav"), b"", 1);
    touch(&dir.path().join("b.json"), b"", 2);
    let found = discover(dir.path(), &[".sav".to_owned(), ".json".to_owned()]);
    assert_eq!(found.len(), 2);
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlinked_dirs() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("sub/a.json"), b"", 1);
    std::os::unix::fs::symlink(dir.path(), dir.path().join("sub/loop")).unwrap();
    assert_eq!(discover(dir.path(), &json()).len(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p saveforge-core --test discovery`
Expected: compile error, unresolved import `saveforge_core::discovery`.

- [ ] **Step 3: Write `src/discovery.rs`**

```rust
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
    found.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.path.cmp(&b.path)));
    found
}
```

- [ ] **Step 4: Register the module in `src/lib.rs`**

```rust
pub mod discovery;
mod error;
pub mod manifest;
pub mod platform;
pub mod steam;
pub mod tokens;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p saveforge-core --test discovery`
Expected: all tests pass (5 on Windows, 6 on Unix).

- [ ] **Step 6: Lint gate**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/saveforge-core
git commit -m "feat(core): recursive save discovery"
```

---

### Task 6: Backup + atomic write

**Files:**
- Create: `crates/saveforge-core/src/backup.rs`
- Modify: `crates/saveforge-core/src/lib.rs` (add `pub mod backup;`)
- Test: `crates/saveforge-core/tests/backup.rs`

**Interfaces:**
- Consumes: `CoreError::io`.
- Produces:
  - `backup::save_with_backup(target: &Path, bytes: &[u8], backup_root: &Path, plugin_id: &str) -> Result<PathBuf>` returns the backup file path.
  - `backup::save_with_backup_at(target, bytes, backup_root, plugin_id, now: time::OffsetDateTime, rename: impl FnOnce(&Path, &Path) -> std::io::Result<()>) -> Result<PathBuf>` is the injectable variant, used by tests.
  - `backup::write_atomic(target: &Path, bytes: &[u8]) -> Result<()>`
  - const `backup::TMP_SUFFIX = ".saveforge.tmp"`

- [ ] **Step 1: Write the failing tests `tests/backup.rs`**

```rust
use std::fs;
use std::io;
use std::path::Path;

use saveforge_core::CoreError;
use saveforge_core::backup::{TMP_SUFFIX, save_with_backup, save_with_backup_at, write_atomic};
use time::macros::datetime;

fn real_rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[test]
fn backs_up_original_then_replaces_content() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("Standard-1.json");
    fs::write(&target, b"old").unwrap();
    let backups = dir.path().join("backups");

    let backup = save_with_backup_at(
        &target,
        b"new",
        &backups,
        "planet-crafter",
        datetime!(2026-10-10 12:30:45 UTC),
        real_rename,
    )
    .unwrap();

    assert_eq!(
        backup,
        backups.join("planet-crafter").join("Standard-1.json.2026-10-10T12-30-45Z.bak")
    );
    assert_eq!(fs::read(&backup).unwrap(), b"old");
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert!(!dir.path().join(format!("Standard-1.json{TMP_SUFFIX}")).exists());
}

#[test]
fn second_backup_in_same_second_gets_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("s.json");
    fs::write(&target, b"v1").unwrap();
    let now = datetime!(2026-10-10 12:30:45 UTC);
    let first = save_with_backup_at(&target, b"v2", dir.path(), "p", now, real_rename).unwrap();
    let second = save_with_backup_at(&target, b"v3", dir.path(), "p", now, real_rename).unwrap();
    let third = save_with_backup_at(&target, b"v4", dir.path(), "p", now, real_rename).unwrap();

    assert_eq!(second.file_name().unwrap(), "s.json.2026-10-10T12-30-45Z-1.bak");
    assert_eq!(third.file_name().unwrap(), "s.json.2026-10-10T12-30-45Z-2.bak");
    assert_eq!(fs::read(first).unwrap(), b"v1");
    assert_eq!(fs::read(second).unwrap(), b"v2");
    assert_eq!(fs::read(&target).unwrap(), b"v4");
}

#[test]
fn failed_rename_leaves_original_untouched_and_removes_tmp() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("s.json");
    fs::write(&target, b"original").unwrap();

    let result = save_with_backup_at(
        &target,
        b"new",
        &dir.path().join("b"),
        "p",
        datetime!(2026-01-01 00:00:00 UTC),
        |_, _| Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked")),
    );

    assert!(matches!(result, Err(CoreError::Io { .. })));
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert!(!dir.path().join(format!("s.json{TMP_SUFFIX}")).exists());
}

#[test]
fn missing_target_fails_before_writing_anything() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("missing.json");
    let result = save_with_backup(&target, b"x", &dir.path().join("b"), "p");
    assert!(matches!(result, Err(CoreError::Io { .. })));
    assert!(!target.exists());
    assert!(!dir.path().join(format!("missing.json{TMP_SUFFIX}")).exists());
}

#[test]
fn write_atomic_creates_and_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("settings.json");
    write_atomic(&target, b"a").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"a");
    write_atomic(&target, b"bb").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"bb");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p saveforge-core --test backup`
Expected: compile error, unresolved import `saveforge_core::backup`.

- [ ] **Step 3: Write `src/backup.rs`**

```rust
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
    save_with_backup_at(target, bytes, backup_root, plugin_id, OffsetDateTime::now_utc(), |from, to| {
        fs::rename(from, to)
    })
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
        .format(format_description!("[year]-[month]-[day]T[hour]-[minute]-[second]Z"))
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
            CoreError::io(path, io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))
        })
}
```

- [ ] **Step 4: Register the module in `src/lib.rs`**

```rust
pub mod backup;
pub mod discovery;
mod error;
pub mod manifest;
pub mod platform;
pub mod steam;
pub mod tokens;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p saveforge-core --test backup`
Expected: all 5 tests pass.

- [ ] **Step 6: Lint gate**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/saveforge-core
git commit -m "feat(core): backup and atomic save write"
```

---

### Task 7: App paths + settings

**Files:**
- Create: `crates/saveforge-core/src/paths.rs`, `crates/saveforge-core/src/settings.rs`
- Modify: `crates/saveforge-core/src/lib.rs` (add `pub mod paths; pub mod settings;`)
- Test: `crates/saveforge-core/tests/settings.rs`

**Interfaces:**
- Consumes: `backup::write_atomic`, `CoreError::{io, Json}`.
- Produces:
  - `paths::AppPaths { data_dir: PathBuf }` with `detect() -> Option<Self>`, `under(impl Into<PathBuf>) -> Self`, `settings_file()`, `catalog_cache_file()`, `logs_dir()`, `crash_log()` (all `-> PathBuf`)
  - `settings::AppSettings { catalog_url: String, plugin_directory: PathBuf, backup_directory: PathBuf, check_updates_on_startup: bool, plugin_save_directories: BTreeMap<String, String> }` with `defaults(&AppPaths) -> Self`, `load(path: &Path, defaults: AppSettings) -> LoadedSettings`, `save(&self, path: &Path) -> Result<()>`
  - `settings::LoadedSettings { settings: AppSettings, warning: Option<String> }`
  - `settings::RELEASE_CATALOG_URL`, `settings::file_url(&Path) -> String`

- [ ] **Step 1: Write the failing tests `tests/settings.rs`**

```rust
use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::paths::AppPaths;
use saveforge_core::settings::{AppSettings, RELEASE_CATALOG_URL, file_url};

fn defaults(dir: &Path) -> AppSettings {
    AppSettings::defaults(&AppPaths::under(dir))
}

#[test]
fn app_paths_layout() {
    let p = AppPaths::under("/data");
    assert_eq!(p.settings_file(), PathBuf::from("/data/settings.json"));
    assert_eq!(p.catalog_cache_file(), PathBuf::from("/data/catalog-cache.json"));
    assert_eq!(p.logs_dir(), PathBuf::from("/data/logs"));
    assert_eq!(p.crash_log(), PathBuf::from("/data/crash.log"));
    assert!(AppPaths::detect().is_some());
}

#[test]
fn defaults_match_spec() {
    let d = defaults(Path::new("/data"));
    assert_eq!(d.backup_directory, PathBuf::from("/data/backups"));
    assert!(d.check_updates_on_startup);
    assert!(d.plugin_save_directories.is_empty());
    if cfg!(debug_assertions) {
        assert!(d.plugin_directory.ends_with("target/dev-plugins"));
        assert_eq!(d.catalog_url, file_url(Path::new("/data/catalog.json")));
    } else {
        assert_eq!(d.plugin_directory, PathBuf::from("/data/plugins"));
        assert_eq!(d.catalog_url, RELEASE_CATALOG_URL);
    }
}

#[test]
fn missing_file_loads_defaults_without_warning() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = AppSettings::load(&dir.path().join("settings.json"), defaults(dir.path()));
    assert_eq!(loaded.settings, defaults(dir.path()));
    assert_eq!(loaded.warning, None);
}

#[test]
fn corrupt_file_loads_defaults_with_warning() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    let loaded = AppSettings::load(&path, defaults(dir.path()));
    assert_eq!(loaded.settings, defaults(dir.path()));
    assert!(loaded.warning.is_some());
}

#[test]
fn partial_file_merges_over_defaults_and_ignores_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{ "checkUpdatesOnStartup": false,
             "pluginSaveDirectories": { "planet-crafter": "%APPDATA%/X" },
             "someFutureKey": 1 }"#,
    )
    .unwrap();
    let loaded = AppSettings::load(&path, defaults(dir.path()));
    assert_eq!(loaded.warning, None);
    assert!(!loaded.settings.check_updates_on_startup);
    assert_eq!(loaded.settings.plugin_save_directories["planet-crafter"], "%APPDATA%/X");
    assert_eq!(loaded.settings.backup_directory, defaults(dir.path()).backup_directory);
}

#[test]
fn save_then_load_round_trips_with_camel_case_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut s = defaults(dir.path());
    s.catalog_url = "https://example.com/catalog.json".to_owned();
    s.plugin_save_directories.insert("planet-crafter".to_owned(), "~/saves".to_owned());
    s.save(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    for key in ["catalogUrl", "pluginDirectory", "backupDirectory", "checkUpdatesOnStartup", "pluginSaveDirectories"] {
        assert!(text.contains(key), "missing {key} in {text}");
    }
    assert_eq!(AppSettings::load(&path, defaults(Path::new("/other"))).settings, s);
}

#[test]
fn save_creates_missing_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a/b/settings.json");
    defaults(dir.path()).save(&path).unwrap();
    assert!(path.is_file());
}

#[test]
fn file_url_formats() {
    assert_eq!(file_url(Path::new("/home/u/catalog.json")), "file:///home/u/catalog.json");
    assert_eq!(file_url(Path::new(r"C:\Users\u\catalog.json")), "file:///C:/Users/u/catalog.json");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p saveforge-core --test settings`
Expected: compile error, unresolved imports `saveforge_core::paths`, `saveforge_core::settings`.

- [ ] **Step 3: Write `src/paths.rs`**

```rust
//! Layout of the app data directory.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

impl AppPaths {
    /// Platform data dir for `SaveForge`, or `None` if no home directory is known.
    pub fn detect() -> Option<Self> {
        directories::ProjectDirs::from("", "", "SaveForge").map(|d| Self {
            data_dir: d.data_dir().to_path_buf(),
        })
    }

    pub fn under(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    pub fn catalog_cache_file(&self) -> PathBuf {
        self.data_dir.join("catalog-cache.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn crash_log(&self) -> PathBuf {
        self.data_dir.join("crash.log")
    }
}
```

- [ ] **Step 4: Write `src/settings.rs`**

```rust
//! `settings.json` (spec §5.3).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backup::write_atomic;
use crate::paths::AppPaths;
use crate::{CoreError, Result};

pub const RELEASE_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/YOUR_ORG/YOUR_REPO/main/catalog.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub catalog_url: String,
    pub plugin_directory: PathBuf,
    pub backup_directory: PathBuf,
    pub check_updates_on_startup: bool,
    /// Plugin id -> raw (unexpanded) save directory chosen by the user.
    pub plugin_save_directories: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct LoadedSettings {
    pub settings: AppSettings,
    /// Set when the file existed but could not be used; shown in the status bar.
    pub warning: Option<String>,
}

/// On-disk shape: every key optional so older/partial files merge over defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StoredSettings {
    catalog_url: Option<String>,
    plugin_directory: Option<PathBuf>,
    backup_directory: Option<PathBuf>,
    check_updates_on_startup: Option<bool>,
    plugin_save_directories: Option<BTreeMap<String, String>>,
}

impl StoredSettings {
    fn apply_to(self, d: AppSettings) -> AppSettings {
        AppSettings {
            catalog_url: self.catalog_url.unwrap_or(d.catalog_url),
            plugin_directory: self.plugin_directory.unwrap_or(d.plugin_directory),
            backup_directory: self.backup_directory.unwrap_or(d.backup_directory),
            check_updates_on_startup: self
                .check_updates_on_startup
                .unwrap_or(d.check_updates_on_startup),
            plugin_save_directories: self
                .plugin_save_directories
                .unwrap_or(d.plugin_save_directories),
        }
    }
}

impl AppSettings {
    pub fn defaults(paths: &AppPaths) -> Self {
        Self {
            catalog_url: default_catalog_url(paths),
            plugin_directory: default_plugin_directory(paths),
            backup_directory: paths.data_dir.join("backups"),
            check_updates_on_startup: true,
            plugin_save_directories: BTreeMap::new(),
        }
    }

    /// Never fails: missing file means defaults; unreadable or invalid file means
    /// defaults plus a warning.
    pub fn load(path: &Path, defaults: AppSettings) -> LoadedSettings {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return LoadedSettings { settings: defaults, warning: None };
            }
            Err(e) => {
                return LoadedSettings {
                    settings: defaults,
                    warning: Some(format!("Could not read settings ({e}); using defaults.")),
                };
            }
        };
        match serde_json::from_str::<StoredSettings>(&text) {
            Ok(stored) => LoadedSettings { settings: stored.apply_to(defaults), warning: None },
            Err(e) => LoadedSettings {
                settings: defaults,
                warning: Some(format!("Settings file is invalid ({e}); using defaults.")),
            },
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(|source| CoreError::Json {
            path: path.to_path_buf(),
            source,
        })?;
        write_atomic(path, &json)
    }
}

/// `file://` URL for a local path (forward slashes, three slashes before a drive letter).
pub fn file_url(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

fn default_catalog_url(paths: &AppPaths) -> String {
    if cfg!(debug_assertions) {
        file_url(&paths.data_dir.join("catalog.json"))
    } else {
        RELEASE_CATALOG_URL.to_owned()
    }
}

fn default_plugin_directory(paths: &AppPaths) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dev-plugins")
    } else {
        paths.data_dir.join("plugins")
    }
}
```

Note: `defaults_match_spec` checks `ends_with("target/dev-plugins")`. `Path::ends_with` compares whole components, and `..` components come earlier in the path, so the check holds on every OS.

- [ ] **Step 5: Register the modules in `src/lib.rs`**

```rust
pub mod backup;
pub mod discovery;
mod error;
pub mod manifest;
pub mod paths;
pub mod platform;
pub mod settings;
pub mod steam;
pub mod tokens;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p saveforge-core --test settings`
Expected: all 8 tests pass.

- [ ] **Step 7: Lint gate**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/saveforge-core
git commit -m "feat(core): app paths and settings persistence"
```

---

### Task 8: Full gate, supply-chain checks, roadmap update

**Files:**
- Modify: `docs/superpowers/plans/2026-10-10-roadmap.md` (M1 status)

**Interfaces:**
- Consumes: everything above.
- Produces: a green branch ready to merge.

- [ ] **Step 1: Install the supply-chain tools (once per machine)**

Run: `cargo install cargo-deny --locked && cargo install cargo-machete --locked`
Expected: both install. Check with `cargo deny --version` and `cargo machete --version`.

- [ ] **Step 2: Run the full gate exactly as CI does**

Run:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cargo machete
```
Expected: fmt prints nothing. Clippy reports 0 warnings. Tests pass across `platform`, `manifest`, `steam`, `tokens`, `discovery`, `backup`, `settings`. `cargo deny check` prints `advisories ok, bans ok, licenses ok, sources ok`. `cargo machete` reports no unused dependencies.

If `cargo deny check licenses` rejects a license used by a dependency, look it up on crates.io. If it is OSI-approved and permissive, add it to the `allow` list in `deny.toml` and note it in the commit message. If not, stop and ask the user.

- [ ] **Step 3: Mark M1 done in the roadmap**

Change the M1 row status in `docs/superpowers/plans/2026-10-10-roadmap.md` from `planned` to `done`.

- [ ] **Step 4: Commit**

```bash
git add deny.toml docs/superpowers/plans/2026-10-10-roadmap.md
git commit -m "chore: pass full M1 quality gate"
```

- [ ] **Step 5: Hand off**

Use superpowers:finishing-a-development-branch to merge `feat/m1-workspace-core` into `main`.
