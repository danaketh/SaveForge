# SaveForge Rust/iced Rewrite — Design Spec

**Date:** 2026-10-10
**Status:** Draft — awaiting review
**Source app:** `../dotnet` (Avalonia 12 / .NET 10, ~6.4k LOC)

## 1. Goals

The Rust app **replaces** the .NET product. Feature parity is required, except where this spec explicitly changes behaviour.

| Goal | Consequence |
|---|---|
| Small native binary | Single executable, no runtime. Release profile: LTO, `codegen-units=1`, `strip`. |
| Learn Rust / iced | Idiomatic Rust and iced's Elm architecture over a 1:1 port of the MVVM structure. |
| Safer plugin model | Third-party plugins run sandboxed as WebAssembly components. No native plugin code. |
| Replace .NET version | All screens and flows of the .NET app exist in v1. |

### Non-goals (v1)

- Compatibility with .NET plugins or the .NET SDK (`ISaveDocument`, dot-path get/set, `EditorKind`).
- Migration of the .NET `settings.json`. Rust starts with fresh settings.
- Multiple open documents, file watching, backup pruning, package signatures, images in plugin UI, light theme, self-update of the host, host UI localization.

## 2. Decisions

| Topic | Decision |
|---|---|
| Plugin runtime | WebAssembly Component Model on `wasmtime`. Guests are built with `wit-bindgen` for `wasm32-wasip2`. |
| Plugin I/O | Bytes in, bytes out. Plugins have no filesystem, network, or env access. The host does all I/O. |
| Plugin UI | Elm-style UI inside the plugin. The plugin owns its state and exports `view()` (a tree of host widgets) and `update(event)`. The host has a single renderer. |
| Schema-driven plugins | A guest-side helper in `saveforge-sdk` builds the UI tree from a `FormSchema`. The host has no schema renderer. |
| Store | Ported as-is: catalog JSON, zip packages, SHA-256 verification, install/update/remove, startup update check. |
| First-party plugins | Not bundled. Planet Crafter is installed from the store like any plugin. |
| Platforms | Windows, Linux, macOS. |
| Visuals | Same layout and dark amber palette as .NET through a custom iced `Theme`. Uses the native OS title bar, not a custom one. |
| Repo | A single cargo workspace containing host, SDK, plugins, and `xtask`. |
| Extras ported | StardewLike sample plugin, plugin scaffolder, real-save tests, strict lint gate. |

## 3. Architecture

### 3.1 Workspace layout

```
rust/
  Cargo.toml                 # workspace, shared [lints], [profile.release]
  crates/
    saveforge-core/          # domain types, settings, path tokens, Steam lookup,
                             # save discovery, backup + atomic write, semver compat
    saveforge-plugin-api/    # WIT world, wasmtime host bindings, UI-tree types
    saveforge-host/          # wasmtime engine, plugin loading, registry, detection
    saveforge-store/         # catalog fetch/cache, download, sha256, install/remove, updates
    saveforge-app/           # iced binary: screens, theme, UI-tree renderer
  sdk/
    saveforge-sdk/           # guest crate: wit-bindgen glue, Plugin trait, builders, form helper
  plugins/
    planet-crafter/
    sample-stardew/
  xtask/                     # build/package plugins, scaffold new plugin
  docs/
```

### 3.2 Dependency rules

```
saveforge-app    → core, host, store
saveforge-host   → core, plugin-api
saveforge-store  → core, host
plugins/*        → saveforge-sdk only
saveforge-sdk    → (WIT file shared with plugin-api; no host crates)
```

`saveforge-plugin-api` and `saveforge-sdk` read the same `wit/` directory, so neither side can drift from the other.

### 3.3 Key crates

`iced`, `wasmtime` (component model, Cranelift), `wasmtime-wasi`, `wit-bindgen` (guest), `serde`, `serde_json` (`preserve_order`, `arbitrary_precision` where noted), `ureq` (rustls on ring, OS trust store via `rustls-platform-verifier`), `tokio` (iced executor), `sha2`, `zip`, `semver`, `directories`, `rfd`, `tracing`, `tracing-subscriber`, `tracing-appender`, `thiserror`, `anyhow` (xtask only), `tempfile` (store staging, tests).

Exact versions are pinned in the implementation plan.

### 3.4 Runtime model

- There is one `wasmtime::Engine` per app. Components are compiled once per plugin version and cached as `.cwasm` next to `plugin.wasm`. The cache is invalidated when the wasmtime version or the wasm hash changes.
- Each **open document** gets one `Store` and one component instance. Document state lives in plugin linear memory. Closing the document drops the instance.
- `detect` uses a short-lived instance per plugin. Instances are pooled per plugin for the duration of one detection pass.
- `open` and `serialize` run off the UI thread through iced `Task`s, because parsing a save can take multiple MB.
- `view` and `update` run **synchronously on the UI thread**. An async round-trip would drop or reorder keystrokes in text inputs.
- Guards:
  - epoch interruption with a 250 ms deadline per `view`/`update` call and 10 s per `open`/`serialize`/`detect`
  - `StoreLimits` capping linear memory at 512 MB per instance
- WASI p2 is linked with an empty context: no preopened dirs, no sockets, no env vars, no args. Guest stdout/stderr go to `tracing` at debug level.

## 4. Plugin Contract

### 4.1 Package

A zip file containing:

```
manifest.json
plugin.wasm
icon.png        (optional)
```

### 4.2 `manifest.json`

```json
{
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
    "windows": "%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter",
    "linux": "{steam_compat:1284190}/AppData/LocalLow/MijuGames/Planet Crafter",
    "macos": null
  },
  "auxFiles": [
    {
      "id": "localization",
      "optional": true,
      "maxBytes": 8388608,
      "candidates": [
        "{steam_app:1284190}/Planet Crafter_Data/Resources/LocalizationSpaceCrafter.txt"
      ]
    }
  ]
}
```

Validation rules, applied at load and at install:

- `id` matches `^[a-z0-9]+(-[a-z0-9]+)*$`.
- `version`, `minHostVersion`, and `maxHostVersion` are valid SemVer.
- `apiVersion` must be compatible with the host's WIT package version. Before 1.0, compatible means the same `major.minor`.
- The host version must satisfy `minHostVersion <= host` and `host <= maxHostVersion` when `maxHostVersion` is set.
- `component` and `icon` are relative paths that resolve inside the plugin dir. `..` and absolute paths are rejected.
- `fileExtensions` is non-empty. Each entry starts with `.` and is compared case-insensitively.
- `auxFiles[].id` values are unique. `maxBytes` defaults to 8 MB, with a hard cap of 64 MB.

### 4.3 Path tokens

Tokens are expanded by `saveforge-core`. Unknown tokens are left verbatim, as in .NET. If a token cannot be resolved, for example when Steam is not installed, the whole path counts as unresolved.

| Token | Windows | Linux | macOS |
|---|---|---|---|
| `%APPDATA%` | Roaming AppData | `$XDG_CONFIG_HOME` / `~/.config` | `~/Library/Application Support` |
| `%LOCALAPPDATA%` | Local AppData | `$XDG_DATA_HOME` / `~/.local/share` | `~/Library/Application Support` |
| `%USERPROFILE%`, leading `~` | user home | user home | user home |
| `%TEMP%` | temp dir | temp dir | temp dir |
| `{steam_app:ID}` | install dir of app ID across all Steam libraries | same | same |
| `{steam_compat:ID}` | — (unresolved) | `<library>/steamapps/compatdata/ID/pfx/drive_c/users/steamuser` | — |

Steam libraries are found by locating the Steam root and parsing `steamapps/libraryfolders.vdf`. The `appmanifest_ID.acf` file gives `installdir`. The Steam root is:

- Windows: the registry value `HKCU\Software\Valve\Steam\SteamPath`, falling back to `%ProgramFiles(x86)%\Steam`
- Linux: `~/.steam/steam`, then `~/.local/share/Steam`
- macOS: `~/Library/Application Support/Steam`

`defaultSaveDirectories` is looked up for the current OS. A missing key or `null` means the plugin has no default on that OS.

### 4.4 WIT world

```wit
package saveforge:plugin@0.1.0;

interface ui {
  /// WIT has no recursive types: the tree is a flat arena. nodes[0] is the root.
  type node-id = u32;

  record ui-tree { nodes: list<node> }
  record node { widget: widget, children: list<node-id> }

  enum text-style { body, muted, heading, error }
  record text-span { text: string, style: text-style }

  record tabs { id: string, titles: list<string>, selected: u32 }

  record text-input { value: string, placeholder: option<string>, max-len: option<u32> }
  record number-input { value: f64, min: option<f64>, max: option<f64>, decimals: u8, step: option<f64> }
  record pick-option { value: string, display: string }
  record pick-list { options: list<pick-option>, selected: option<string>, placeholder: option<string> }

  variant input {
    text(text-input),
    number(number-input),
    checkbox(bool),
    pick(pick-list),
  }

  record field {
    id: string,
    label: string,
    help: option<string>,
    error: option<string>,
    read-only: bool,
    input: input,
  }

  enum button-style { primary, secondary, danger }
  record button { id: string, label: string, style: button-style, enabled: bool }

  record list-spec {
    id: string,
    /// Shows a remove button on each row (children are rows).
    removable: bool,
    /// When set, renders a pick-list + "Add" button under the rows.
    add-options: option<pick-list>,
  }

  variant widget {
    column,
    row,
    scrollable,
    divider,
    section(string),
    tabs(tabs),
    text(text-span),
    field(field),
    button(button),
    %list(list-spec),
  }

  variant event {
    text-changed(tuple<string, string>),
    number-changed(tuple<string, f64>),
    toggled(tuple<string, bool>),
    picked(tuple<string, string>),
    pressed(string),
    tab-selected(tuple<string, u32>),
    list-add(tuple<string, string>),
    list-remove(tuple<string, u32>),
  }
}

interface editor {
  use ui.{ui-tree, event};

  record aux-file { id: string, bytes: list<u8> }

  resource document {
    view: func() -> ui-tree;
    update: func(ev: event);
    is-dirty: func() -> bool;
    /// Pure: must not change dirty state. Err blocks the save and is shown to the user.
    serialize: func() -> result<list<u8>, string>;
    /// Called by the host only after the bytes were durably written; clears dirty state.
    mark-saved: func();
  }

  /// Some(priority) if the plugin can open the file; higher wins.
  detect: func(file-name: string, header: list<u8>) -> option<u32>;

  open: func(file-name: string, bytes: list<u8>, aux: list<aux-file>) -> result<document, string>;
}

interface host {
  enum level { trace, debug, info, warn, error }
  log: func(level: level, msg: string);
}

world plugin {
  import host;
  export editor;
}
```

### 4.5 Contract rules

- **Tree shape:** `tabs` has one child, the active page. The plugin renders only the selected tab's content. `list` children are its rows, and each row is any widget subtree. A node index out of range, a cycle, or a node with more than one parent makes the tree invalid. The host then shows "Plugin returned an invalid UI" and treats it as a trap (§7).
- **Tree size cap:** 20,000 nodes. A larger tree is invalid.
- **Number input:** iced has no numeric widget, so the host keeps a text buffer per field `id`. `number-changed` is emitted only when the buffer parses as `f64`, is within `min`/`max`, and has no more than `decimals` fractional digits. Otherwise the host shows an inline error and the plugin is not called. The buffer resets to the plugin's value whenever the field's `value` changes in a new tree.
- **Text input:** `text-changed` is emitted on every edit. `max-len` is enforced by the host.
- **IDs:** Widget `id`s are unique within one tree. Host-side buffers are keyed by `id`, and buffers whose `id` disappears from the tree are dropped.
- **Read-only fields** render as disabled and never emit events.
- **`detect`** receives up to the first 512 bytes. The host only calls it for files whose extension is in `fileExtensions`.
- **`open`** receives aux files that were found and within `maxBytes`. A missing optional aux file is omitted. A missing required aux file aborts the open with an error naming the file id, and the plugin is not called.

### 4.6 `saveforge-sdk` (guest)

- `Plugin` trait with `detect` and `open`. `Document` trait with `view`, `update`, `is_dirty`, and `serialize`. An `export_plugin!` macro wires up `wit-bindgen`.
- Builders for the UI tree, e.g. `col![section("Stats", col![number("health", "Health", v).range(0.0, 100.0)])]`. They produce the flat arena.
- `log::{info, warn, …}` macros forwarding to `host.log`.
- `form` module, the port of `SaveForge.Forms` + `SDK.Schema`:
  - `FormSchema`, `FormPage`, `FormSection`
  - field kinds: `Text`, `Numeric`, `Checkbox`, `Enum`, `ReadOnly`, `List`, `ObjectSection`
  - `VisibilityRule` with `Equals`, `NotEquals`, `GreaterThan`, `LessThan`
  - `FormDocument<C: Codec>`, which keeps a `serde_json::Value`, renders the schema (pages become tabs), applies events by dot-path, evaluates visibility, validates `required` / `max_length` / `regex` / numeric range, and serializes through the plugin-provided `Codec`.

## 5. Application

### 5.1 Shell

```
┌──────────────────────────── info bar (dismissable) ────────────────────────────┐
├──────┬──────────────┬──────────────────────────────────────────────────────────┤
│ rail │ save list    │ main pane                                                │
│ 68px │ 280px        │ Home · Editor · Store · PluginDetail · Settings · About  │
│      │ (Home/Editor)│                                                          │
│ ⚙ ℹ 🛒│              │                                                          │
├──────┴──────────────┴──────────────────────────────────────────────────────────┤
│ status bar 28px                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

- `Screen` enum: `Home | Editor | Store | PluginDetail(PluginId) | Settings | About`.
- `Editor` is `Home` with an open document; visiting Store, Settings or About keeps the document open.
- The rail has plugin icons on top. Each shows the manifest icon or, if there is none, the first letter of the name. Store, Settings, and About sit at the bottom. The selected plugin gets an amber indicator bar. Failed plugins are greyed out with the load error in a tooltip.
- Window: native decorations, 1200×750 default, 900×600 minimum, centered.
- The theme is a custom iced `Theme` with the palette ported from `Themes/Colors.axaml` and `DarkTheme.axaml`.

### 5.2 Flows

**Startup**
1. Load `settings.json` from the app data dir (`directories::ProjectDirs`). If it is missing or invalid, use defaults and log a warning.
2. Scan `{pluginDir}/{id}/{version}/manifest.json`. If several versions of one id exist, the highest SemVer wins.
3. For each plugin: validate the manifest, check compat, and compile or load `.cwasm` off-thread. Register it as `Loaded` or `Failed(reason)`.
4. If `checkUpdatesOnStartup` is on, the update check runs. The status bar shows "Checking for plugin updates…". If updates exist, the info bar shows "Updates available for N plugin(s). Go to Store to update." If there are none, the status bar is cleared. On error it shows "Update check failed. Retry in Settings."

**Home (save discovery)**
- Selecting a plugin resolves its save directory in this order: `settings.pluginSaveDirectories[id]`, then the manifest default for the current OS, then token expansion.
- Panel states:
  - *path not set*, when the path is unset, empty, unresolved, or the directory does not exist
  - *editing path*: text input, browse (`rfd` folder picker), ✓, ✕, and a live resolved-path hint
  - *list*: plugin name, raw path, ✏️ button, resolved path, save count, and rows showing file name, modified date, and human-readable size
  - *empty*: "No saves found"
- Discovery is recursive and filtered by `fileExtensions`, sorted by modified time descending. Per-entry errors are skipped. It runs off-thread and has a refresh button.
- Confirming a path saves it to settings and rescans. If the settings write fails, a warning goes to the status bar and the in-memory state is kept.
- The rail subtitle shows the save count, "path not set", or "load failed".

**Open**
1. Read the file. Call `detect` on every loaded plugin whose extensions match.
2. The highest priority wins. On a tie, a modal asks the user to pick a plugin. If nothing matches, the status bar shows "No plugin can open <file>".
3. Resolve aux files and call `open` off-thread. While it runs, show a loading spinner in the main pane.
4. On success, show the editor (on `Home`) with the live instance. On `Err`, show the error in the status bar and stay on Home.
5. Only one document is open at a time. Opening another while the current one is dirty triggers the unsaved-changes guard.

**Edit**
- An iced message maps to a plugin `event`, then `update`, then `view`, then a rebuilt `Element`. No tree diffing is needed.
- The window title is `SaveForge — <file>`, with a `•` suffix when `is-dirty`.
- Toolbar: Save (Ctrl/Cmd+S) and Close.

**Save**
1. Call `serialize` off-thread. On `Err`, show it in the status bar and stop.
2. Copy the original to `{backupDir}/{pluginId}/{fileName}.{yyyy-MM-ddTHH-mm-ssZ}.bak`. If the backup fails, abort the save.
3. Write `{fileName}.saveforge.tmp` in the same directory, `fsync`, then rename it over the original. Windows uses replace semantics.
4. If the write or rename fails, delete the tmp file, leave the original untouched, and show the error.
5. On success the host calls `mark-saved` and the status bar shows "Saved. Backup: <path>". Because `serialize` is pure, a failed write leaves the document dirty.

**Unsaved-changes guard** (new; .NET has none)
- Shown when closing the editor, opening another save, switching plugin, or closing the window while dirty.
- Offers Save / Discard / Cancel.
- Window close uses iced's close-request event.

**Store**
1. Fetch the catalog from `catalogUrl` (`https://` or `file://`). On success, write `catalog-cache.json`. On failure, fall back to the cache and show "offline" in the status bar.
2. Show the list with a search box matching name, description, tags, and author. Entries are marked *Installed*, *Update available*, or *Incompatible* (host version outside range).
3. The detail page shows name, author, description, version, tags, repository link, and Install / Update / Remove with a progress bar.
4. Install:
   - download to a temp file
   - verify SHA-256 against the catalog
   - extract into a staging dir `{pluginDir}/.staging/{uuid}`, rejecting entries with `..` or absolute paths (zip-slip)
   - validate that the manifest `id`/`version` match the catalog entry
   - atomically move to `{pluginDir}/{id}/{version}`
   - load into the registry with no restart

   Any failure removes the staging dir and temp file.
5. Update runs Install for the new version, then removes the old version dir.
6. Remove unloads the plugin and deletes `{pluginDir}/{id}`. It is blocked with a message while that plugin has an open document.

**Settings**
- Fields: catalog URL, plugin directory, backup directory, check updates on startup, and a "Check now" button.
- Saving updates the existing settings struct in place, so `pluginSaveDirectories` is preserved.
- Changing the plugin directory takes effect after restart. The UI says so.

**About**
- App version, wasmtime version, links, licenses (generated by `cargo about` or a static list).

### 5.3 Settings file

```json
{
  "catalogUrl": "https://raw.githubusercontent.com/YOUR_ORG/YOUR_REPO/main/catalog.json",
  "pluginDirectory": "<data_dir>/plugins",
  "backupDirectory": "<data_dir>/backups",
  "checkUpdatesOnStartup": true,
  "pluginSaveDirectories": { "planet-crafter": "%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter" }
}
```

Debug builds default `pluginDirectory` to `<workspace>/target/dev-plugins` and `catalogUrl` to `file://<data_dir>/catalog.json`.

### 5.4 Fixes over .NET behaviour

| .NET issue | Rust behaviour |
|---|---|
| Editor backup hardcoded `%APPDATA%/SaveForge/backups/editor`, ignoring `BackupDirectory` | Uses `backupDirectory` setting |
| Settings save rebuilt `AppSettings`, dropping `PluginSaveDirectories` and `CheckUpdatesOnStartup` | In-place update |
| Zip extracted straight into the target dir; partial installs possible | Staging dir and atomic move |
| Planet Crafter re-serialized typed models; unknown fields outside `WorldObjects` could be lost | Lossless round-trip (§6) |
| Localization probed hardcoded `C:/D:/E:/F:` paths | Steam library discovery |
| Planet Crafter default save dir was `%APPDATA%/MijuGames/The Planet Crafter/Saves`, where the game does not save | `%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter` (checked against game version 2.008) |
| No unsaved-changes guard | Guard modal |

## 6. Plugins

### 6.1 Planet Crafter (`plugins/planet-crafter`)

- **Detection:** extension `.json` and `"terraTokens"` within the first 512 bytes give priority 20. Otherwise no match.
- **Format:** groups are separated by `@` and objects within a group by `|`, with optional whitespace around them. A save may start with a UTF-8 byte order mark (current game versions write one). `@` and `|` inside JSON strings are text, so the file is scanned object by object rather than split. A save needs at least 11 groups. Groups the editor reads:

  0. TerraTokens (single)
  1. Planets (one object per planet, e.g. `Prime`, `Humble`)
  2. Players
  3. WorldObjects
  4. Inventories
  5. Stats (single)
  6. Messages
  7. StoryEvents
  8. DisplayInfo (single)

  The groups after DisplayInfo differ between game versions: 1.x saves have TerrainLayers, PlanetInstances and a trailing empty group; 2.008 saves have planet instances at index 9 and an empty group 10. They are kept verbatim and never interpreted.

- **Lossless round-trip.** Each object keeps its original text and is stored as a `serde_json` map with `preserve_order` and `arbitrary_precision`. Only objects that an edit changed are re-serialized (compact, key order and number text kept); the rest are written from their original text. The exact text between objects (whitespace, separators, BOM) is recorded and replayed. Typed accessors (e.g. `player.health()`) read and write the underlying values. **Invariant:** `serialize(parse(x)) == x` byte-for-byte for an unmodified save.
- **UI** (parity with `PlanetCrafterEditorView.axaml`). The JSON key is in parentheses.

  | Tab | Field | Key | Input | Constraints |
  |---|---|---|---|---|
  | Characters | Character | `players[]` | pick-list | display `name`, or `name (Host)` when `host` |
  | Characters / Stats | Health | `playerGaugeHealth` | number | 0–100, 1 decimal |
  | | Oxygen | `playerGaugeOxygen` | number | 0–370, 1 decimal |
  | | Thirst | `playerGaugeThirst` | number | 0–100, 1 decimal |
  | | Toxic | `playerGaugeToxic` | number | 0–100, 1 decimal |
  | Characters / Equipment | items of inventory `equipmentId` | list | removable, add from known items |
  | Characters / Inventory | items of inventory `inventoryId` | list | removable, add from known items |
  | Terra Tokens | Terra Tokens | group 0 `terraTokens` | number | ≥ 0, 0 decimals |
  | | All-Time Terra Tokens | group 0 `allTimeTerraTokens` | number | read-only |
  | | Unlocked Groups | group 0 `unlockedGroups` | text | read-only |
  | Planet State | Oxygen / Heat / Pressure / Plants / Insects / Animals, one section per planet titled with its `planetId` | group 1 `unit{Oxygen,Heat,Pressure,Plants,Insects,Animals}Level` | number | ≥ 0, 2 decimals |
  | Save Info | Display Name | group 8 `saveDisplayName` | text | max 64 |
  | | Planet ID | group 8 `planetId` | text | read-only |
  | | Crafted Objects | group 5 `craftedObjects` | text | read-only |

- **Inventory linkage** (port of `PlayerViewModel`):
  - Inventory items: find the inventory whose `id` equals the player's `inventoryId` (or `equipmentId`). Parse its `woIds` by splitting on `,` and `|`, dropping non-numeric entries. Look up each id in WorldObjects and skip any that are missing.
  - Add: create a new WorldObject `{ "id": max(existing ids)+1, "gId": <selected> }`, append it to group 3, and append its id to `woIds`. `woIds` is rewritten comma-joined.
  - Remove: remove the id from `woIds` only. The WorldObject stays, as in .NET.
  - **Open question for implementation:** `max+1` ids and orphaned WorldObjects are carried over from .NET unchanged. Before shipping, check them against real saves, since the game may reserve id ranges.
- **Localization:** the aux file `localization`. Parse `GROUP_NAME_{GId}` keys and take the English (first) value. Known items are sorted by display name, case-insensitively. Without the file, raw GIds are shown, and the add pick-lists offer only GIds already present in the save. Game version 2.008 no longer ships `LocalizationSpaceCrafter.txt` (names live in the Unity assets), so in practice the raw-GId fallback is what users see; the aux file stays optional for older installs.

### 6.2 Sample Stardew (`plugins/sample-stardew`)

A port of `SaveForge.SamplePlugin.StardewLike` using `saveforge-sdk::form`. It shows the schema pattern, including a `VisibilityRule`. The default save dir is `%APPDATA%/StardewValley/Saves` on Windows, `~/.config/StardewValley/Saves` on Linux and macOS.

## 7. Error Handling

- **Error types:** each library crate defines a `thiserror` enum: `CoreError`, `HostError`, `StoreError`. The app maps them to user-facing strings. `anyhow` is used only in `xtask`.
- **Plugin trap, epoch timeout, memory limit, or invalid UI tree:**
  - drop the instance
  - the editor shows "Plugin crashed: <reason>" with a Back button
  - the file on disk is untouched
  - after 3 traps in one session the plugin is marked `Failed` until restart
- **`open` / `serialize` returning `Err`:** shown to the user. On `serialize` errors the document stays open.
- **Background tasks** (discovery, catalog fetch, update check) never block the UI. Failures go to the status bar, and the catalog falls back to its cache.
- **Logging:** `tracing` with a daily rolling file in `<data_dir>/logs`, kept for 7 days. Debug builds also log to stderr.
- **Panics:** a panic hook writes `<data_dir>/crash.log` before exit. The release profile uses `panic = "abort"`.

## 8. Testing

| Layer | Approach |
|---|---|
| `saveforge-core` | Unit tests: path tokens per OS (`cfg`), VDF/ACF parsing with fixtures, discovery on `tempfile` trees (recursion, extension filter, missing dir, sort order), backup + atomic write including a simulated rename failure, settings round-trip and defaults on corrupt file, semver compat matrix, manifest validation. |
| `saveforge-store` | Catalog parsing, SHA-256 check, install/update/remove against a `file://` catalog with fixture zips, staging cleanup on bad hash / bad zip / zip-slip / id mismatch, offline cache fallback. |
| `saveforge-host` | Fixture components built by `xtask`: well-behaved, trapping, infinite loop (epoch), memory hog (limits), wrong `apiVersion`, invalid UI tree. Detection priority and tie handling. |
| `saveforge-sdk` | Tree builder correctness, `form` rendering, events by dot-path, `VisibilityRule` (port of `VisibilityRuleTests`), validation. |
| Plugins | Native `cargo test` on parser and model logic. Planet Crafter byte-identical round-trip on `docs/sample-saves/PlanetCrafter-Standard-1.json`, plus edit → serialize → parse assertions. |
| Real saves | `#[ignore]` tests reading paths from env (`SAVEFORGE_PC_SAVE`, …), run with `cargo test -- --ignored`. |
| App | Unit tests on `update()` state transitions (screen nav, path editing, dirty guard, number buffer) without rendering. No GUI snapshot tests in v1. |

The .NET test files (`tests/SaveForge.Tests/*.cs`) are the checklist of behaviours to cover.

## 9. Quality Gate & CI

- **Workspace `[lints]`:**
  - `clippy::all` and `clippy::pedantic` at warn, with a short documented allow-list
  - `unsafe_code = "forbid"`
  - CI runs `cargo clippy --all-targets -- -D warnings`
- **Other checks:** `cargo fmt --check`, `cargo deny check` (licenses, advisories, bans), `cargo machete`.
- **GitHub Actions matrix** on `windows-latest`, `ubuntu-latest`, `macos-latest`: fmt → clippy → `cargo xtask plugins` (`wasm32-wasip2`) → `cargo test`.
- **Release workflow:** per-OS `cargo build --release` artifacts, plugin zips from `cargo xtask package`, and generated catalog entries with SHA-256.
- **Release profile:** `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`, `opt-level = 3`.

## 10. Tooling (`xtask`)

| Command | Purpose |
|---|---|
| `cargo xtask plugins [-p <id>]` | Build plugins for `wasm32-wasip2` (release) and copy `manifest.json` + `plugin.wasm` + icon into `target/dev-plugins/{id}/{version}/`. Replaces `CopyToDevPlugins`. |
| `cargo xtask package [-p <id>]` | Build plugin zips into `target/packages/` and print catalog entries with SHA-256. |
| `cargo xtask new-plugin <id>` | Scaffold `plugins/<id>/` with `Cargo.toml`, `manifest.json`, a `lib.rs` using `saveforge-sdk::form`, and one test. Adds it to the workspace members. Replaces `SaveForge.NewPlugin`. |

## 11. Docs to port

- `README.md`: build, run, test, plugin dev workflow.
- `docs/ARCHITECTURE.md`: §3, §5.2.
- `docs/PLUGIN_AUTHORING.md`: rewritten for the Rust guest SDK, covering the form helper and custom `view`/`update`.
- `docs/CATALOG_SCHEMA.md`: unchanged schema. Package contents updated (§4.1).
- `CLAUDE.md`: Rust conventions and commands.

## 12. Suggested milestone order

1. Workspace skeleton, lints, CI; `saveforge-core` (settings, tokens, Steam, discovery, backup/atomic write).
2. WIT + `saveforge-plugin-api` + `saveforge-sdk` builders; `saveforge-host` with fixture plugins.
3. `saveforge-app` shell, theme, Home flow, UI-tree renderer, editor + save + guard.
4. Sample Stardew plugin + `form` helper.
5. Planet Crafter plugin with lossless round-trip.
6. `saveforge-store` + Store/PluginDetail/Settings/About screens.
7. `xtask package`/`new-plugin`, release workflow, docs.
