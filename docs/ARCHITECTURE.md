# Architecture

## Crate map

| Crate | Role |
|---|---|
| `saveforge-core` | Settings, path tokens, Steam library lookup, save discovery, backup and atomic write, manifest validation. |
| `saveforge-plugin-api` | Host-side bindings of the WIT world in `wit/`, and the UI-tree types. |
| `saveforge-host` | One `wasmtime` engine per app, plugin loading, the plugin registry, detection, and one sandboxed instance per open document. |
| `saveforge-store` | Catalog fetch with an offline cache, downloads, SHA-256 verification, and package install, update and remove. |
| `saveforge-app` | The iced app: screens, theme, and the renderer that turns a plugin's UI tree into widgets. |
| `saveforge-sdk` | The guest crate plugins build on: the `Plugin` and `Document` traits, UI builders, the `form` helper, and `export_plugin!`. |
| `plugins/*` | `planet-crafter` and `sample-stardew`. |
| `xtask` | `cargo xtask plugins`, `package` and `new-plugin`. |

## Dependency rules

```
saveforge-app    → core, host, store
saveforge-host   → core, plugin-api
saveforge-store  → core, host
plugins/*        → saveforge-sdk only
saveforge-sdk    → (WIT file shared with plugin-api; no host crates)
```

`saveforge-plugin-api` and `saveforge-sdk` read the same `wit/` directory, so host and guest cannot drift apart.

## Runtime model

- **Engine and cache:** there is one `wasmtime::Engine` per app. Components are compiled once and cached on disk, keyed by the wasm and the wasmtime version.
- **One instance per document:** each open document gets its own store and component instance, and its state lives in plugin memory. Closing the document drops the instance.
- **Detection:** `detect` runs on short-lived instances.
- **Threads:**
  - `open` and `serialize` run off the UI thread as iced tasks, because a save can be several MB.
  - `view` and `update` run synchronously on the UI thread, so keystrokes are never dropped or reordered.
- **Guards:**
  - epoch interruption: 250 ms per `view`/`update`, 10 s per `open`/`serialize`/`detect`
  - linear memory capped at 512 MB per instance
  - after 3 traps in a session, a plugin is disabled until restart
- **Sandbox:** WASI is linked with an empty context: no files, no network, no environment. Plugins get bytes in and return bytes out; the host does all I/O. Guest stdout and stderr go to the log.

## Screens

`Screen` is `Home`, `Store`, `PluginDetail(id)`, `Settings` or `About`. The editor lives on `Home`: an open document stays open while you visit the other screens. The save list shows only on `Home`. Background results (detection, open, save, catalog loads, store jobs) carry a request number, and a result whose request is no longer current is ignored.

## Key flows

### Startup

1. Load `settings.json` from the data directory. A missing or broken file means defaults, plus a status-bar warning.
2. Scan `{pluginDir}/{id}/{version}/` off-thread; the highest version of each id wins. Each plugin is validated, checked for compatibility and compiled, then registered as loaded or failed.
3. With "check for updates on startup" on, fetch the catalog and compare it with the installed versions:
   - **Running:** the status bar shows `Checking for plugin updates…`.
   - **Updates found:** the info bar shows `Updates available for N plugin(s). Go to Store to update.`
   - **Failure:** the status bar shows `Update check failed. Retry in Settings.`

### Detect and open

1. Choosing a save calls `detect` on every loaded plugin whose extensions match. Each one gets the first 512 bytes.
2. The highest priority wins. On a tie, a dialog asks which plugin to use. No match is reported in the status bar.
3. The plugin's aux files are resolved, and `open` runs off-thread.
4. The editor shows the plugin's UI tree. Each edit becomes an event, then `update`, then a fresh `view`.

### Save with backup

1. `serialize` runs off-thread.
2. The original is copied to `{backupDir}/{pluginId}/{fileName}.{timestamp}.bak`. If the copy fails, the save stops.
3. The new bytes go to a temp file next to the save, which is flushed and renamed over the original.
4. If the write fails, the temp file is removed and the original is untouched.

### Store install

1. Download the package into `{pluginDir}/.staging`, hashing as it streams. Then check the SHA-256 against the catalog.
2. Extract into a staging directory, rejecting `..`, absolute paths, symlinks, and archives over 256 MiB.
3. The package manifest's `id` and `version` must match the catalog entry, and the plugin must be compatible with the host.
4. Move the old copy of the same version aside, then rename the new directory into `{pluginDir}/{id}/{version}`.
5. Load the plugin into the registry, with no restart.

Any failure leaves `.staging` empty and an existing install untouched. An update installs the new version and removes the old one only after the new one has loaded. Removing a plugin is refused while one of its saves is open.

### Catalog

- **Source and cache:** the catalog is fetched from `https://` or `file://`. A successful fetch is cached. A failed one falls back to the cache, and the status bar says the Store is offline.
- **Trust:** redirects may not leave `https://`. An `https://` catalog may only point at `https://` packages.
- **Links:** only `https://` links are opened in the browser.
