# Milestone 7: Plugin Tooling, Release and Docs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Plugin authors can scaffold, package and publish plugins with `cargo xtask`. A tag push builds the release. The docs describe the Rust product.

**Architecture:**
- **`xtask package`:** zips each shippable plugin into `target/packages/{id}-{version}.zip` and writes a matching `catalog.json`. The zips are reproducible, so identical inputs give identical SHA-256s.
- **`xtask new-plugin`:** writes a form-based plugin crate from templates. It also registers the crate in the workspace and in the CI wasm lint line.
- **Release:** `.github/workflows/release.yml` builds the app on three OSes, packages the plugins, and attaches everything to a GitHub release.
- **Docs:** the five documents in spec §11 are rewritten for Rust.

**Tech Stack:** Rust 1.99 (edition 2024). `xtask` gains `zip` 9, `sha2` 0.11 and `serde_json`, all already workspace dependencies. Its tests use `saveforge-store` and `saveforge-plugin-api` as dev-dependencies. GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md`: §4.1 (package), §4.2 (manifest), §9 (release workflow), §10 (xtask), §11 (docs).

**Prior milestones:** M1–M6 are merged. The APIs used here:

| Area | API |
|---|---|
| xtask | `xtask::{WASM_TARGET, workspace_root, target_dir, PluginKind, PluginCrate, find_plugin_crates, wasm_artifact, install}`; `main.rs` has a `plugins` subcommand |
| Manifests | `saveforge_core::manifest::{PluginManifest::{load, parse, check_compat}, is_valid_id}` |
| URLs | `saveforge_core::settings::{file_url, RELEASE_CATALOG_URL}` |
| Store (tests only) | `saveforge_store::{Catalog::parse, install::install}` |
| Template basis | `plugins/sample-stardew` (`FormDocument<JsonCodec>`, `Plugin`, `export_plugin!`) |

## Global Constraints

- **Builds on the dev machine:** run every cargo command with `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6`, from a shell at below-normal priority:
  ```powershell
  (Get-Process -Id $PID).PriorityClass = 'BelowNormal'; $env:CARGO_BUILD_JOBS = 6; $env:RUST_TEST_THREADS = 6
  ```
  Do **not** write this into any `config.toml`.
- **Branch:** `feat/m7-tooling-release`, never `main`.
- **Lints:** clippy pedantic `-D warnings`, native and wasm. Clippy 1.99 rejects `assert!(x.is_empty())`; use `assert_eq!` / `assert_ne!`.
- **xtask errors:** `anyhow` (spec §7). The messages below are exact.
- **Package layout (spec §4.1):**
  - Entries in this order: `manifest.json`, the manifest's `component` path, then its `icon` path when set.
  - Names use `/` and have no directory prefix.
  - Deflate compression, every entry timestamped `1980-01-01 00:00:00` (`zip::DateTime::default()`), unix permissions `0o644`.
- **Package file name:** `{id}-{version}.zip`.
- **Catalog output:**
  - Pretty JSON with a trailing newline: `{"schemaVersion": "1.0", "plugins": [...]}`.
  - Each entry has `id`, `name`, `description`, `author`, `latestVersion`, `minHostVersion`, `maxHostVersion` (`null` when unset), `packageUrl`, `sha256` (lowercase) and `tags: []`.
  - No `generatedAt`, so identical inputs give identical output.
- **Base URL:**
  - `--base-url` must start with `https://` or `file://`; otherwise the error is `base URL must start with https:// or file://: {url}`.
  - A missing trailing `/` is added.
  - The default is `file_url(target/packages)` + `/`, so `target/packages/catalog.json` works as a catalog URL in a debug build.
- **Scaffold:**
  - `cargo xtask new-plugin <id> [--name <display name>] [--ext <.ext>] [--author <name>]`
  - Defaults:
    - name: the title-cased id words plus ` Save Editor` (`my-game` → `My Game Save Editor`)
    - ext: `.json`
    - author: `Unknown`
  - The id must pass `is_valid_id`. Otherwise: `invalid plugin id '{id}': use lowercase letters, digits and single hyphens, e.g. my-game`.
  - An existing `plugins/{id}` fails with `plugins/{id} already exists`, and nothing is written.
- **Scaffold template values:**
  - manifest `version` `0.1.0`
  - `apiVersion` = `xtask::scaffold::API_VERSION` = `"0.1"`
  - `minHostVersion` = `env!("CARGO_PKG_VERSION")` of xtask (the workspace version)
  - `maxHostVersion` `null`, `component` `plugin.wasm`
  - `description` `"{name} for SaveForge."`
  - The generated crate has no `TODO` text.
- **MSRV:** the workspace `rust-version` is `"1.96"`, the highest `rust-version` among dependencies (wasmtime 49). The current `"1.90"` is wrong.
- **Release catalog URL:** `RELEASE_CATALOG_URL = "https://github.com/YOUR_ORG/YOUR_REPO/releases/latest/download/catalog.json"`, still a placeholder. The release workflow publishes `catalog.json` as a release asset, so this URL always serves the latest one.

## Review Focus

1. **`new-plugin` with a bad id or an existing folder** (`My Game`, `../x`, `a--b`, an existing `plugins/demo`). Expected: a clear error; no files written; `Cargo.toml` and `ci.yml` unchanged. Tests: `invalid_ids_are_rejected`, `existing_plugin_dir_is_left_alone` (Task 2).
2. **A workspace `Cargo.toml` that isn't laid out as expected** (CRLF line endings, a single-line `members`, the member already present). Expected: correct insertion that keeps the line endings, or an error before anything is written. Test: `members_are_inserted_in_order` (Task 2).
3. **`package -p` naming a test fixture or an unknown id; a manifest whose icon file is missing.** Expected: an error naming the problem, and no half-written zip. Tests: `package_refuses_fixtures_and_unknown_ids`, `missing_icon_is_an_error` (Task 1).
4. **Rebuilding the same plugin twice.** Expected: byte-identical zips and the same SHA-256, so catalog entries do not churn. Test: `packages_are_reproducible` (Task 1).
5. **A package the real store can't install** (a directory prefix, backslashes, the wrong schema). Expected: `xtask`'s catalog parses with `saveforge_store::Catalog::parse` and its package installs with `saveforge_store::install::install`. Test: `catalog_round_trips_through_the_store` (Task 1).

---

## File Structure

```
rust/
  Cargo.toml                       # rust-version 1.96
  xtask/Cargo.toml                 # + zip, sha2, serde_json; dev: saveforge-store, saveforge-plugin-api
  xtask/src/lib.rs                 # + pub mod package; pub mod scaffold; select_plugins
  xtask/src/package.rs             # zips + catalog JSON
  xtask/src/scaffold.rs            # NewPlugin, render, scaffold, Cargo.toml/CI edits
  xtask/src/main.rs                # + package, new-plugin subcommands
  xtask/templates/                 # Cargo.toml.tmpl, manifest.json.tmpl, lib.rs.tmpl, test.rs.tmpl
  xtask/tests/package.rs  xtask/tests/scaffold.rs
  crates/saveforge-core/src/settings.rs   # RELEASE_CATALOG_URL
  .github/workflows/release.yml    # new
  .github/workflows/ci.yml         # + msrv job
  README.md  CLAUDE.md  docs/ARCHITECTURE.md  docs/PLUGIN_AUTHORING.md  docs/CATALOG_SCHEMA.md
```

---

### Task 1: `cargo xtask package`

**Files:**
- Modify: `xtask/Cargo.toml`, `xtask/src/lib.rs`, `xtask/src/main.rs`
- Create: `xtask/src/package.rs`
- Test: `xtask/tests/package.rs`

**Interfaces:**
- Produces, in `xtask`:
  - `select_plugins(crates: Vec<PluginCrate>, filter: Option<&str>, shippable_only: bool) -> anyhow::Result<Vec<PluginCrate>>`
    - The filter matches the package name or the manifest id, as `plugins` does today.
    - Errors:
      - `no plugin crate matched {filter}`, or `no plugin crate found` when there is no filter
      - `{name} is a test fixture, not a plugin` when `shippable_only` is set and the filter names a fixture
    - Without a filter, `shippable_only` silently skips fixtures.
    - `plugins` switches to this function (`shippable_only = false`).
  - `fn build_wasm(root: &Path, crates: &[PluginCrate]) -> anyhow::Result<()>` in `main.rs`: the existing cargo invocation, now shared by both subcommands.
- Produces, in `xtask::package`:
  - `Package { manifest: PluginManifest, path: PathBuf, sha256: String }`.
  - `package_file_name(manifest: &PluginManifest) -> String`.
  - `build_package(crate_dir: &Path, wasm: &Path, out_dir: &Path) -> anyhow::Result<Package>`:
    - loads and validates `crate_dir/manifest.json`
    - zips per Global Constraints
    - creates `out_dir`
    - writes the zip through a temp file in `out_dir` that is renamed into place, so an error leaves no partial zip
  - `catalog_json(packages: &[Package], base_url: &str) -> anyhow::Result<String>`.
- CLI:
  - Usage: `cargo xtask package [-p <package-or-plugin-id>] [--base-url <url>]`.
  - Steps:
    1. Select crates with `shippable_only = true`.
    2. Build them with `build_wasm`.
    3. Delete and recreate `target/packages`.
    4. Call `build_package` for each crate.
    5. Write `target/packages/catalog.json`.
  - Output:
    - one stdout line per package: `{zip path}  sha256:{hex}`
    - then the catalog JSON
  - The usage line in `USAGE` lists all three subcommands.

- [ ] **Step 1: Create the branch**

```bash
git checkout -b feat/m7-tooling-release
```

- [ ] **Step 2: Write the failing tests `xtask/tests/package.rs`**

Fixture: a tempdir crate dir with `manifest.json` (the `MANIFEST` constant from `tests/xtask.rs`: id `demo`, version `1.2.3`, icon `icon.png`, component `plugin.wasm`), `icon.png` = `b"png"`, and a fake `plugin.wasm` = `b"\0asm fake"` elsewhere. Read zip entries with `zip::ZipArchive`.

```rust
#[test] fn package_contains_manifest_component_and_icon() // file name "demo-1.2.3.zip"; entry names == ["manifest.json","plugin.wasm","icon.png"]; plugin.wasm bytes == fake wasm
#[test] fn component_paths_keep_forward_slashes()        // manifest component "bin/plugin.wasm" → entry "bin/plugin.wasm"
#[test] fn packages_are_reproducible()                   // build into two out dirs → identical bytes and identical sha256
#[test] fn sha256_matches_the_file()                     // sha256 == lowercase hex of Sha256::digest(fs::read(path))
#[test] fn missing_icon_is_an_error()                    // delete icon.png → Err whose chain mentions "icon.png"; out_dir has no *.zip
#[test] fn base_url_must_be_https_or_file()              // catalog_json(.., "http://x/") Err == "base URL must start with https:// or file://: http://x/";
                                                         // catalog_json(.., "https://h/r") → packageUrl "https://h/r/demo-1.2.3.zip"
#[test] fn catalog_round_trips_through_the_store()      // base = file_url(out_dir); Catalog::parse(json) → entry demo 1.2.3, sha == package.sha256,
                                                         // maxHostVersion None; install(entry, tmp/plugins) → Ok(dir) with manifest.json, plugin.wasm, icon.png
#[test] fn package_refuses_fixtures_and_unknown_ids()   // via find_plugin_crates on a temp workspace (plugins/demo + fixtures/test-plugin):
                                                         // select_plugins(.., Some("test-plugin"), true) Err "test-plugin is a test fixture, not a plugin";
                                                         // Some("nope") Err "no plugin crate matched nope"; (None, true) → ["demo"]; (None, false) → ["demo","test-plugin"]
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test -p xtask --test package`
Expected: compile errors (`package` module and `select_plugins` are missing).

- [ ] **Step 4: Implement `package.rs`, `select_plugins` and the subcommand**

Hash the zip bytes in memory with `sha2::Sha256` (a package is at most a few MB), formatting the hex with a `fold` + `write!` (clippy rejects `format!` in `collect`). Build the catalog with `serde_json::json!` and `to_string_pretty`.

- [ ] **Step 5: Run tests and lints, then try it**

Run: `cargo test -p xtask`, the lint gate, then `cargo xtask package`.
Expected:
- 8 new tests pass, and the existing xtask tests still pass.
- No warnings.
- `target/packages/` holds `planet-crafter-1.0.0.zip`, `stardew-like-0.1.0.zip` and `catalog.json`.
- A second `cargo xtask package` prints the same two SHA-256s.

- [ ] **Step 6: Commit**

```bash
git add xtask Cargo.lock
git commit -m "feat(xtask): package plugins into reproducible zips with a catalog"
```

---

### Task 2: `cargo xtask new-plugin`

**Files:**
- Create: `xtask/src/scaffold.rs`, `xtask/templates/{Cargo.toml,manifest.json,lib.rs,test.rs}.tmpl`
- Modify: `xtask/src/lib.rs`, `xtask/src/main.rs`, `xtask/Cargo.toml` (dev: `saveforge-plugin-api`)
- Test: `xtask/tests/scaffold.rs`

**Interfaces:**
- Produces, in `xtask::scaffold`:
  - `pub const API_VERSION: &str = "0.1"`.
  - `NewPlugin { id, name, extension, author: String }`:
    - `NewPlugin::new(id: &str) -> anyhow::Result<Self>` validates the id and fills in the defaults.
    - `crate_name(&self) -> String`: `-` becomes `_`.
    - `type_name(&self) -> String`: PascalCase, so `my-game` becomes `MyGame`.
  - `render(template: &str, plugin: &NewPlugin) -> String` replaces these placeholders:

    | Placeholder | Value |
    |---|---|
    | `{{id}}` | the plugin id |
    | `{{crate_name}}` | `crate_name()` |
    | `{{type_name}}` | `type_name()` |
    | `{{name}}` | display name |
    | `{{extension}}` | file extension |
    | `{{author}}` | author |
    | `{{api_version}}` | `API_VERSION` |
    | `{{host_version}}` | the workspace version |

    Templates are pulled in with `include_str!`.
  - `add_workspace_member(cargo_toml: &str, member: &str) -> anyhow::Result<String>`:
    - Finds the multi-line `members = [` array and inserts `    "{member}",` in sorted position.
    - Keeps the file's line endings: CRLF stays CRLF.
    - Errors: `{member} is already a workspace member`, or `could not find a multi-line members = [ list in Cargo.toml`.
  - `add_wasm_clippy_package(ci_yml: &str, package: &str) -> Option<String>`:
    - On the line containing both `cargo clippy` and `--target wasm32-wasip2`, inserts ` -p {package}` just before ` --target`.
    - Returns `None` when there is no such line or the package is already listed.
  - `scaffold(workspace: &Path, plugin: &NewPlugin) -> anyhow::Result<PathBuf>`:
    1. Computes the new `Cargo.toml` text first (so an error writes nothing).
    2. Refuses an existing `plugins/{id}`.
    3. Writes `plugins/{id}/Cargo.toml`, `manifest.json`, `src/lib.rs` and `tests/{crate_name}.rs`.
    4. Writes the workspace `Cargo.toml`.
    5. Updates `.github/workflows/ci.yml` if it exists and has the clippy line.
    6. Returns the plugin dir.
- **Templates** (modelled on `plugins/sample-stardew`):
  - `Cargo.toml.tmpl`: package `{{id}}`, workspace version/edition/rust-version/publish, `crate-type = ["cdylib", "rlib"]`, the dependency `saveforge-sdk.workspace = true`, and `[lints] workspace = true`.
  - `manifest.json.tmpl`: the values in Global Constraints, plus `"fileExtensions": ["{{extension}}"]`.
  - `lib.rs.tmpl`:
    - `pub struct {{type_name}};` and `pub const PRIORITY: u32 = 1;`
    - `detect` returns `Some(PRIORITY)` when the header, after leading whitespace and a UTF-8 BOM, starts with `{`. Its doc comment says to replace this with a check for a key unique to the game's saves.
    - `open` calls `FormDocument::<JsonCodec>::open(schema(), &bytes)`.
    - `schema()` has one page `General` with one section and `text("name", "Name").required()`.
    - Ends with `export_plugin!({{type_name}});`.
  - `test.rs.tmpl`:
    - `detects_json_objects`: `detect("save{{extension}}", br#"{"name":"Ada"}"#) == Some(PRIORITY)` and `detect(.., b"PK\x03\x04") == None`.
    - `opens_a_minimal_save`: `open` of `{"name":"Ada"}` gives a view containing a `Widget::Field` labelled `Name`.
- CLI usage: `cargo xtask new-plugin <id> [--name <display name>] [--ext <.ext>] [--author <name>]`. A `--ext` value without a leading `.` gets one. On success it prints `created plugins/{id}`, then `next: cargo test -p {id} && cargo xtask plugins -p {id}`.

- [ ] **Step 1: Write the failing tests `xtask/tests/scaffold.rs`**

```rust
#[test] fn plugin_defaults_from_id()            // NewPlugin::new("my-game"): name "My Game Save Editor", extension ".json", author "Unknown",
                                                // crate_name "my_game", type_name "MyGame"
#[test] fn invalid_ids_are_rejected()            // "My Game", "../x", "a--b", "" → Err; message for "My Game" ==
                                                // "invalid plugin id 'My Game': use lowercase letters, digits and single hyphens, e.g. my-game"
#[test] fn members_are_inserted_in_order()       // members [crates/a, plugins/b, xtask] + "plugins/a2" → [crates/a, plugins/a2, plugins/b, xtask];
                                                // the same input with \r\n → output has no lone \n; adding "plugins/b" again → Err;
                                                // `members = ["a"]` (single line) → Err
#[test] fn ci_clippy_line_gets_the_package()     // a line "      - run: cargo clippy -p saveforge-sdk -p planet-crafter --target wasm32-wasip2 -- -D warnings"
                                                // → contains "-p planet-crafter -p my-game --target"; no such line → None; already listed → None
#[test] fn scaffold_writes_a_valid_plugin()      // temp workspace (Cargo.toml with members, .github/workflows/ci.yml):
                                                // the four files exist; PluginManifest::parse(manifest) has id "my-game", version 0.1.0,
                                                // file_extensions [".json"], check_compat(&Version::parse(env!("CARGO_PKG_VERSION")), &Version::new(0,1,0)) Ok;
                                                // workspace Cargo.toml contains "\"plugins/my-game\","; ci.yml contains "-p my-game"; no file contains "TODO";
                                                // no "{{" left in any file
#[test] fn existing_plugin_dir_is_left_alone()   // pre-create plugins/my-game/marker.txt → Err "plugins/my-game already exists";
                                                // marker intact; Cargo.toml and ci.yml byte-identical to before
#[test] fn api_version_matches_the_host()        // API_VERSION == format!("{}.{}", saveforge_plugin_api::API_VERSION.major, ...minor)
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p xtask --test scaffold`
Expected: compile errors (the `scaffold` module is missing).

- [ ] **Step 3: Implement `scaffold.rs`, the templates and the subcommand**

- [ ] **Step 4: Run tests and lints**

Run: `cargo test -p xtask`, then the lint gate.
Expected: 7 new tests pass; no warnings.

- [ ] **Step 5: Prove a generated plugin builds, passes its tests and lints for wasm, then remove it**

```bash
cargo xtask new-plugin scaffold-check
cargo test -p scaffold-check
cargo clippy -p scaffold-check --all-targets -- -D warnings
cargo clippy -p scaffold-check --target wasm32-wasip2 -- -D warnings
cargo xtask plugins -p scaffold-check
git checkout -- Cargo.toml Cargo.lock .github/workflows/ci.yml
```

Then delete `plugins/scaffold-check` and `target/dev-plugins/scaffold-check`.
Expected:
- 2 tests pass.
- Both clippy runs are clean.
- `plugins` prints `scaffold-check -> …/dev-plugins/scaffold-check/0.1.0`.
- `git status` is clean afterwards (apart from the untracked `session.bat`).

Record the outputs in the ledger.

- [ ] **Step 6: Commit**

```bash
git add xtask Cargo.lock
git commit -m "feat(xtask): scaffold new form-based plugins"
```

---

### Task 3: Release workflow, release catalog URL and MSRV

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `.github/workflows/ci.yml` (msrv job), `Cargo.toml` (`rust-version`), `crates/saveforge-core/src/settings.rs`
- Test: `crates/saveforge-core/tests/settings.rs`

**Interfaces:** none. Later tasks only document these.

- **`release.yml`:**
  - Triggers: `push: tags: ["v*"]` and `workflow_dispatch`.
  - Actions are pinned to the majors CI already uses: `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`. Plus `softprops/action-gh-release@v2`.
  - Job `app`, a matrix of three:

    | `os` | Binary | Asset |
    |---|---|---|
    | `windows-latest` | `target/release/saveforge.exe` | `saveforge-windows-x64.exe` |
    | `ubuntu-latest` | `target/release/saveforge` | `saveforge-linux-x64` |
    | `macos-latest` | `target/release/saveforge` | `saveforge-macos-arm64` |

    It runs `cargo build --release -p saveforge-app`, copies the binary to `dist/{asset}`, and uploads it as an artifact named after the asset.
  - Job `plugins` (`ubuntu-latest`, `targets: wasm32-wasip2`):
    - Runs `cargo xtask package --base-url "https://github.com/${{ github.repository }}/releases/download/${{ github.ref_name }}/"`.
    - Uploads `target/packages/*` as the artifact `plugins`.
  - Job `publish`:
    - `needs: [app, plugins]`, `if: startsWith(github.ref, 'refs/tags/')`, `permissions: contents: write`.
    - Downloads all artifacts with `merge-multiple: true` into `dist`.
    - Runs `softprops/action-gh-release@v2` with `files: dist/*`.
- **`ci.yml`:** a new job `msrv` (`ubuntu-latest`, `dtolnay/rust-toolchain@1.96`, rust-cache) that runs `cargo check --workspace --all-targets`.

- [ ] **Step 1: Write the failing test (`settings.rs`)**

```rust
#[test]
fn release_catalog_url_points_at_the_latest_release() {
    assert!(RELEASE_CATALOG_URL.starts_with("https://github.com/"));
    assert!(RELEASE_CATALOG_URL.ends_with("/releases/latest/download/catalog.json"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p saveforge-core --test settings release_catalog_url`
Expected: FAIL (the URL is still the `raw.githubusercontent.com` placeholder).

- [ ] **Step 3: Change `RELEASE_CATALOG_URL`, set `rust-version = "1.96"`, write `release.yml` and the `msrv` job**

- [ ] **Step 4: Verify**

1. Run `cargo test -p saveforge-core --test settings`. Expected: all pass.
2. Validate both workflows with PyYAML (`python -I -c "import yaml,sys; …"`). Note that PyYAML parses the key `on` as `True`. The checks:
   - `release.yml`:
     - the trigger includes the `v*` tags and `workflow_dispatch`
     - the jobs are exactly `app`, `plugins` and `publish`
     - `publish.needs == ["app", "plugins"]`
     - the `app` matrix has 3 entries
     - one step's `run` contains `cargo xtask package --base-url`
   - `ci.yml`: it has a job `msrv`.

   Expected: the script prints `ok`.
3. Install the MSRV toolchain (this needs network and about 300 MB): `rustup toolchain install 1.96 --profile minimal`.
4. Run `cargo +1.96 check --workspace --all-targets`. Expected: finished, no errors.
   - If 1.96 fails on our own code, fix the code where the fix is small.
   - Otherwise raise `rust-version` (and the CI job) to the lowest version that passes, and record a ruling.
   - The 1.96 build writes a second set of artifacts into `target/`; run `cargo +1.96 clean -p saveforge-app` (or note the disk use) afterwards.

- [ ] **Step 5: Commit**

```bash
git add .github Cargo.toml Cargo.lock crates/saveforge-core
git commit -m "ci: release workflow, MSRV 1.96 check, release catalog URL"
```

---

### Task 4: Docs port, roadmap and gate

**Files:**
- Create: `README.md`, `CLAUDE.md`, `docs/ARCHITECTURE.md`, `docs/PLUGIN_AUTHORING.md`, `docs/CATALOG_SCHEMA.md`
- Modify: `docs/superpowers/plans/2026-10-10-roadmap.md`

Content per file (spec §11). Every command shown must be one that works in this repo. Every code snippet must be copied from real code, with its file named; don't write new code snippets.

- **`README.md`:**
  - what SaveForge is (one paragraph)
  - requirements: Rust stable ≥ 1.96, `rustup target add wasm32-wasip2`
  - build and run: `cargo xtask plugins`, then `cargo run -p saveforge-app`. Explain that debug builds load `target/dev-plugins` and the file catalog
  - tests:
    - `cargo test --workspace` (after `cargo xtask plugins`)
    - real-save tests: `SAVEFORGE_PC_SAVE=<path> cargo test -p planet-crafter -- --ignored`
  - a plugins table: `planet-crafter` and `stardew-like`, with ids, games and status
  - plugin development: `new-plugin`, `plugins -p`, and `package` with the local `target/packages/catalog.json`
  - the quality gate commands from CI
  - the project layout (spec §3.1)
  - links to the docs
- **`docs/ARCHITECTURE.md`:**
  - the crate map and dependency rules (spec §3.1–3.2)
  - the runtime model (§3.4)
  - key flows (§5.2), as implemented:
    - startup plugin scan
    - detect and open
    - save with backup (atomic write)
    - store install (staging → hash → extract → manifest check → move)
    - the startup update check (status bar / info bar)
    - the `Screen` model (the editor lives on Home)
- **`docs/PLUGIN_AUTHORING.md`:**
  1. Scaffold with `cargo xtask new-plugin`.
  2. The `manifest.json` reference: a field table plus the validation rules from spec §4.2.
  3. The `Plugin` trait (`detect`, `open`) and detection priority.
  4. The form helper: the schema builders, `VisibilityRule` and validation. Example from `plugins/sample-stardew/src/lib.rs`.
  5. Custom `view`/`update` with the `Document` trait and the UI builders. Example from `plugins/planet-crafter`.
  6. Aux files and path tokens (§4.3).
  7. The contract rules (§4.5).
  8. Testing natively with `cargo test -p`.
  9. Building and running in the app (`cargo xtask plugins`).
  10. Packaging and publishing (`cargo xtask package --base-url`, then the catalog entry).
- **`docs/CATALOG_SCHEMA.md`:**
  - the .NET schema and field table, unchanged
  - plus the rules the Rust store enforces:
    - ids must be valid
    - `sha256` is 64 hex digits in any case
    - URLs are `https://` or `file://`
    - an `https://` catalog may only reference `https://` packages
    - invalid entries are skipped, not fatal
  - package contents (§4.1)
  - generating entries with `cargo xtask package`
  - hosting: the release asset URL `https://github.com/{owner}/{repo}/releases/latest/download/catalog.json`
- **`CLAUDE.md`:**
  - the project overview, the workspace layout and dependency rules
  - build, run and test commands; `cargo xtask` commands
  - the debug plugin directory
  - conventions:
    - `thiserror` in libraries, `anyhow` only in xtask
    - `unsafe_code` forbidden; clippy pedantic `-D warnings`
    - `assert_eq!` instead of `assert!(x.is_empty())`
    - no plugin filesystem access
    - request numbers for stale background results in the app
  - a pointer to `docs/PLUGIN_AUTHORING.md`

- [ ] **Step 1: Write the five documents**

- [ ] **Step 2: Check them**

Run:
- `rg -n -i "dotnet|\.csproj|ISaveDocument|Avalonia|_devplugins|TODO" README.md CLAUDE.md docs/*.md`. Expected: no matches.
- For each `cargo …` command line in the docs, make sure the subcommand exists: `xtask` has `plugins`, `package` and `new-plugin`; `-p` targets are real packages.

Record the check in the ledger.

- [ ] **Step 3: Full gate (below-normal priority, 6 jobs)**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p saveforge-sdk -p test-plugin -p sample-stardew -p planet-crafter --target wasm32-wasip2 -- -D warnings
cargo xtask plugins
cargo test --workspace
cargo deny check
cargo machete
```

Expected:
- fmt and clippy clean.
- `cargo test --workspace`: 340 existing + 16 new = **356 passed**, 2 ignored. The new tests are xtask package 8, xtask scaffold 7 and core 1.
- `cargo deny` prints `advisories ok, bans ok, licenses ok, sources ok`.
- machete reports nothing unused.

Any failure stops the gate. If a new advisory or licence appears, stop and tell the user.

- [ ] **Step 4: Roadmap and commit**

Set the M7 row to `| 7 | `xtask package` / `new-plugin`, release workflow, docs port | `2026-10-17-m7-tooling-release-docs.md` | done |`.

```bash
git add README.md CLAUDE.md docs
git commit -m "docs: port README, architecture, plugin authoring, catalog schema and CLAUDE.md to Rust"
```

- [ ] **Step 5: Hand off**

Use superpowers:finishing-a-development-branch.
