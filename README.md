# SaveForge

SaveForge is a desktop app for editing save files of single-player games. Each game is supported by a plugin: a sandboxed WebAssembly component that parses the save, describes an editor UI, and writes the save back. The app finds your saves, backs them up before every write, and installs plugins from a catalog. It runs on Windows, Linux and macOS, built with Rust and [iced](https://iced.rs).

## Requirements

- Rust stable, 1.96 or newer (`rustup update stable`)
- The WebAssembly target for plugins: `rustup target add wasm32-wasip2`

## Build and run

```bash
# Build the plugins into target/dev-plugins (debug builds load them from there)
cargo xtask plugins

# Run the app
cargo run -p saveforge-app
```

Debug builds use two development defaults:
- **Plugins:** loaded from `target/dev-plugins`.
- **Catalog:** the `file://` catalog at `catalog.json` in the app's data directory.

Release builds use `<data dir>/plugins` and the published catalog instead.

### Release build

```bash
cargo build --release -p saveforge-app
```

The binary is `target/release/saveforge` (`saveforge.exe` on Windows).

### Downloading a release

Each GitHub release has the app for Windows (`saveforge-windows-x64.exe`), Linux (`saveforge-linux-x64.tar.gz`) and macOS (`saveforge-macos-arm64.tar.gz`). Unpack the `.tar.gz` with `tar -xzf` and run `./saveforge`. The macOS binary is not signed. On first launch, right-click it and choose Open, or clear the quarantine flag with `xattr -d com.apple.quarantine saveforge`.

## Tests

```bash
cargo xtask plugins        # the host and app tests load the built components
cargo test --workspace
```

Real-save tests are ignored by default. Point them at a save to run them:

```bash
SAVEFORGE_PC_SAVE=/path/to/Standard-1.json cargo test -p planet-crafter -- --ignored
```

## Plugins

| Plugin | ID | Game | Status |
|---|---|---|---|
| Planet Crafter Save Editor | `planet-crafter` | [The Planet Crafter](https://store.steampowered.com/app/1284190/The_Planet_Crafter/) | Dev |
| StardewLike Save Editor | `stardew-like` | Sample / reference game | Dev |

The Planet Crafter plugin edits characters, inventories, Terra Tokens, terraforming levels and save info. It round-trips the save byte for byte apart from your edits. It reads item names from the game's localization file when Steam has the game installed.

## Plugin development

```bash
cargo xtask new-plugin my-game                 # scaffold plugins/my-game
cargo test -p my-game                          # native tests
cargo xtask plugins -p my-game                 # build into target/dev-plugins
cargo xtask package                            # zips + catalog.json in target/packages
```

`cargo xtask package` writes `target/packages/catalog.json` with `file://` package URLs. Point the app's catalog URL (Settings) at that file to try installs from the Store. Pass `--base-url https://…/` for a catalog to publish.

See [docs/PLUGIN_AUTHORING.md](docs/PLUGIN_AUTHORING.md) for the full guide.

## Quality gate

CI runs:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
# plus the wasm clippy line in .github/workflows/ci.yml (it lists every plugin)
cargo xtask plugins
cargo test --workspace
cargo deny check
cargo machete
```

Lints are `clippy::pedantic` at `-D warnings`, and `unsafe_code` is forbidden. A separate CI job checks the workspace on Rust 1.96. A `v*` tag runs `.github/workflows/release.yml`, which publishes the app binaries, the plugin zips and `catalog.json`.

## Project structure

```
crates/
  saveforge-core/        # settings, path tokens, Steam lookup, discovery, backup + atomic write
  saveforge-plugin-api/  # WIT bindings for the host, UI-tree types
  saveforge-host/        # wasmtime engine, plugin loading, registry, detection
  saveforge-store/       # catalog fetch/cache, download, SHA-256, install/remove, updates
  saveforge-app/         # the iced app: screens, theme, UI-tree renderer
sdk/saveforge-sdk/       # the guest SDK plugins build on
plugins/                 # planet-crafter, sample-stardew
fixtures/test-plugin/    # a component used by host and app tests
wit/                     # the plugin interface, shared by host and SDK
xtask/                   # cargo xtask plugins | package | new-plugin
docs/                    # architecture, plugin authoring, catalog schema, sample saves
```

## Further reading

- [Architecture](docs/ARCHITECTURE.md)
- [Plugin Authoring Guide](docs/PLUGIN_AUTHORING.md)
- [Catalog Schema](docs/CATALOG_SCHEMA.md)
