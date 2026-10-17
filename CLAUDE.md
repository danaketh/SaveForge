# SaveForge — Claude Context

## Project overview

SaveForge is a desktop app (Rust, iced) for editing save files of single-player games through plugins. A plugin is a sandboxed WebAssembly component (`wasm32-wasip2`, wasmtime component model). It parses a save, describes an editor UI as a tree of host widgets, and serializes the edited save. The host does all file and network I/O.

- **Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md`
- **Milestone plans:** `docs/superpowers/plans/`

## Workspace layout

```
crates/saveforge-core/        # settings, path tokens, Steam lookup, discovery, backup + atomic write, manifests
crates/saveforge-plugin-api/  # host bindings of wit/, UI-tree types
crates/saveforge-host/        # wasmtime engine, plugin loading, registry, detection, sessions
crates/saveforge-store/       # catalog + cache, downloads, SHA-256, install/update/remove
crates/saveforge-app/         # iced app (binary `saveforge`): screens, theme, renderer
sdk/saveforge-sdk/            # guest SDK: Plugin/Document traits, ui builders, form helper, export_plugin!
plugins/planet-crafter/       # Planet Crafter (custom view/update, lossless round-trip)
plugins/sample-stardew/       # reference schema-driven plugin
fixtures/test-plugin/         # component used by host/app tests
wit/                          # the plugin interface (shared by host and SDK)
xtask/                        # cargo xtask plugins | package | new-plugin
```

Dependency rules:

```
saveforge-app    → core, host, store
saveforge-host   → core, plugin-api
saveforge-store  → core, host
plugins/*        → saveforge-sdk only
```

## Commands

```bash
cargo xtask plugins [-p <id>]                  # build plugins into target/dev-plugins (and the test fixture)
cargo run -p saveforge-app                     # run the app
cargo test --workspace                         # all tests (run `cargo xtask plugins` first)
cargo xtask package [-p <id>] [--base-url <url>]   # zips + catalog.json in target/packages
cargo xtask new-plugin <id> [--name ..] [--ext ..] [--author ..]   # scaffold plugins/<id>
SAVEFORGE_PC_SAVE=<path> cargo test -p planet-crafter -- --ignored   # real-save test
```

The quality gate (CI):
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- the wasm clippy line in `.github/workflows/ci.yml`
- `cargo test --workspace`, `cargo deny check`, `cargo machete`

MSRV is 1.96 and is checked in CI.

## Dev mode

Debug builds default to:
- **Plugin directory:** `target/dev-plugins`.
- **Catalog URL:** `file://<data dir>/catalog.json`.

`cargo xtask package` writes `target/packages/catalog.json` with `file://` package URLs, which can be used as the catalog in Settings to try installs. Release builds use `<data dir>/plugins` and `RELEASE_CATALOG_URL`.

## Conventions

- **Errors:** library crates define `thiserror` enums (`CoreError`, `HostError`, `StoreError`). `anyhow` is used only in `xtask`.
- **Lints:** `unsafe_code` is forbidden, and `clippy::pedantic` runs at `-D warnings`, native and wasm.
  - Clippy rejects `assert!(x.is_empty())` and `assert!(!x.is_empty())`; use `assert_eq!` / `assert_ne!`.
  - Prefer `fold` + `write!` over `map(format!).collect()`.
- **Plugins:** never get filesystem, network or environment access. They take bytes and aux files from the host.
- **App updates:** `App::update` is a state machine.
  - Slow work runs through the `blocking(...)` helper, or `job_task` for store jobs with progress.
  - Every background result carries a request number, and stale results are ignored.
- **App tests:** tests drive `App::update` with hand-fed results; tasks are not executed. View tests use `iced_test::simulator`.
- **Tests:** shared helpers are in each crate's `tests/common/mod.rs`. Real-save tests are `#[ignore]` and read their path from the environment.
- **The store treats catalog data as untrusted:**
  - `https://`/`file://` only, no redirect downgrade
  - an `https://` catalog may only list `https://` packages
  - only `https://` links are opened
  - zip entries are checked for slip, symlinks and size

## Plugin authoring

See `docs/PLUGIN_AUTHORING.md`. Quick start: `cargo xtask new-plugin my-game`, then `cargo test -p my-game`, then `cargo xtask plugins -p my-game`.
