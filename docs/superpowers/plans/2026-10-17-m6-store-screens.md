# Milestone 6: Store, Settings and About Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A plugin store, Settings and About, in the app.
- **Store:** browse a catalog from `https://` or `file://` (with an offline cache), install, update and remove plugins with verified downloads, no restart needed.
- **Startup:** an update check that reports through an info bar.
- **Settings and About:** two new screens.

**Architecture:**
- **New crate `saveforge-store`:** catalog model and parsing, URL transport with cache fallback, and package install/remove. It does all filesystem and network work synchronously and is called from the app's existing `spawn_blocking` tasks.
- **`saveforge-host`:** gains per-plugin registry updates (`load_entry`, `upsert`, `remove`), so installs take effect without a rescan.
- **`saveforge-app`:**
  - gains a `Screen` enum, rail navigation buttons, an info bar, and the three new screens
  - install progress streams into the UI through an iced channel task

**Tech Stack:** Rust 1.99 (edition 2024).
- **New dependencies:**

  | Crate | Version and features | Licence |
  |---|---|---|
  | `ureq` | `3.4.2`, `default-features = false`, `features = ["rustls-no-provider", "platform-verifier"]` | MIT/Apache-2.0 |
  | `rustls` | `0.23`, `default-features = false`, `features = ["ring", "std", "tls12"]` | |
  | `zip` | `9.0.4`, `default-features = false`, `features = ["deflate-flate2-zlib-rs"]` | MIT |
  | `sha2` | `0.11.1` | |
  | `open` | `5.4.4` | MIT |

- **Existing:** `tempfile` (now a normal dependency of the store), `semver`, `serde`, `serde_json` (workspace-wide `arbitrary_precision`), `thiserror`, `tracing`, `iced` 0.14 and `iced_test`.

**Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md`: §3.1–3.3 (crates), §4.1 (package), §5.1 (shell), §5.2 Startup step 4 / Store / Settings / About, §5.3 (settings), §7 (errors), §8 (`saveforge-store` tests). The catalog schema is `../dotnet/docs/CATALOG_SCHEMA.md`, ported unchanged.

**Decisions made with the user (2026-10-17):**
- **HTTP client:** `ureq` 3 instead of the spec's `reqwest`. `reqwest` 0.13's rustls backend needs CMake and NASM on Windows; `ureq` is synchronous and smaller.
- **TLS roots:** the OS certificate store via `rustls-platform-verifier`.
- **Builds:** run cargo at 6 jobs and below-normal priority (see Global Constraints).
- **Licence exception:** `rustls-platform-verifier` lists `webpki-root-certs` (CDLA-Permissive-2.0) as a dependency **only on `wasm32`**, so it is never compiled into the app. `cargo deny` checks all targets, so Task 2 adds a scoped `[[licenses.exceptions]]` entry for that one crate. The allowlist is unchanged.

**Prior milestones:** M1–M5 are merged. The APIs used here:

| Area | API |
|---|---|
| Settings | `saveforge_core::settings::{AppSettings { catalog_url, plugin_directory, backup_directory, check_updates_on_startup, plugin_save_directories }, AppSettings::save(&Path), file_url(&Path)}` |
| Paths | `saveforge_core::paths::AppPaths::{catalog_cache_file(), settings_file()}` |
| Core helpers | `saveforge_core::backup::write_atomic(&Path, &[u8])`, `saveforge_core::manifest::{PluginManifest::{load, check_compat}, is_valid_id}`, `CoreError` |
| Host | `saveforge_host::{HOST_VERSION, PluginEngine, PluginRegistry, PluginEntry, LoadedPlugin}` and `saveforge_plugin_api::API_VERSION` |
| App internals | `blocking(work, to_message)`, request numbers for stale results (`next_request`), `self.status` / `self.busy`, `Modal` |
| App tests | `tests/app.rs` drives `App::update` with hand-fed results (`ready_app`, `World`); `common::{World, install_fixture, fixture_wasm, engine}` |

## Global Constraints

- **Builds on the dev machine:** run every cargo command with `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6`, from a shell lowered to below-normal priority. Full-speed builds freeze the PC. In PowerShell:
  ```powershell
  (Get-Process -Id $PID).PriorityClass = 'BelowNormal'; $env:CARGO_BUILD_JOBS = 6; $env:RUST_TEST_THREADS = 6
  ```
  Child processes inherit the priority. Do **not** write this into any `config.toml`.
- **Crate rules (spec §3.2):** `saveforge-store` depends on `saveforge-core` and `saveforge-host` only, never on the app. Errors are a `thiserror` enum named `StoreError`. No `anyhow`.
- **URLs:** the catalog URL and package URLs must start with `https://` or `file://`. Anything else (including `http://`) fails with `only https:// and file:// URLs are supported: {url}`.
- **HTTP client:** one shared `ureq::Agent`:
  - TLS through `RootCerts::PlatformVerifier` and `rustls::crypto::ring::default_provider()`
  - connect timeout 15 s and response timeout 30 s
  - `User-Agent: SaveForge/{HOST_VERSION}`
- **Size limits:** catalog download at most 8 MiB (`8 * 1024 * 1024`); package download at most 64 MiB (`64 * 1024 * 1024`); total extracted bytes at most 256 MiB (`256 * 1024 * 1024`).
- **Catalog JSON (unchanged schema, camelCase keys):**

  | Field | Type | Required |
  |---|---|---|
  | `schemaVersion` | string | yes |
  | `generatedAt` | string | no |
  | `plugins` | array | yes |
  | `id` | string, must pass `is_valid_id` | yes |
  | `name` | string | yes |
  | `author` | string | yes |
  | `latestVersion` | SemVer | yes |
  | `minHostVersion` | SemVer | yes |
  | `packageUrl` | string | yes |
  | `sha256` | 64 hex chars, any case | yes |
  | `description` | string, default `""` | no |
  | `maxHostVersion` | SemVer or `null` | no |
  | `iconUrl`, `repositoryUrl` | string or `null` | no |
  | `screenshotUrls`, `tags` | string arrays, default `[]` | no |

  An entry missing a required field, or with an invalid value, is **skipped** with a `tracing::warn!`; the rest of the catalog loads.
- **Cache:** a successful fetch writes the raw catalog text to `AppPaths::catalog_cache_file()` with `write_atomic`. A failed fetch falls back to that cache. With no cache, the fetch error is returned.
- **Install layout (spec §5.2 Store step 4):**
  1. Download into a temp file under `{pluginDir}/.staging`, hashing as it streams.
  2. Compare the SHA-256 hash, ignoring case.
  3. Extract into a fresh `tempfile` dir under `{pluginDir}/.staging`.
  4. Load `manifest.json` from it; `id` and `version` must equal the catalog entry, and `check_compat(&HOST_VERSION, &API_VERSION)` must pass.
  5. If `{pluginDir}/{id}/{version}` already exists, move it into staging.
  6. Rename the extracted dir to `{pluginDir}/{id}/{version}`.

  The temp file and dir clean up on drop, so **any** failure leaves `.staging` empty and leaves an existing install untouched. The registry already skips names starting with `.` when scanning.
- **Zip rules:** reject (as `StoreError::UnsafePackage`) any entry whose `enclosed_name()` is `None` (`..` or an absolute path) and any symlink entry. Stop with `StoreError::PackageTooLarge` once extracted bytes exceed the limit. Directories are created as needed.
- **Entry status:**

  | Status | When |
  |---|---|
  | `NotInstalled` | not installed and compatible |
  | `Incompatible` | not installed and the host is outside `[minHostVersion, maxHostVersion]` |
  | `UpdateAvailable { installed }` | installed, the catalog version is greater, and it is compatible |
  | `Installed { version }` | every other installed case, including a newer but incompatible catalog version |

- **Search:** case-insensitive substring over name, description, author and each tag. An empty or whitespace-only query matches everything.
- **User-facing text (exact):**

  | Situation | Text |
  |---|---|
  | Update check running | `Checking for plugin updates…` (status) |
  | Updates found | `Updates available for {n} plugin(s). Go to Store to update.` (info bar) |
  | Update check failed | `Update check failed. Retry in Settings.` (status) |
  | Catalog served from cache | `Offline: showing the cached catalog.` (status) |
  | Install finished | `Installed {name} {version}.` |
  | Update finished | `Updated {name} to {version}.` |
  | Remove finished | `Removed {name}.` |
  | Remove blocked | `Close the open save before removing {name}.` |
  | Settings saved | `Settings saved.`, plus ` Plugin directory changes take effect after restart.` when the plugin directory changed |
  | Bad catalog URL | `Catalog URL must start with https:// or file://` (form error) |
  | Empty directory | `{label} must not be empty.` (form error) |
  | Link failed | `Could not open {url}: {error}` |
  | Store, no catalog | `Could not load the catalog: {error}` |
  | Store, no matches | `No plugins match your search.` |

- **Screens (spec §5.1):** `Screen::{Home, Store, PluginDetail(String), Settings, About}`. The spec's `Editor` screen stays `Home` with an open editor (ruling: an editor already exists as `Option<Editor>`; navigating away keeps the document open). The save panel shows only on `Home`. The rail bottom has `Store`, `Settings` and `About` buttons with tooltips; the active one is highlighted.
- **About:** `SaveForge {HOST_VERSION}`, `wasmtime {WASMTIME_VERSION}`, `Plugin API {API_VERSION}`. Links:
  - Repository: `REPOSITORY_URL = "https://github.com/YOUR_ORG/YOUR_REPO"`, a placeholder like `RELEASE_CATALOG_URL`
  - iced: `https://iced.rs`
  - wasmtime: `https://wasmtime.dev`

  Then a static licence list of the main components:

  | Component | Licence |
  |---|---|
  | iced | MIT |
  | wasmtime | Apache-2.0 WITH LLVM-exception |
  | rustls | Apache-2.0 OR ISC OR MIT |
  | ureq | MIT OR Apache-2.0 |
  | zip | MIT |
  | serde / serde_json | MIT OR Apache-2.0 |

- **Lints:** clippy pedantic `-D warnings`, native and wasm. Clippy 1.99 rejects `assert!(x.is_empty())` and `assert!(!x.is_empty())`; use `assert_eq!` / `assert_ne!`.
- **Branch:** work on `feat/m6-store-screens`, never on `main`.

## Review Focus

1. **A hostile or broken package:** `../evil.txt`, an absolute path, a symlink, a zip bomb, or not a zip at all. Expected: rejected with a clear error; nothing written outside staging; `.staging` empty afterwards; an existing version untouched. Tests: `zip_slip_entries_are_rejected`, `absolute_paths_are_rejected`, `oversized_extraction_is_rejected`, `corrupt_zip_is_rejected`, `failed_update_keeps_the_installed_version` (Task 3).
2. **One malformed catalog entry** (a bad version, a missing `packageUrl`, a bad id like `../x`). Expected: that entry is skipped, the others are listed. Test: `malformed_entries_are_skipped` (Task 1).
3. **Offline or misconfigured catalog** (network down, a typo in the URL, `http://`). Expected: the cached catalog with an "Offline" note, or a clear error when there is no cache; the app stays usable. Tests: `fetch_failure_falls_back_to_cache`, `no_cache_returns_the_fetch_error`, `http_urls_are_rejected` (Task 2), `offline_catalog_says_so` (Task 7).
4. **A package that lies** (a manifest `id` or `version` that differs from the catalog, or an incompatible host range). Expected: rejected before anything is moved into place. Tests: `manifest_mismatch_is_rejected`, `incompatible_package_is_rejected` (Task 3).
5. **`file://` catalog URLs with spaces, `%20`, or Windows drive letters** (the debug default is `file://` + the data dir, which contains the user name). Expected: the right file is read. Test: `file_urls_round_trip` (Task 2).

---

## File Structure

```
rust/
  Cargo.toml                                   # + member crates/saveforge-store, deps ureq/rustls/zip/sha2/open
  deny.toml                                    # scoped exception for webpki-root-certs (wasm32-only)
  crates/saveforge-core/src/settings.rs        # + path_from_file_url (inverse of file_url)
  crates/saveforge-store/
    Cargo.toml
    src/lib.rs          # re-exports, MAX_* limits
    src/error.rs        # StoreError
    src/catalog.rs      # Catalog, CatalogEntry, EntryStatus, parse, status, search, updates
    src/transport.rs    # agent, read_url (https/file) with progress and size cap
    src/cache.rs        # load_catalog with cache fallback
    src/install.rs      # install, remove, remove_version, zip extraction
    tests/catalog.rs  tests/transport.rs  tests/install.rs  tests/common/mod.rs (fixture zips)
  crates/saveforge-host/src/registry.rs        # load_entry, upsert, remove, PluginEntry::version
  crates/saveforge-host/src/engine.rs          # WASMTIME_VERSION
  crates/saveforge-app/src/app/mod.rs          # Screen, info bar, new messages, jobs
  crates/saveforge-app/src/app/view.rs         # rail nav, info bar, screen dispatch
  crates/saveforge-app/src/app/store_view.rs   # Store list + PluginDetail
  crates/saveforge-app/src/app/settings_view.rs  # Settings form + About
  crates/saveforge-app/src/store.rs            # StoreState, Job, progress task
  crates/saveforge-app/src/settings_form.rs    # SettingsForm, validation
  crates/saveforge-app/tests/store.rs  tests/settings_about.rs  tests/store_install.rs
```

---

### Task 1: `saveforge-store` crate: catalog model

**Files:**
- Modify: `Cargo.toml` (member `crates/saveforge-store`; workspace dep `saveforge-store = { path = "crates/saveforge-store" }`)
- Create: `crates/saveforge-store/Cargo.toml` (deps: `saveforge-core`, `saveforge-host`, `semver`, `serde`, `serde_json`, `thiserror`, `tracing`; `[lints] workspace = true`), `src/lib.rs`, `src/error.rs`, `src/catalog.rs`
- Test: `crates/saveforge-store/tests/catalog.rs`

**Interfaces:**
- Produces, in `saveforge_store`:
  - `Catalog { schema_version: String, generated_at: Option<String>, plugins: Vec<CatalogEntry> }`.
  - `Catalog::parse(text: &str) -> Result<Catalog, StoreError>`. Fails only if the top level is not an object with a string `schemaVersion` and an array `plugins`.
  - `CatalogEntry { id, name, description, author: String, latest_version, min_host_version: semver::Version, max_host_version: Option<Version>, package_url, sha256: String, icon_url: Option<String>, screenshot_urls: Vec<String>, tags: Vec<String>, repository_url: Option<String> }`. Derives `Debug, Clone, PartialEq, Deserialize` with `rename_all = "camelCase"`. `sha256` is stored lowercase.
  - `CatalogEntry` methods:
    - `is_compatible(&self, host: &Version) -> bool`
    - `matches(&self, query: &str) -> bool`
    - `status(&self, installed: Option<&Version>, host: &Version) -> EntryStatus`
  - `EntryStatus::{NotInstalled, Installed { version: Version }, UpdateAvailable { installed: Version }, Incompatible}`, deriving `Debug, Clone, PartialEq, Eq`.
  - `Catalog::entry(&self, id: &str) -> Option<&CatalogEntry>`.
  - `Catalog::search(&self, query: &str) -> Vec<&CatalogEntry>`, keeping catalog order.
  - `Update { id: String, name: String, installed: Version, latest: Version }`.
  - `Catalog::updates(&self, installed: &[(String, Version)], host: &Version) -> Vec<Update>`: the installed plugins with status `UpdateAvailable`.
  - `StoreError` (thiserror). Variants and `Display` text, exactly:

    | Variant | Text |
    |---|---|
    | `InvalidCatalog(String)` | `invalid catalog: {0}` |
    | `UnsupportedUrl(String)` | `only https:// and file:// URLs are supported: {0}` |
    | `Http { url: String, message: String }` | `could not download {url}: {message}` |
    | `Io { path: PathBuf, source: io::Error }` | `I/O error at {path}: {source}` |
    | `TooLarge { url: String, limit: u64 }` | `{url} is larger than {limit} bytes` |
    | `HashMismatch { expected: String, actual: String }` | `package checksum mismatch: expected {expected}, got {actual}` |
    | `UnsafePackage(String)` | `unsafe package entry: {0}` |
    | `PackageTooLarge` | `package expands to more than 256 MiB` |
    | `BadPackage(String)` | `invalid package: {0}` |
    | `ManifestMismatch { expected: String, found: String }` | `package is {found}, catalog says {expected}` |
    | `Core(#[from] CoreError)` | `{0}` |
    | `InvalidId(String)` | `invalid plugin id: {0}` |

- [ ] **Step 1: Create the branch**

```bash
git checkout -b feat/m6-store-screens
```

- [ ] **Step 2: Write the failing tests `crates/saveforge-store/tests/catalog.rs`**

Use a `const CATALOG: &str` fixture with three valid entries:
- `planet-crafter`: 1.2.0, min 0.1.0, tags `["survival"]`, author `SaveForge Team`, uppercase `sha256`
- `stardew-like`: 0.1.0, min 0.1.0, max 0.1.5
- `future-only`: 2.0.0, min 9.0.0

Write a `host()` helper returning `Version::new(0, 1, 0)`. Tests:

```rust
#[test] fn parses_entries_in_order()                // ids == ["planet-crafter","stardew-like","future-only"]; sha256 lowercased; description defaults to ""
#[test] fn malformed_entries_are_skipped()          // add entries: latestVersion "one", no packageUrl, id "../x", sha256 "abc" → still exactly the 3 valid ids
#[test] fn non_catalog_json_is_rejected()           // Catalog::parse("[]") and ("{\"plugins\":1}") are Err(InvalidCatalog(_)); parse("nope") Err
#[test] fn compatibility_uses_min_and_max()         // planet-crafter true; stardew-like true at 0.1.0, false at 0.2.0; future-only false
#[test] fn status_covers_every_case()               // future-only none → Incompatible; planet-crafter none → NotInstalled;
                                                    // planet-crafter Some(1.0.0) → UpdateAvailable{installed:1.0.0};
                                                    // planet-crafter Some(1.2.0) → Installed{version:1.2.0};
                                                    // future-only Some(1.0.0) → Installed{version:1.0.0} (newer but incompatible)
#[test] fn search_matches_name_description_author_and_tags() // "PLANET"→[planet-crafter]; "survival"→[planet-crafter]; "saveforge team"→[planet-crafter]; "  "→all 3; "zzz"→[]
#[test] fn updates_lists_only_compatible_newer_versions() // installed [(planet-crafter,1.0.0),(future-only,1.0.0),(gone,1.0.0)] → one Update{planet-crafter,1.0.0→1.2.0}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test -p saveforge-store --test catalog`
Expected: compile errors (the crate is empty).

- [ ] **Step 4: Implement `error.rs`, `catalog.rs`, and `lib.rs` (re-exports)**

Parse the text into `serde_json::Value`, take the `plugins` array, and deserialize each element into `CatalogEntry` on its own. Keep an entry only if `is_valid_id(&id)` holds and `sha256` is 64 hex digits; then lowercase `sha256`. Otherwise `tracing::warn!(index, reason, "skipping catalog entry")`. Search lowercases both sides.

- [ ] **Step 5: Run tests and lints**

Run: `cargo test -p saveforge-store --test catalog`, then `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 7 passed, no warnings.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/saveforge-store
git commit -m "feat(store): catalog model, status, search and updates"
```

---

### Task 2: URL transport and cached catalog loading

**Files:**
- Modify: `Cargo.toml` (workspace deps `ureq`, `rustls` exactly as in Tech Stack), `crates/saveforge-store/Cargo.toml` (+ `ureq`, `rustls`), `deny.toml`, `crates/saveforge-core/src/settings.rs`
- Create: `crates/saveforge-store/src/transport.rs`, `src/cache.rs`
- Test: `crates/saveforge-store/tests/transport.rs`, `crates/saveforge-core/tests/settings.rs` (one new test)

**Interfaces:**
- Produces:
  - `saveforge_core::settings::path_from_file_url(url: &str) -> Option<PathBuf>`, the inverse of `file_url`:
    - strips `file://`
    - on a `/X:/` prefix, drops the leading `/`
    - decodes `%XX`
    - `None` for anything that isn't a `file://` URL
  - `saveforge_store::transport`:
    - `MAX_CATALOG_BYTES: u64 = 8 * 1024 * 1024` and `MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024`.
    - `read_url(url: &str, limit: u64, sink: &mut dyn io::Write, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<u64, StoreError>`:
      - streams the body into `sink` and returns the byte count
      - calls `progress(done, total)` at least once at the end
      - `file://` uses the file length as the total; `https://` uses `Content-Length` when sent
      - exceeding `limit` gives `TooLarge`
  - `saveforge_store::cache`:
    - `CatalogSource::{Online, Cached}`.
    - `CatalogLoad { catalog: Catalog, source: CatalogSource }`.
    - `load_catalog(url: &str, cache_path: &Path) -> Result<CatalogLoad, StoreError>`.
- `deny.toml`: add

  ```toml
  [[licenses.exceptions]]
  # Dependency of rustls-platform-verifier on wasm32 targets only; never
  # compiled into SaveForge (the store crate is native-only).
  crate = "webpki-root-certs"
  allow = ["CDLA-Permissive-2.0"]
  ```

- [ ] **Step 1: Write the failing tests**

`crates/saveforge-core/tests/settings.rs`:

```rust
#[test]
fn file_urls_round_trip() {
    for p in [r"C:\Users\John Smith\AppData\Roaming\SaveForge\catalog.json", "/home/john smith/.local/share/saveforge/catalog.json"] {
        assert_eq!(path_from_file_url(&file_url(Path::new(p))), Some(PathBuf::from(p.replace('\\', "/"))));
    }
    assert_eq!(path_from_file_url("file:///C:/a%20b/c.json"), Some(PathBuf::from("C:/a b/c.json")));
    assert_eq!(path_from_file_url("https://x/c.json"), None);
}
```

`crates/saveforge-store/tests/transport.rs` (tempdir fixtures; no network):

```rust
#[test] fn file_urls_are_read_with_progress()   // write 10_000 bytes; read_url(file_url) → Ok(10_000), sink == bytes, last progress == (10_000, Some(10_000))
#[test] fn http_urls_are_rejected()             // read_url("http://example.com/c.json", ..) → Err(UnsupportedUrl) with Display "only https:// and file:// URLs are supported: http://example.com/c.json"
#[test] fn oversized_downloads_stop_at_the_limit() // 2_000-byte file, limit 1_000 → Err(TooLarge{limit:1000,..})
#[test] fn missing_file_is_an_io_error()        // nonexistent file:// → Err(Io{..})
#[test] fn online_catalog_is_cached()           // load_catalog(file_url(catalog.json), cache) → source Online; cache file text == catalog text
#[test] fn fetch_failure_falls_back_to_cache()  // cache holds a valid catalog, URL points to a missing file → source Cached, entries from cache
#[test] fn no_cache_returns_the_fetch_error()   // missing URL + missing cache → Err(Io{..}) for the URL path
#[test] fn invalid_online_catalog_falls_back_to_cache() // URL serves "not json" + valid cache → Cached
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p saveforge-core --test settings file_urls_round_trip` and `cargo test -p saveforge-store --test transport`
Expected: compile errors (`path_from_file_url`, `transport` and `cache` are missing).

- [ ] **Step 3: Implement `path_from_file_url`, `transport.rs` and `cache.rs`**

- **Agent:** build the shared agent once in a `std::sync::LazyLock<ureq::Agent>`, configured as below.

  ```rust
  let tls = TlsConfig::builder()
      .provider(TlsProvider::Rustls)
      .root_certs(RootCerts::PlatformVerifier)
      .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
      .build();
  ureq::Agent::config_builder()
      .tls_config(tls)
      .timeout_connect(Some(Duration::from_secs(15)))
      .timeout_recv_response(Some(Duration::from_secs(30)))
      .user_agent(format!("SaveForge/{}", *HOST_VERSION))
      .build()
      .new_agent()
  ```

- **Streaming:** read with `response.body_mut().as_reader()` in 64 KiB chunks.
- **Errors:** map `ureq` failures to `Http { url, message: e.to_string() }`.
- **Cache:** `load_catalog` fetches into a `Vec`, parses it, then `write_atomic`s the cache. A cache-write failure is only a `tracing::warn!`. On any fetch or parse error it falls back to the cache.

- [ ] **Step 4: Run tests and lints**

Run: `cargo test -p saveforge-core --test settings && cargo test -p saveforge-store`, then the lint gate and `cargo deny check`.
Expected: settings +1 passed, transport 8 passed, catalog 7 passed. `cargo deny` prints `advisories ok, bans ok, licenses ok, sources ok`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock deny.toml crates/saveforge-core crates/saveforge-store
git commit -m "feat(store): https/file transport with size caps and cached catalog"
```

---

### Task 3: Install, update and remove packages

**Files:**
- Modify: `Cargo.toml` (workspace deps `zip`, `sha2`), `crates/saveforge-store/Cargo.toml` (+ `zip`, `sha2`, `tempfile`, `saveforge-plugin-api`; dev: `zip` is reused for fixtures)
- Create: `crates/saveforge-store/src/install.rs`
- Test: `crates/saveforge-store/tests/install.rs`, `crates/saveforge-store/tests/common/mod.rs`

**Interfaces:**
- Consumes: `transport::{read_url, MAX_PACKAGE_BYTES}`, `CatalogEntry`, `PluginManifest::load`, `HOST_VERSION`, `API_VERSION`.
- Produces, in `saveforge_store::install`:
  - `MAX_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024`.
  - `install(entry: &CatalogEntry, plugins_dir: &Path, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<PathBuf, StoreError>` returns `{plugins_dir}/{id}/{version}`. Its steps are listed under Global Constraints, "Install layout".
  - `remove(plugins_dir: &Path, id: &str) -> Result<(), StoreError>`: `InvalidId` unless `is_valid_id`; deletes `{plugins_dir}/{id}`; a missing dir is `Ok`.
  - `remove_version(plugins_dir: &Path, id: &str, version: &Version) -> Result<(), StoreError>`: the same checks; deletes `{plugins_dir}/{id}/{version}` only.
- Test fixture `tests/common/mod.rs`:
  - `package(files: &[(&str, &[u8])]) -> Vec<u8>` builds a zip with `zip::ZipWriter` using `CompressionMethod::Stored`.
  - `manifest_json(id, version, min_host) -> String` is a valid manifest pointing at `plugin.wasm`.
  - `sha256_hex(&[u8]) -> String`.
  - `entry_for(id, version, package_path, sha) -> CatalogEntry` uses `file_url`.

- [ ] **Step 1: Write the failing tests `crates/saveforge-store/tests/install.rs`**

Each test uses a `tempfile::TempDir` as `plugins_dir` and writes the package to `<tmp>/pkg.zip`.

```rust
#[test] fn installs_into_id_and_version()         // ok path: returns plugins/demo/1.0.0; manifest.json + plugin.wasm there; .staging empty
#[test] fn checksum_mismatch_is_rejected()        // wrong sha → HashMismatch; plugins/demo absent; .staging empty
#[test] fn zip_slip_entries_are_rejected()        // entry "../evil.txt" → UnsafePackage; <tmp>/evil.txt absent; .staging empty
#[test] fn absolute_paths_are_rejected()          // entry "/abs.txt" (and "C:/abs.txt" on Windows via cfg) → UnsafePackage
#[test] fn oversized_extraction_is_rejected()     // call extract helper `extract_zip(reader, dest, limit)` with limit 1_000 on a 2_000-byte entry → PackageTooLarge
#[test] fn corrupt_zip_is_rejected()              // bytes "not a zip" with matching sha → BadPackage(_)
#[test] fn manifest_mismatch_is_rejected()        // package says version 1.0.1, catalog 1.0.0 → ManifestMismatch{expected:"demo 1.0.0", found:"demo 1.0.1"}
#[test] fn incompatible_package_is_rejected()     // manifest minHostVersion 99.0.0 → Err(StoreError::Core(_)) whose Display contains "incompatible plugin"; plugins/demo absent; .staging empty
#[test] fn reinstalling_the_same_version_replaces_it() // install twice with different plugin.wasm bytes → second bytes on disk; .staging empty
#[test] fn failed_update_keeps_the_installed_version() // install 1.0.0, then 1.1.0 with bad sha → 1.0.0 dir intact
#[test] fn remove_deletes_the_plugin_folder()     // install, remove → plugins/demo absent; remove again → Ok
#[test] fn remove_rejects_bad_ids()               // remove(dir, "../x") → InvalidId; remove_version(dir,"demo",1.0.0) deletes only that version
```

`extract_zip(reader: impl Read + Seek, dest: &Path, limit: u64) -> Result<(), StoreError>` is `pub` so the limit can be tested without building a 256 MiB fixture.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p saveforge-store --test install`
Expected: compile errors (`install` is missing).

- [ ] **Step 3: Implement `install.rs`**

- **Download:** create the staging root, then `tempfile::NamedTempFile::new_in(staging_root)`, and download into a writer that both writes to the file and feeds a `Sha256` (a small `HashingWriter`). Format the hash as lowercase hex: `digest.iter().map(|b| format!("{b:02x}")).collect()`.
- **Extract:** into `tempfile::Builder::new().tempdir_in(staging_root)`. In `extract_zip`:
  - `zip::ZipArchive::new` failing → `BadPackage(e.to_string())`.
  - Per entry: `enclosed_name()` is `None` → `UnsafePackage(name)`; `is_symlink()` → `UnsafePackage(name)`.
  - Copy through `Read::take(remaining + 1)` and fail with `PackageTooLarge` once the running total passes `limit`.
- **Manifest:** a manifest that fails to load maps to `Core(_)`. Compare `{id} {version}` strings for `ManifestMismatch`.
- **Move:**
  1. `fs::create_dir_all({plugins_dir}/{id})`.
  2. If the target exists, `fs::rename` it to a sibling temp dir in staging (dropped afterwards).
  3. `fs::rename(staged.keep(), target)`.

- [ ] **Step 4: Run tests and lints**

Run: `cargo test -p saveforge-store` and the lint gate.
Expected: catalog 7, transport 8, install 12 passed; no warnings.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/saveforge-store
git commit -m "feat(store): verified install, update and remove with zip-slip and size guards"
```

---

### Task 4: Registry updates and the wasmtime version

**Files:**
- Modify: `crates/saveforge-host/src/registry.rs`, `crates/saveforge-host/src/engine.rs`, `crates/saveforge-host/src/lib.rs` (export `WASMTIME_VERSION`)
- Test: `crates/saveforge-host/tests/registry.rs`, `crates/saveforge-host/tests/engine.rs`

**Interfaces:**
- Produces, in `saveforge_host`:
  - `PluginRegistry::load_entry(engine: &PluginEngine, plugins_dir: &Path, id: &str) -> PluginEntry`. This is the existing `load_newest` for `{plugins_dir}/{id}`.
  - `PluginRegistry::upsert(&mut self, entry: PluginEntry)`: replaces the entry with the same id or inserts it, keeps entries sorted by id, and resets that id's fault count.
  - `PluginRegistry::remove(&mut self, id: &str) -> bool`.
  - `PluginRegistry::installed_versions(&self) -> Vec<(String, Version)>`: one row per entry with a known version (see `version()`).
  - `PluginEntry::version(&self) -> Option<Version>`: `Loaded` gives the manifest version; `Failed` parses `dir.file_name()`.
  - `pub const WASMTIME_VERSION: &str = "49.0.3"`.

- [ ] **Step 1: Write the failing tests**

`registry.rs`:

```rust
#[test] fn upsert_adds_and_replaces_in_id_order() // scan with "b-plugin"; install fixture "a-plugin"; upsert(load_entry(..,"a-plugin")) → ids ["a-plugin","b-plugin"]; upsert again → still 2
#[test] fn upsert_resets_fault_count()            // record_fault twice, upsert reloaded entry, record_fault twice more → still Loaded
#[test] fn remove_drops_the_entry()               // remove("b-plugin") == true, then false; get("b-plugin") None
#[test] fn installed_versions_lists_loaded_and_failed() // fixture at 0.1.0 → [("x",0.1.0)]; failed dir "{id}/1.2.3" with bad manifest → contains ("bad",1.2.3)
```

`engine.rs`:

```rust
#[test]
fn wasmtime_version_matches_the_lockfile() {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock")).unwrap();
    let block = lock.split("[[package]]").find(|b| b.contains("\nname = \"wasmtime\"\n")).unwrap();
    assert!(block.contains(&format!("version = \"{WASMTIME_VERSION}\"")), "{block}");
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo xtask plugins && cargo test -p saveforge-host --test registry --test engine`
Expected: compile errors (`upsert`, `remove`, `load_entry`, `installed_versions` and `WASMTIME_VERSION` are missing).

- [ ] **Step 3: Implement them in `registry.rs` and `engine.rs`**

- [ ] **Step 4: Run tests and lints**

Run: `cargo test -p saveforge-host` and the lint gate.
Expected: all host tests pass (4 new registry, 1 new engine); no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/saveforge-host
git commit -m "feat(host): per-plugin registry updates and wasmtime version constant"
```

---

### Task 5: Screens, rail navigation, info bar, About

**Files:**
- Modify: `crates/saveforge-app/Cargo.toml` (+ `saveforge-store`, `open`), `crates/saveforge-app/src/app/mod.rs`, `src/app/view.rs`, `src/lib.rs`
- Create: `crates/saveforge-app/src/app/settings_view.rs` (About for now; Settings joins in Task 6)
- Test: `crates/saveforge-app/tests/settings_about.rs`

**Interfaces:**
- Produces, in `saveforge_app`:
  - `pub enum Screen { Home, Store, PluginDetail(String), Settings, About }`, deriving `Debug, Clone, PartialEq, Eq`, defaulting to `Home`.
  - `App::screen(&self) -> &Screen` and `App::info(&self) -> Option<&str>`.
  - `pub const REPOSITORY_URL: &str = "https://github.com/YOUR_ORG/YOUR_REPO"`.
  - New `Message` variants: `Navigate(Screen)`, `DismissInfo`, `OpenUrl(String)`.
  - `SelectPlugin` also sets `Screen::Home`, including when the same plugin is already selected; then it returns `Task::none()` as before.
- Behaviour:
  - **`Navigate`** never closes or guards the editor; the open document survives a visit to About.
  - **`OpenUrl`** calls `open::that_detached(&url)`; an error sets status `Could not open {url}: {error}`.
  - **`view`:**
    - The info bar (when `info` is set) sits above the main row, with a `Dismiss` button.
    - The save panel renders only on `Screen::Home`.
    - The main pane dispatches on `screen`.
    - The rail adds a `space::vertical()` and then three buttons, `Store`, `Settings` and `About`, with tooltips; the active one uses `theme::rail_button(true)`.

- [ ] **Step 1: Write the failing tests `crates/saveforge-app/tests/settings_about.rs`**

Reuse the `ready_app` and `World` helpers; copy `settings`/`ready_app` from `tests/app.rs` into this file.

```rust
#[test] fn navigation_switches_screens_and_keeps_the_editor() // open a fixture save; Navigate(About) → screen About, editor still Some; SelectPlugin("test-plugin") → Home, editor Some
#[test] fn about_shows_versions_and_links()   // simulator(app.view()): find(format!("SaveForge {}", *HOST_VERSION)), find(format!("wasmtime {WASMTIME_VERSION}")), find("Repository")
#[test] fn info_bar_can_be_dismissed()       // here: DismissInfo with no info is a no-op, info() stays None (Task 7 extends it with UpdatesChecked)
#[test] fn rail_has_store_settings_and_about() // simulator: find("Store"), find("Settings"), find("About")
#[test] fn save_panel_is_only_on_home()      // on Home find("Set path") or the plugin name works; Navigate(Settings) → find of the save-panel title fails
```

Note: `info_bar_can_be_dismissed` is completed in Task 7 once `UpdatesChecked` exists. Here it asserts only the no-op.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p saveforge-app --test settings_about`
Expected: compile errors (`Screen`, `Navigate`, `screen()`, `WASMTIME_VERSION` use).

- [ ] **Step 3: Implement it**

`settings_view.rs` holds `fn about(&self) -> Element<'_, Message>`, with the version lines, link buttons sending `OpenUrl`, and the licence table from Global Constraints.

- [ ] **Step 4: Run tests and lints**

Run: `cargo test -p saveforge-app` and the lint gate.
Expected: all app tests pass (the existing 40-odd plus 5 new); no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/saveforge-app
git commit -m "feat(app): screens, rail navigation, info bar and About"
```

---

### Task 6: Settings screen

**Files:**
- Create: `crates/saveforge-app/src/settings_form.rs`
- Modify: `src/app/mod.rs`, `src/app/settings_view.rs`, `src/lib.rs`
- Test: `crates/saveforge-app/tests/settings_about.rs`

**Interfaces:**
- Produces, in `saveforge_app::settings_form`:
  - `SettingsForm { catalog_url: String, plugin_directory: String, backup_directory: String, check_updates_on_startup: bool, errors: Vec<String> }`, built with `SettingsForm::from_settings(&AppSettings)`.
  - `SettingsField::{CatalogUrl, PluginDirectory, BackupDirectory}`.
  - `SettingsForm::validate(&self) -> Vec<String>`, in field order:
    - `Catalog URL must start with https:// or file://`
    - `Plugin directory must not be empty.`
    - `Backup directory must not be empty.`
  - `SettingsForm::apply(&self, settings: &mut AppSettings) -> bool` updates the four fields in place and returns whether `plugin_directory` changed.
- `App::settings_form(&self) -> &SettingsForm`, for tests and the view.
- New `Message` variants: `SettingsEdited(SettingsField, String)`, `SettingsToggled(bool)`, `BrowseSettingsDir(SettingsField)`, `SettingsDirBrowsed(SettingsField, Option<PathBuf>)`, `SaveSettings`, `CheckUpdatesNow`.
- **On save:**
  - Validation errors are shown under the form, and nothing is saved.
  - Otherwise, `apply`, then `settings.save(&paths.settings_file())`.
    - On success: status `Settings saved.` (plus the restart sentence when the plugin directory changed).
    - On failure: status `Could not save settings ({e}); changes kept for this session.`
  - A changed catalog URL drops the loaded catalog (`StoreState::catalog = NotLoaded`, Task 7). In this task, record the old URL and add the reset in Task 7.
  - `Navigate(Settings)` rebuilds the form from the current settings.
- **`CheckUpdatesNow`** is wired in Task 7. Here it only sends the status `Checking for plugin updates…`.

- [ ] **Step 1: Write the failing tests (append to `settings_about.rs`)**

```rust
#[test] fn settings_form_validates_fields()        // catalog_url "ftp://x", backup "" → errors == ["Catalog URL must start with https:// or file://", "Backup directory must not be empty."]
#[test] fn saving_settings_keeps_save_directories() // settings has plugin_save_directories {test-plugin: <saves>}; edit backup dir, SaveSettings → reload file: plugin_save_directories unchanged, backupDirectory new
#[test] fn changing_plugin_directory_says_restart() // edit plugin dir, save → status == "Settings saved. Plugin directory changes take effect after restart."; app still uses old registry
#[test] fn invalid_settings_are_not_saved()       // bad URL, SaveSettings → settings file unchanged on disk; form.errors non-empty (assert_ne!(len,0)); view shows the error text
#[test] fn unwritable_settings_keep_changes_in_memory() // settings path is a directory → status starts with "Could not save settings"; app's settings has the new backup dir
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p saveforge-app --test settings_about`
Expected: compile errors (`SettingsField`, `SaveSettings` and the other new messages are missing).

- [ ] **Step 3: Implement `settings_form.rs`, the update arms, and `fn settings(&self)` in `settings_view.rs`**

The form has three `text_input`s with `Browse…` buttons (an `rfd` folder picker, run the way `BrowsePath` already does), a checkbox `Check for plugin updates on startup`, and buttons `Save` and `Check now`. Show the errors in `theme::error_text`. Under the plugin directory field, show the muted note `Takes effect after restart.`

- [ ] **Step 4: Run tests and lints**

Run: `cargo test -p saveforge-app` and the lint gate.
Expected: +5 passed; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/saveforge-app
git commit -m "feat(app): settings screen"
```

---

### Task 7: Store screens, jobs and the startup update check

**Files:**
- Create: `crates/saveforge-app/src/store.rs`, `crates/saveforge-app/src/app/store_view.rs`
- Modify: `src/app/mod.rs`, `src/app/view.rs`, `src/lib.rs`
- Test: `crates/saveforge-app/tests/store.rs`, and `info_bar_can_be_dismissed` in `settings_about.rs`

**Interfaces:**
- Consumes: `saveforge_store::{Catalog, CatalogEntry, EntryStatus, Update, cache::{load_catalog, CatalogLoad, CatalogSource}, install::{install, remove, remove_version}}`, `PluginRegistry::{load_entry, upsert, remove, installed_versions}`, `HOST_VERSION`.
- Produces, in `saveforge_app::store`:
  - `CatalogState::{NotLoaded, Loading, Ready(CatalogLoad), Failed(String)}`.
  - `JobKind::{Install, Update, Remove}`.
  - `Job { request: u64, plugin_id: String, kind: JobKind, progress: Option<(u64, Option<u64>)> }`.
  - `JobOutcome::{Installed { entry: PluginEntry, name: String, version: Version, replaced: Option<Version> }, Removed { id: String, name: String }}`.
  - `StoreState { catalog: CatalogState, query: String, job: Option<Job> }`.
  - `fn progress_fraction(progress: Option<(u64, Option<u64>)>) -> Option<f32>` returns `done / total` when the total is known and above 0.
  - `App::store(&self) -> &StoreState`, for tests. Tests read the registry through the accessor `tests/app.rs::repeated_plugin_crashes_disable_the_plugin` already uses; add a `pub fn registry(&self) -> Option<&PluginRegistry>` if none exists.
  - `fn job_task(request: u64, work: impl FnOnce(&mut dyn FnMut(u64, Option<u64>)) -> Result<JobOutcome, String> + Send + 'static) -> Task<Message>` streams progress messages, then a finished message.
- New `Message` variants:

  | Variant | Purpose |
  |---|---|
  | `CatalogLoaded { request: u64, result: Result<CatalogLoad, String> }` | catalog fetch finished |
  | `RefreshCatalog` | fetch again |
  | `StoreSearch(String)` | search box changed |
  | `InstallPlugin(String)`, `UpdatePlugin(String)`, `RemovePlugin(String)` | start a job |
  | `JobProgress { request: u64, done: u64, total: Option<u64> }` | progress from the job |
  | `JobFinished { request: u64, result: Result<JobOutcome, String> }` | job finished |
  | `UpdatesChecked { result: Result<(CatalogLoad, Vec<Update>), String> }` | update check finished |

- Behaviour:
  - **Catalog loading:** `Navigate(Store)` and `Navigate(PluginDetail(_))` start `load_catalog` (via `blocking`) when the catalog is `NotLoaded` or `Failed`. `RefreshCatalog` always starts it. A `CatalogLoaded` result with a stale `request` is ignored. `Cached` sets the status `Offline: showing the cached catalog.`.
  - **Startup:** in `PluginsLoaded`, when `check_updates_on_startup` is set, show the status `Checking for plugin updates…` and run `load_catalog` plus `catalog.updates(registry.installed_versions(), &HOST_VERSION)` in one `blocking` task. `CheckUpdatesNow` (Task 6) runs the same.
  - **`UpdatesChecked`:**
    - `Ok` with n > 0: info bar `Updates available for {n} plugin(s). Go to Store to update.`
    - `Ok` with 0: clear that status.
    - `Ok` in either case also sets `catalog = Ready(load)`.
    - `Err`: status `Update check failed. Retry in Settings.`
  - **Jobs:**
    - Only one job at a time; `Install`, `Update` and `Remove` messages while a job runs are ignored.
    - **Install:** runs `install(entry, plugins_dir, progress)`, then `PluginRegistry::load_entry`, giving `Installed { replaced: None }`.
    - **Update:** runs Install for the new version, then `remove_version(old)` when the old version differs, giving `Installed { replaced: Some(old) }`.
    - **Remove:** runs `remove(plugins_dir, id)`, giving `Removed`.
    - The plugin directory is the one the registry was scanned from, kept as `App::plugin_dir` at startup (a Settings change does not affect it until restart).
  - **`JobFinished`:**
    - `Installed`: `registry.upsert(entry)`; status `Installed {name} {version}.` or `Updated {name} to {version}.`
    - `Removed`: `registry.remove(id)`; when it was `home.selected`, clear the selection; status `Removed {name}.`
    - `Err(e)`: status `e`.
    - The job is cleared in every case.
  - **Remove guard:** `RemovePlugin(id)` while the open editor belongs to `id` sets the status `Close the open save before removing {name}.` and starts nothing.
  - **Store list view:**
    - a search `text_input` (placeholder `Search plugins`) and a `Refresh` button
    - rows: name (bold), `by {author}`, description, and a status tag (`Installed`, `Update available`, `Incompatible`, or none); clicking a row sends `Navigate(PluginDetail(id))`
    - `Loading…` while loading, `Could not load the catalog: {e}` on failure, `No plugins match your search.` when empty
  - **Detail view:**
    - name, `by {author}`, `Version {latest}` (plus `installed {v}` when installed), description, `Tags: a, b`, and a `Repository` button when `repository_url` is set
    - the action button for the status: `Install`, `Update`, `Remove` (shown for `Installed` and `UpdateAvailable`), or the muted text `Needs SaveForge {min}` / `Needs SaveForge {min}–{max}`
    - while this plugin's job runs: an `iced::widget::progress_bar(0.0..=1.0, fraction)` (or `Working…` text when the size is unknown), with the buttons disabled
    - a `Back to Store` button

- [ ] **Step 1: Write the failing tests `crates/saveforge-app/tests/store.rs`**

Build a `file://` catalog in the world dir whose entry `test-plugin` has version `0.1.0`, plus an entry `other` (not installed). Feed results by calling `load_catalog` / `install` directly, in the style of `tests/app.rs`.

```rust
#[test] fn store_lists_entries_with_status()        // Navigate(Store) → screen Store; feed CatalogLoaded(load_catalog(..)) → simulator finds "test-plugin" row with "Installed" and "other" without a tag
#[test] fn stale_catalog_results_are_ignored()      // two RefreshCatalog; feed the first request's result → catalog still Loading
#[test] fn offline_catalog_says_so()                // feed Ok(CatalogLoad{source: Cached}) → status "Offline: showing the cached catalog."
#[test] fn search_filters_the_list()                // StoreSearch("other") → view finds "other", not "test-plugin" ; StoreSearch("zzz") → "No plugins match your search."
#[test] fn install_outcome_adds_the_plugin_to_the_rail() // package the fixture (manifest id "other" + test-plugin.wasm) as zip; run install()+load_entry; feed JobFinished(Ok(Installed)) → registry has "other" Loaded; status "Installed other 0.1.0."
#[test] fn remove_is_blocked_while_its_save_is_open() // open a test-plugin save; RemovePlugin("test-plugin") → status "Close the open save before removing <name>."; no job
#[test] fn remove_outcome_drops_plugin_and_selection() // JobFinished(Ok(Removed{id:"test-plugin"..})) → registry lacks it; home.selected None; status "Removed <name>."
#[test] fn second_job_is_ignored_while_one_runs()   // InstallPlugin("other") twice → only one job (job request unchanged)
#[test] fn job_failure_reports_and_clears_job()     // JobFinished(Err("package checksum mismatch: …")) → status equals it; job None
#[test] fn progress_fraction_handles_unknown_totals() // (50,Some(200))→Some(0.25); (5,None)→None; (0,Some(0))→None
#[test] fn startup_update_check_reports_updates()    // settings check_updates true; PluginsLoaded → status "Checking for plugin updates…"; feed UpdatesChecked(Ok((load, vec![Update{..}]))) → info "Updates available for 1 plugin(s). Go to Store to update."
#[test] fn failed_update_check_points_to_settings()  // UpdatesChecked(Err(..)) → status "Update check failed. Retry in Settings."
```

Complete `info_bar_can_be_dismissed` in `settings_about.rs`: after an `UpdatesChecked` with one update, `info()` is `Some`; after `DismissInfo`, it is `None`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo xtask plugins && cargo test -p saveforge-app --test store`
Expected: compile errors (the `store` module and the new messages are missing).

- [ ] **Step 3: Implement `store.rs`, the update arms, and `store_view.rs`**

`job_task` bridges the blocking job and its progress into one iced task. Build it with `iced::stream::channel` and `Task::run`; the outline below is the approach, not the final code.

```rust
let stream = iced::stream::channel(32, move |mut out: iced::futures::channel::mpsc::Sender<Message>| async move {
    let mut progress_out = out.clone();
    let result = tokio::task::spawn_blocking(move || work(&mut |done, total| {
        let _ = progress_out.try_send(Message::JobProgress { request, done, total });
    }))
    .await
    .unwrap_or_else(|e| Err(format!("job panicked: {e}")));
    let _ = out.send(Message::JobFinished { request, result }).await;
});
Task::run(stream, |m| m)
```

Progress messages may be dropped when the channel is full; that is fine, because the next one or `JobFinished` follows. `JobProgress` and `JobFinished` with a stale `request` are ignored.

- [ ] **Step 4: Run tests and lints**

Run: `cargo test -p saveforge-app` and the lint gate.
Expected: store 12 passed, settings_about 10 passed; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/saveforge-app
git commit -m "feat(app): store list, plugin detail, install/update/remove jobs, startup update check"
```

---

### Task 8: End to end, smoke run, docs and gate

**Files:**
- Test: `crates/saveforge-app/tests/store_install.rs`
- Modify: `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md`, `docs/superpowers/plans/2026-10-10-roadmap.md`, `.github/workflows/ci.yml` (if the store crate needs nothing new, leave it)

- [ ] **Step 1: Write the end-to-end test `store_install.rs`**

```rust
#[test]
fn installs_from_a_file_catalog_and_opens_a_save() {
    // world with an EMPTY plugins dir; package fixtures/test-plugin manifest (id "store-demo") + fixture wasm as zip;
    // catalog.json with that entry (sha256 of the zip) at a path containing a space ("my catalog/catalog.json");
    // load_catalog(file_url(..)) → Online; install(entry, plugins_dir, &mut |_,_|{}) → Ok;
    // PluginRegistry::load_entry → Loaded; upsert into a fresh registry;
    // detect_file + open_document on a fixture save → opens with plugin "store-demo".
}
```

- [ ] **Step 2: Run it**

Run: `cargo xtask plugins && cargo test -p saveforge-app --test store_install`
Expected: 1 passed. (RED here is proven by Tasks 2–4. If it fails, the integration is wrong; debug it rather than loosening the test.)

- [ ] **Step 3: Smoke-run the app with a local catalog**

1. Run `cargo xtask plugins`.
2. Copy `target/dev-plugins/stardew-like/0.1.0/` into a temp folder.
3. Zip its three files into `<tmp>/stardew-like-0.1.0.zip`.
4. Write `catalog.json` beside the zip, with the `stardew-like` entry and the zip's SHA-256.
5. Launch the debug app at below-normal priority. In Settings, set the catalog URL to that `file://` URL, Save, and open Store. Expect `StardewLike Save Editor` with the `Installed` tag (it is already in `dev-plugins`).
6. Remove it. Expect the rail to lose **S**.
7. Install it again from the detail page. Expect the progress bar, then the status `Installed StardewLike Save Editor 0.1.0.` and **S** back on the rail.
8. Take a `PrintWindow` screenshot as in M5 (wait about 30 s for the cold component cache), then stop the process.

Record what the screenshots show in the ledger. Afterwards, restore the debug settings' catalog URL. The settings file lives under the app data dir; delete only the `catalogUrl` key you changed, or put back its previous value.

- [ ] **Step 4: Update the spec**

In §3.3, replace `` `reqwest` (rustls) `` with `` `ureq` (rustls on ring, OS trust store via `rustls-platform-verifier`) ``. In §5.1, add after the `Screen` bullet: `` `Editor` is `Home` with an open document; visiting Store, Settings or About keeps the document open. ``

- [ ] **Step 5: Full gate (below-normal priority, 6 jobs)**

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
- `cargo test --workspace`: 272 existing + 56 new = **328 passed**, with 2 ignored. The 56 new tests are core 1, store 27 (catalog 7, transport 8, install 12), host 5, and app 23 (settings_about 10, store 12, store_install 1).
- `cargo deny` prints `advisories ok, bans ok, licenses ok, sources ok`.
- machete reports nothing unused.

Compare against the actual sum. Any failure stops the gate. A new licence or advisory: stop and report it to the user.

- [ ] **Step 6: Roadmap and commit**

Change the M6 row to `| 6 | `saveforge-store` + Store / PluginDetail / Settings / About screens | `2026-10-17-m6-store-screens.md` | done |`.

```bash
git add crates/saveforge-app/tests/store_install.rs docs/superpowers Cargo.lock
git commit -m "test: install from a file catalog end to end; spec and roadmap for M6"
```

- [ ] **Step 7: Hand off**

Use superpowers:finishing-a-development-branch.
