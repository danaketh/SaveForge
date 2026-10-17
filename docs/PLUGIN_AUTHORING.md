# Plugin Authoring Guide

A SaveForge plugin is a Rust crate compiled to a WebAssembly component (`wasm32-wasip2`). It depends only on `saveforge-sdk`. The app runs it in a sandbox with no file, network or environment access:
- **Input:** the save's bytes, plus any auxiliary files the manifest asks for.
- **Output:** a UI tree to render, and the edited bytes when the user saves.

There are two ways to build the editor:
- **The `form` helper:** describe the fields as a schema; the SDK renders it, applies edits by dot-path and validates them. This suits JSON-like saves.
- **A custom editor:** implement `view` and `update` yourself with the UI builders.

## 1. Scaffold

```bash
cargo xtask new-plugin my-game --name "My Game Save Editor" --ext .sav --author "You"
cargo test -p my-game
cargo xtask plugins -p my-game
cargo run -p saveforge-app
```

`new-plugin` creates `plugins/my-game/` with `Cargo.toml`, `manifest.json`, `src/lib.rs` (a form-based plugin) and `tests/my_game.rs`. It also adds the crate to the workspace and to the CI wasm lint. `--name`, `--ext` and `--author` are optional. The id must be lowercase letters, digits and single hyphens, starting with a letter.

`cargo xtask plugins` builds every plugin, or one with `-p`, into `target/dev-plugins/{id}/{version}/`, which debug builds of the app load.

## 2. `manifest.json`

From `plugins/planet-crafter/manifest.json`:

```json
{
  "id": "planet-crafter",
  "name": "Planet Crafter Save Editor",
  "version": "1.0.0",
  "author": "SaveForge Team",
  "description": "Edit saves for Planet Crafter.",
  "apiVersion": "0.1",
  "minHostVersion": "0.1.0",
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

The app checks these rules when it loads a plugin and when it installs one:

- `id` matches `^[a-z0-9]+(-[a-z0-9]+)*$`.
- `version`, `minHostVersion` and `maxHostVersion` are SemVer.
- `apiVersion` is compatible with the host's plugin API. Before 1.0 that means the same `major.minor`; currently `0.1`.
- The app's version must be at least `minHostVersion`, and at most `maxHostVersion` when that is set.
- `component` and `icon` are relative paths inside the plugin folder; `..` and absolute paths are rejected.
- `fileExtensions` is non-empty. Each entry starts with `.` and is compared case-insensitively.
- `defaultSaveDirectories` gives a starting folder per OS. A missing key or `null` means no default. The user can always set the folder in the app.
- `auxFiles[].id` values are unique. `maxBytes` defaults to 8 MB and is capped at 64 MB.

## 3. The `Plugin` trait

```rust
pub trait Plugin: 'static {
    type Doc: Document;

    /// `Some(priority)` when this plugin can open the file. `header` holds at
    /// most the first 512 bytes.
    fn detect(file_name: &str, header: &[u8]) -> Option<u32>;

    /// Parses the save. `aux` holds the manifest's aux files that were found.
    fn open(file_name: &str, bytes: Vec<u8>, aux: Vec<AuxFile>) -> Result<Self::Doc, String>;
}
```

`detect` is called only for files whose extension is in `fileExtensions`. When several plugins claim a file, the highest priority wins, and a tie asks the user. Match on something only your game's saves contain. `plugins/sample-stardew` uses priority 5 for a `"player"` key; `plugins/planet-crafter` uses 20 for its more specific `"terraTokens"` check.

Wire the plugin up once per crate with `export_plugin!(MyPlugin);`.

## 4. Schema-driven editors (`saveforge_sdk::form`)

`FormDocument<C: Codec>` keeps the save as a `serde_json::Value`, renders a `FormSchema` (each page becomes a tab), applies edits by dot-path, evaluates visibility rules, validates, and serializes through the codec. `JsonCodec` handles plain JSON files: it keeps key order and accepts a byte-order mark. For any other format, implement `Codec` (`decode(&[u8]) -> Result<Value, String>` and `encode(&Value) -> Result<Vec<u8>, String>`).

From `plugins/sample-stardew/src/lib.rs`:

```rust
impl Plugin for Stardew {
    type Doc = StardewDocument;

    fn detect(_file_name: &str, header: &[u8]) -> Option<u32> {
        header
            .windows(br#""player""#.len())
            .any(|w| w == br#""player""#)
            .then_some(PRIORITY)
    }

    fn open(
        _file_name: &str,
        bytes: Vec<u8>,
        _aux: Vec<AuxFile>,
    ) -> Result<StardewDocument, String> {
        StardewDocument::open(schema(), &bytes)
    }
}
```

and its schema, abridged:

```rust
FormSchema::new()
    .page(
        FormPage::new("Player")
            .section(
                FormSection::new()
                    .heading("Identity")
                    .field(
                        text("player.name", "Name")
                            .help("The farmer's name.")
                            .required()
                            .max_len(50),
                    )
                    .field(number("player.level", "Level").range(1.0, 100.0))
                    .field(checkbox("player.married", "Married"))
                    .field(text("player.spouse", "Spouse").required().visible_when(
                        VisibilityRule::new("player.married", VisibilityOp::Equals, true),
                    )),
            ),
    )
```

**Field kinds:**

| Builder | Edits |
|---|---|
| `text` | a string |
| `number` | a number |
| `checkbox` | a bool |
| `choice(key, label, &[(value, display)])` | one of a fixed set |
| `object(key, label, fields)` | a nested object |
| `list(key, label, item_label, fields)` | an array of objects, with add and remove |

**Modifiers:**
- `.help`, `.read_only`, `.required`, `.max_len` and `.pattern` (a regex)
- `.range`, `.min`, `.max` and `.decimals` for numbers
- `.visible_when(VisibilityRule::new(path, op, value))`, where `op` is `Equals`, `NotEquals`, `GreaterThan` or `LessThan`

A field with an invalid value shows its error under the field, and saving is refused until every field is valid.

## 5. Custom editors

Implement `Document` yourself when a form doesn't fit:

```rust
pub trait Document: 'static {
    fn view(&self) -> ui::UiTree;
    fn update(&mut self, event: ui::Event);
    fn is_dirty(&self) -> bool;
    /// Must not change the dirty state; the host calls `mark_saved` after the
    /// bytes are on disk.
    fn serialize(&self) -> Result<Vec<u8>, String>;
    fn mark_saved(&mut self);
}
```

**Building the view:** `view` returns a tree built with `saveforge_sdk::ui`:
- layout: `column`, `row`, `scrollable`, `section`, `tabs`, `divider`
- text: `text`, `muted`, `heading`, `error_text`
- fields: `text_input`, `number`, `checkbox`, `pick`
- `button`, and `list` (with `.removable` and `.add_options`)

**Handling events:** every widget with an `id` sends events back to `update`:
- `TextChanged`, `NumberChanged`, `Toggled`, `Picked`
- `Pressed`, `TabSelected`
- `ListAdd`, `ListRemove`

From `plugins/planet-crafter/src/document.rs`:

```rust
impl Document for PlanetCrafterDocument {
    fn view(&self) -> UiTree {
        let titles = TAB_TITLES.iter().map(|t| (*t).to_owned()).collect();
        let selected = self.tab.min(3);
        let page = match selected {
            0 => self.characters_page(),
            1 => self.terra_tokens_page(),
            2 => self.planet_page(),
            _ => self.save_info_page(),
        };
        ui::tabs(TABS, titles, selected, page).into_tree()
    }

    fn update(&mut self, event: Event) {
        let changed = match event {
            Event::TabSelected((_, index)) => {
                self.tab = index;
                false
            }
            // …
            Event::NumberChanged((id, value)) => self.set_number(&id, value),
            Event::TextChanged((id, value)) => self.set_text(&id, &value),
            Event::ListAdd((id, gid)) => self.add_item(&id, &gid),
            Event::ListRemove((id, index)) => self.remove_item(&id, index),
            Event::Toggled(_) | Event::Pressed(_) => false,
        };
        self.dirty |= changed;
    }
    // …
}
```

and one page:

```rust
ui::number(
    TERRA_TOKENS_ID,
    "Terra Tokens",
    float(tokens.get("terraTokens")),
)
.range(0.0, TERRA_TOKENS_MAX)
.into(),
```

**Contract rules:**
- **Tabs:** `tabs` has one child, the active page. Render only the selected tab's content.
- **Ids:** unique within one tree. The host keeps text and number buffers by id.
- **Size:** a tree may have at most 20,000 nodes. An invalid tree counts as a crash.
- **Number fields:** the host sends `NumberChanged` only for values that parse and fit the field's range and decimals.
- **Read-only fields:** never send events.
- **Dirty state:** `serialize` must not change it; the host calls `mark_saved` after the file is written.
- **Logging:** use `saveforge_sdk::{debug, info, warn, error}!`. Output goes to the app's log.

A trap, a call over its time limit (250 ms for `view`/`update`, 10 s for `open`/`serialize`/`detect`), or using more than 512 MB of memory crashes the plugin. The editor then shows the error and the file is left untouched. After three crashes in one session, the plugin is disabled until the app restarts.

## 6. Auxiliary files and path tokens

Plugins cannot read files themselves. To use one besides the save, such as a localization table, declare it in `auxFiles`. The host reads the first candidate path that exists and is at most `maxBytes`, and passes it to `open` as an `AuxFile { id, bytes }`:
- A missing optional file is left out.
- A missing required file stops the open with an error naming its `id`.

Paths in `candidates` and `defaultSaveDirectories` may use these tokens:

| Token | Meaning |
|---|---|
| `%APPDATA%` | Roaming AppData; `~/.config` on Linux; `~/Library/Application Support` on macOS |
| `%LOCALAPPDATA%` | Local AppData; `~/.local/share` on Linux; `~/Library/Application Support` on macOS |
| `%USERPROFILE%`, leading `~` | the user's home |
| `%TEMP%` | the temp directory |
| `{steam_app:ID}` | the install folder of Steam app `ID`, in any Steam library |
| `{steam_compat:ID}` | on Linux, the Proton prefix user folder of app `ID` |

A path whose token can't be resolved (for example, Steam is not installed) is treated as unresolved.

## 7. Testing

Plugins are ordinary crates, so `cargo test -p my-game` runs natively without the app. The scaffold's test opens a minimal save and checks the rendered fields. The existing plugins also round-trip the sample saves in `docs/sample-saves/`. Tests that need a real save read its path from an environment variable and are `#[ignore]`d by default: see `plugins/planet-crafter/tests/real_save.rs`.

## 8. Packaging and publishing

```bash
cargo xtask package -p my-game --base-url https://example.com/releases/
```

This writes `target/packages/my-game-0.1.0.zip` (`manifest.json`, the component, and the icon if the manifest names one) and a `catalog.json` entry with its SHA-256. Without `--base-url`, the entry points at the local zip, so you can try the install in the app's Store first: set the catalog URL in Settings to `target/packages/catalog.json` as a `file://` URL. Publish the zip, add the entry to your catalog, and see [CATALOG_SCHEMA.md](CATALOG_SCHEMA.md) for the format.
