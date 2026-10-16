# Milestone 5: Planet Crafter Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Planet Crafter save editor plugin. An unmodified save is written back byte for byte. Edits change only the objects they touch, and every other number keeps its exact text. Feature parity with the .NET editor: characters with gauges and inventories, Terra Tokens, planet state, and save info.

**Architecture:** One wasm plugin crate in three layers:
- **`format`:** a lossless scanner for the `@`/`|` group format. It keeps every byte between objects and the original text of every object the user doesn't edit.
- **`save`:** typed access plus the inventory ↔ world-object linkage ported from .NET.
- **`document`:** the custom UI tree, built directly from the SDK `ui` builders (not `form`, because inventories and per-planet levels don't fit a schema).

The host is unchanged. An app test runs the real component from `target/dev-plugins`.

**Tech Stack:** Rust 1.99 (edition 2024). `serde_json` with `arbitrary_precision` and `preserve_order` (plugin), and its `StreamDeserializer::byte_offset` to find where each object ends. Existing: `saveforge-sdk` (`Plugin`, `Document`, `ui`, `AuxFile`, `export_plugin!`), `saveforge-app` test helpers, `cargo xtask plugins`.

**Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md` (§4.2 manifest, §4.3 path tokens, §6.1 Planet Crafter, §8 testing). Task 6 corrects §6.1 to the save layout of the current game; see "Real saves" below.

**Prior milestones:** M1–M4 are merged. The APIs used here:
- `saveforge_sdk::{Plugin, Document, AuxFile { id, bytes }, export_plugin!}`
- `saveforge_sdk::ui::{self, El, Event, PickOption, UiTree, Widget, FieldSpec, Input}`. Builders: `column`, `section`, `tabs`, `text`, `muted`, `number`, `text_input`, `pick`, `option`, `list(..).removable(..).add_options(options, placeholder)`, and `FieldEl::{range, min, decimals, read_only, max_len}`.
- Events: `TabSelected`, `Picked`, `NumberChanged`, `TextChanged`, `ListAdd((list id, picked value))`, `ListRemove((list id, row))`, `Toggled`, `Pressed`.
- `saveforge_app::{documents::{detect_file, open_document, save_document, DetectOutcome}, editor::Editor, render::{render, UiMessage}}`
- `saveforge_host::{Event, PluginRegistry}`, and `common::World` in the app tests
- `cargo xtask plugins`, which builds every `plugins/*` crate into `target/dev-plugins/{id}/{version}/` and copies `manifest.json` and the icon

**Real saves (found while drafting, 2026-10-16).** Two real saves from game version 2.008 (`%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter/Standard-1.json`, about 1 MB and 15,329 world objects) differ from the spec and from the old `.NET` sample:
- **Byte order mark:** the file starts with a UTF-8 BOM.
- **Planets:** the Planet group holds one object per planet (`Prime`, `Humble`).
- **Groups after `DisplayInfo`:** they changed between versions. The old sample has 12 groups and these saves have 11, with planet instances at index 9.
- **Save folder:** saves live in `LocalLow`, not in the spec's `%APPDATA%/MijuGames/The Planet Crafter/Saves`.
- **Localization:** the install (`I:/Games/SteamLibrary/steamapps/common/The Planet Crafter`) has no `LocalizationSpaceCrafter.txt`, so the optional localization aux file is not found. The plugin falls back to raw gIds.

This plan follows the real saves; Task 6 brings the spec in line. These saves hold other players' names and Steam ids, so they are never committed. The ignored `real_save` tests read them from `SAVEFORGE_PC_SAVE`.

**Verified draft:** every code block was compiled, linted (clippy pedantic `-D warnings`, native and `wasm32-wasip2`) and tested in a scratch worktree of `main` before this plan was written. That includes the byte-for-byte round-trip of both real saves via the ignored tests.

## Global Constraints

- **wasmtime:** `wasmtime` and `wasmtime-wasi` 49.0.3 (Task 1, Step 2), so `cargo deny` passes.
- **Crate:** `plugins/planet-crafter`, package `planet-crafter`, `crate-type = ["cdylib", "rlib"]`. Dependencies: `saveforge-sdk`, and `serde_json` with `features = ["arbitrary_precision", "preserve_order"]`.
  - Cargo feature unification turns these features on for every crate built in the same `cargo` run (the SDK and `sample-stardew` included). The SDK and Stardew tests must pass both alone and in `cargo test --workspace`; Task 6 checks both.
- **Lossless invariant:** `serialize(parse(x)) == x` byte for byte for an unmodified save (spec §6.1).
  - An object is re-serialized (compact, keys in original order, numbers in original text) only after `Object::set` changed it.
  - New objects are appended with the group's existing separator (`|\n` when the group has none to copy).
- **Format rules:**
  - Groups are separated by `@`; objects within a group by `|`.
  - Allowed between objects: spaces, tabs, `\r`, `\n`, one `|` between two objects, and a leading UTF-8 BOM.
  - `@` and `|` inside JSON strings are text.
  - There must be at least 11 groups. `TerraTokens` (0), `Stats` (5) and `DisplayInfo` (8) hold exactly one object each. `Planets` (1) holds one per planet.
  - Groups after `DisplayInfo` are kept, never interpreted.
- **Error texts (exact):**

  | Condition | Message |
  |---|---|
  | Bytes aren't UTF-8 | `the save is not valid UTF-8` |
  | Fewer than 11 groups | `expected at least 11 groups separated by '@', found {n}` |
  | Wrong object count in a single-object group | `{Group} must hold exactly one object, found {n}` |
  | Object isn't valid JSON | `{Group} object {k} is not valid JSON: {serde error}` |
  | Stray character | `unexpected {char:?} at byte {i} in {Group}` |
  | Group ends with a separator | `{Group} ends with '\|'` |

  Groups past `DisplayInfo` are named `group {i}` in these messages.
- **Manifest:**
  - id `planet-crafter`, name `Planet Crafter Save Editor`, version `1.0.0`, `apiVersion` `0.1`, `minHostVersion` `0.1.0` (the host is 0.1.0 until the M7 release), icon `icon.png`, extensions `[".json"]`
  - default saves: Windows `%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter`, Linux `{steam_compat:1284190}/AppData/LocalLow/MijuGames/Planet Crafter`, macOS `null`
  - optional aux file `localization` (8 MB) from `{steam_app:1284190}/Planet Crafter_Data/Resources/LocalizationSpaceCrafter.txt`
- **Detection:** a `.json` extension (any case) plus `"terraTokens"` within the first 512 bytes gives priority **20**.
- **Widget ids:**

  | Id | Widget |
  |---|---|
  | `tabs` | page tabs: Characters, Terra Tokens, Planet State, Save Info |
  | `character` | character pick-list, value = player index |
  | `player.{playerGaugeHealth,playerGaugeOxygen,playerGaugeThirst,playerGaugeToxic}` | gauges, 0–100 (Oxygen 0–370), 1 decimal |
  | `equipment`, `inventory` | item lists |
  | `tokens.terraTokens` | number, ≥ 0, whole |
  | `tokens.allTimeTerraTokens`, `tokens.unlockedGroups` | read-only |
  | `planet.{index}.unit{Oxygen,Heat,Pressure,Plants,Insects,Animals}Level` | planet levels, ≥ 0, 2 decimals, one section per planet titled with its `planetId` |
  | `info.saveDisplayName` | text, max 64 characters |
  | `info.planetId`, `stats.craftedObjects` | read-only |
- **Inventory linkage (.NET parity):**
  - `woIds` is split on `,` and `|`; entries that aren't non-negative integers are dropped, and ids with no world object are skipped.
  - Add creates `{"id": max + 1, "gId": …}` (or id 1 when there are no world objects) and rewrites `woIds` comma-joined.
  - Remove deletes the first occurrence of the row's id and leaves the world object in place.
  - List rows map to ids through the displayed (filtered) list.
- **Edits:** numbers outside the range, non-finite numbers, read-only ids and unknown ids are ignored. Setting the value already stored is not an edit and leaves the original text alone (`100.0` stays `100.0`).
- **Lints:** clippy 1.99 rejects `assert!(x.is_empty())` and `assert!(!x.is_empty())`; use `assert_eq!` / `assert_ne!`. `float_cmp` needs an `#[allow]` with a reason.
- **Branch:** work on `feat/m5-planet-crafter`, never on `main`.

## Review Focus

Inputs a real save can contain that the spec implies, each pinned by a test:

1. **`@` or `|` inside a string** (a display name like `Base @ Lake | 2`). Expected: part of the text, not a separator; the save round-trips. Test: `separators_inside_strings_are_text` (Task 1).
2. **Editing one field of an object that also holds 17-digit floats or Steam ids above 2^53.** Expected: every other number keeps its exact text. Tests: `large_ids_keep_their_digits_after_an_edit` and `edited_object_is_rewritten_and_keeps_other_number_text` (Task 1).
3. **`woIds` with junk entries, ids with no world object, or pipes.** Expected: rows show only real items, and removing row *n* removes the item shown at row *n*. Tests: `inventory_items_follow_wo_ids_and_skip_unknown_entries` and `wo_ids_split_on_commas_and_pipes` (Task 2), `adding_and_removing_items` (Task 3).
4. **A save from the current game version** (BOM, several planets, 11 groups with no trailing empty group). Expected: opens, edits per planet, round-trips. Tests: `saves_without_the_trailing_group_parse` and `odd_whitespace_and_byte_order_mark_round_trip` (Task 1), `planet_levels_edit_each_planet` (Task 3), `current_game_saves_with_a_byte_order_mark_open_and_round_trip` (Task 4).
5. **A file that isn't a Planet Crafter save after all** (not UTF-8, truncated, a JSON array). Expected: a clear open error, never a panic in the guest. Tests: `non_utf8_is_rejected` (Task 2), `too_few_groups_are_rejected` and `malformed_text_is_rejected_with_its_location` (Task 1), `opening_a_broken_save_reports_why` (Task 4).

---

## File Structure

```
rust/
  Cargo.toml                                    # + member plugins/planet-crafter
  .github/workflows/ci.yml                      # wasm clippy includes planet-crafter
  docs/sample-saves/PlanetCrafter-Standard-1.json   # copied from ../dotnet/docs/sample-saves
  docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md   # §4.2, §5.3, §5.4, §6.1 corrected
  plugins/planet-crafter/
    Cargo.toml  manifest.json  icon.png         # icon copied from the .NET plugin
    src/lib.rs           # Plugin impl: detect, open, export
    src/format.rs        # lossless @/| scanner and writer
    src/save.rs          # typed access, inventory linkage
    src/localization.rs  # GROUP_NAME_* names from the aux file
    src/document.rs      # tabs, view, update
    tests/common/mod.rs  tests/format.rs  tests/save.rs  tests/localization.rs
    tests/document.rs    tests/plugin.rs  tests/real_save.rs
  crates/saveforge-app/tests/planet_crafter_plugin.rs   # end to end through wasm
```

---

### Task 1: Crate and lossless format

**Files:**
- Modify: `Cargo.toml` (wasmtime 49.0.3, workspace member), `Cargo.lock`
- Create: `plugins/planet-crafter/Cargo.toml`, `plugins/planet-crafter/src/lib.rs` (interim), `plugins/planet-crafter/src/format.rs`, `docs/sample-saves/PlanetCrafter-Standard-1.json`
- Test: `plugins/planet-crafter/tests/common/mod.rs`, `plugins/planet-crafter/tests/format.rs`

**Interfaces:**
- Produces `planet_crafter::format`:
  - Constants: `GROUP_COUNT = 11` and `GROUP_NAMES: [&str; 9]`.
  - `Object`:
    - `new(Map<String, Value>)` creates an object that counts as edited.
    - `get(&str) -> Option<&Value>` and `fields() -> &Map<String, Value>`.
    - `set(&str, Value) -> bool` keeps the key's position and returns whether the value changed.
  - `Group`: `objects() -> &[Object]`, `objects_mut() -> &mut [Object]`, and `push(Object)`, which copies the group's separator.
  - `SaveFile`:
    - `parse(&str) -> Result<SaveFile, String>` and `to_text() -> String`.
    - `group(i)` / `group_mut(i)` access a group.
    - `single(i)` / `single_mut(i)` give the one object of groups 0, 5 and 8.
- Produces test helpers in `tests/common/mod.rs`: `GROUPS: [&[&str]; 11]`, `build(&[&[&str]]) -> String`, `minimal() -> String` and `sample() -> Vec<u8>`.

- [ ] **Step 1: Create the branch**

```bash
git checkout -b feat/m5-planet-crafter
```

- [ ] **Step 2: Clear the new wasmtime advisories**

Since M4 merged, RustSec has published RUSTSEC-2026-0352 through RUSTSEC-2026-0364 against `wasmtime` / `wasmtime-wasi` 49.0.2, so `cargo deny check` fails on `main` too. The fix is the 49.0.3 patch release.

Run: `cargo deny check advisories`
Expected: `advisories FAILED`, with the IDs above (this is the RED step).

In the workspace `Cargo.toml` `[workspace.dependencies]`, change `wasmtime = { version = "49.0.2", …` to `wasmtime = { version = "49.0.3", …` (keep its features), and `wasmtime-wasi = "49.0.2"` to `wasmtime-wasi = "49.0.3"`. Then:

```bash
cargo update -p wasmtime -p wasmtime-wasi
cargo deny check
cargo xtask plugins && cargo test -p saveforge-host -p saveforge-app
```

Expected:
- The update moves `wasmtime*`, `cranelift*`, `pulley*` and `wiggle*` from 49.0.2 / 0.136.2 to 49.0.3 / 0.136.3.
- `advisories ok, bans ok, licenses ok, sources ok`.
- The host and app suites pass unchanged.

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: update wasmtime to 49.0.3 (RUSTSEC-2026-0352..0364)"
```

- [ ] **Step 3: Crate, member, sample save**

In the workspace `Cargo.toml` `members`, add `"plugins/planet-crafter",` before `"plugins/sample-stardew",`.

`plugins/planet-crafter/Cargo.toml`:

```toml
[package]
name = "planet-crafter"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
saveforge-sdk.workspace = true
serde_json = { workspace = true, features = ["arbitrary_precision", "preserve_order"] }

[lints]
workspace = true
```

`saveforge-sdk` is unused until Task 3. `cargo machete` only runs in Task 6.

Interim `plugins/planet-crafter/src/lib.rs`:

```rust
//! Planet Crafter save editor (spec §6.1). Built up over the M5 tasks.

pub mod format;
```

Copy the sample save (490 KB, `\r`-wrapped groups, 12 groups including the trailing empty one):

```bash
cp ../dotnet/docs/sample-saves/PlanetCrafter-Standard-1.json docs/sample-saves/
```

- [ ] **Step 4: Write the failing tests**

`plugins/planet-crafter/tests/common/mod.rs`:

```rust
#![allow(dead_code)] // each test file uses a subset

//! A small save in the game's layout: each group wrapped in `\r`, objects
//! separated by `|\n`, two planets, and a trailing empty group.

use std::path::Path;

/// Groups of the minimal save, in file order.
pub const GROUPS: [&[&str]; 11] = [
    &[r#"{"terraTokens":100,"allTimeTerraTokens":200,"unlockedGroups":"A,B"}"#],
    &[
        r#"{"planetId":"Prime","unitOxygenLevel":1.0,"unitHeatLevel":2.5,"unitPressureLevel":0.0,"unitPlantsLevel":44554280960.0,"unitInsectsLevel":0.0,"unitAnimalsLevel":0.0,"unitPurificationLevel":-1.0}"#,
        r#"{"planetId":"Humble","unitOxygenLevel":166326.765625,"unitHeatLevel":25745.0390625,"unitPressureLevel":14958.927734375,"unitPlantsLevel":0.0,"unitInsectsLevel":0.0,"unitAnimalsLevel":0.0,"unitPurificationLevel":-1.0}"#,
    ],
    &[
        r#"{"id":76561198047777538,"name":"Alice","inventoryId":5,"equipmentId":6,"playerGaugeOxygen":100.0,"playerGaugeThirst":37.343666076660159,"playerGaugeHealth":90.0,"playerGaugeToxic":0.0,"host":true}"#,
        r#"{"id":111,"name":"Bob","inventoryId":7,"equipmentId":8,"playerGaugeOxygen":50.0,"playerGaugeThirst":60.0,"playerGaugeHealth":70.0,"playerGaugeToxic":5.0,"host":false}"#,
    ],
    &[
        r#"{"id":10,"gId":"Cobalt","pos":"1,2,3"}"#,
        r#"{"id":11,"gId":"Iron"}"#,
        r#"{"id":12,"gId":"Titanium"}"#,
    ],
    &[
        r#"{"id":5,"woIds":"10,11,99,x","size":4}"#,
        r#"{"id":6,"woIds":"","size":2}"#,
        r#"{"id":7,"woIds":"12","size":4}"#,
    ],
    &[r#"{"craftedObjects":50,"totalSaveFileLoad":0,"totalSaveFileTime":3600}"#],
    &[
        r#"{"stringId":"Msg1","isRead":true}"#,
        r#"{"stringId":"Msg2","isRead":false}"#,
    ],
    &[r#"{"stringId":"StoryEvent1"}"#],
    &[r#"{"saveDisplayName":"TestSave","planetId":"Prime"}"#],
    &[r#"{"layerId":"Layer1"}"#],
    &[r#"{"owner":0,"seed":1004335971}"#],
];

/// Builds a save from groups of object texts.
pub fn build(groups: &[&[&str]]) -> String {
    let mut text = groups
        .iter()
        .map(|objects| format!("\r{}\r", objects.join("|\n")))
        .collect::<Vec<_>>()
        .join("@");
    text.push('@');
    text
}

pub fn minimal() -> String {
    build(&GROUPS)
}

/// `docs/sample-saves/PlanetCrafter-Standard-1.json`, a real save.
pub fn sample() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/sample-saves/PlanetCrafter-Standard-1.json"),
    )
    .unwrap()
}
```

`plugins/planet-crafter/tests/format.rs`:

```rust
mod common;

use planet_crafter::format::{Object, SaveFile};
use serde_json::{Map, Value, json};

fn parse(text: &str) -> SaveFile {
    SaveFile::parse(text).unwrap()
}

fn parse_err(text: &str) -> String {
    SaveFile::parse(text).unwrap_err()
}

#[test]
fn parses_groups_and_objects() {
    let save = parse(&common::minimal());
    assert_eq!(save.single(0).get("terraTokens"), Some(&json!(100)));
    assert_eq!(save.group(1).objects().len(), 2, "one object per planet");
    assert_eq!(save.group(2).objects().len(), 2);
    assert_eq!(save.group(3).objects().len(), 3);
    assert_eq!(save.group(3).objects()[0].get("pos"), Some(&json!("1,2,3")));
    assert_eq!(
        save.single(8).get("saveDisplayName"),
        Some(&json!("TestSave"))
    );
    assert_eq!(save.group(11).objects().len(), 0, "trailing empty group");
}

#[test]
fn unmodified_save_round_trips_byte_for_byte() {
    let text = common::minimal();
    assert_eq!(parse(&text).to_text(), text);
}

#[test]
fn sample_save_round_trips_byte_for_byte() {
    let bytes = common::sample();
    let text = std::str::from_utf8(&bytes).unwrap();
    let save = parse(text);
    assert_eq!(save.group(2).objects().len(), 5);
    assert_eq!(save.group(3).objects().len(), 7460);
    assert_eq!(save.group(4).objects().len(), 750);
    assert_eq!(save.to_text(), text);
}

#[test]
fn separators_inside_strings_are_text() {
    let mut groups = common::GROUPS;
    groups[8] = &[r#"{"saveDisplayName":"Base @ Lake | 2","planetId":"Prime"}"#];
    groups[4] = &[r#"{"id":5,"woIds":"10|11","size":4}"#];
    let text = common::build(&groups);
    let save = parse(&text);
    assert_eq!(
        save.single(8).get("saveDisplayName"),
        Some(&json!("Base @ Lake | 2"))
    );
    assert_eq!(save.group(4).objects().len(), 1);
    assert_eq!(save.to_text(), text);
}

#[test]
fn odd_whitespace_and_byte_order_mark_round_trip() {
    let mut text = String::from("\u{feff}");
    text.push_str(&common::minimal().replace("|\n", " \t|\r\n  "));
    text.push_str("\n\n");
    let save = parse(&text);
    assert_eq!(save.group(2).objects().len(), 2);
    assert_eq!(save.to_text(), text);
}

#[test]
fn edited_object_is_rewritten_and_keeps_other_number_text() {
    let text = common::minimal();
    let mut save = parse(&text);
    assert!(save.group_mut(1).objects_mut()[0].set("unitHeatLevel", json!(3.5)));
    let expected = text.replace(r#""unitHeatLevel":2.5"#, r#""unitHeatLevel":3.5"#);
    assert_eq!(save.to_text(), expected);
    assert!(expected.contains(r#""unitPlantsLevel":44554280960.0"#));
}

#[test]
fn setting_the_same_value_is_no_edit() {
    let mut save = parse(&common::minimal());
    assert!(!save.single_mut(0).set("terraTokens", json!(100)));
}

#[test]
fn large_ids_keep_their_digits_after_an_edit() {
    let text = common::minimal();
    let mut save = parse(&text);
    assert!(save.group_mut(2).objects_mut()[0].set("name", json!("Alicia")));
    let out = save.to_text();
    assert!(
        out.contains(r#"{"id":76561198047777538,"name":"Alicia","#),
        "{out}"
    );
    assert!(out.contains(r#""playerGaugeOxygen":100.0,"#), "{out}");
    assert!(
        out.contains(r#""playerGaugeThirst":37.343666076660159,"#),
        "{out}"
    );
}

#[test]
fn pushed_objects_copy_the_group_separator() {
    let mut save = parse(&common::minimal());
    let mut fields = Map::new();
    fields.insert("id".to_owned(), json!(13));
    save.group_mut(3).push(Object::new(fields.clone()));
    assert!(
        save.to_text()
            .contains("{\"id\":12,\"gId\":\"Titanium\"}|\n{\"id\":13}\r@")
    );

    let mut groups = common::GROUPS;
    groups[6] = &[];
    let mut save = parse(&common::build(&groups));
    save.group_mut(6).push(Object::new(fields));
    assert!(save.to_text().contains("@\r\r{\"id\":13}@"));
}

#[test]
fn saves_without_the_trailing_group_parse() {
    // Game version 2.008 writes 11 groups and no trailing empty one.
    let text = common::minimal();
    let text = text.strip_suffix('@').unwrap();
    let save = parse(text);
    assert_eq!(save.group(10).objects().len(), 1);
    assert_eq!(save.to_text(), text);
}

#[test]
fn too_few_groups_are_rejected() {
    assert_eq!(
        parse_err("{}@{}@{}"),
        "expected at least 11 groups separated by '@', found 3"
    );
}

#[test]
fn single_groups_must_hold_one_object() {
    let mut groups = common::GROUPS;
    groups[0] = &[r#"{"terraTokens":1}"#, r#"{"terraTokens":2}"#];
    assert_eq!(
        parse_err(&common::build(&groups)),
        "TerraTokens must hold exactly one object, found 2"
    );
    groups = common::GROUPS;
    groups[5] = &[];
    assert_eq!(
        parse_err(&common::build(&groups)),
        "Stats must hold exactly one object, found 0"
    );
}

#[test]
fn malformed_text_is_rejected_with_its_location() {
    let text = common::minimal();
    let err = parse_err(&text.replacen(r#"{"id":11,"gId":"Iron"}"#, r#"{"id":11,"gId":}"#, 1));
    assert!(
        err.starts_with("WorldObjects object 2 is not valid JSON"),
        "{err}"
    );

    let err = parse_err(&text.replacen("|\n{\"id\":11,", "|\nx{\"id\":11,", 1));
    assert!(err.starts_with("unexpected 'x' at byte"), "{err}");
    assert!(err.ends_with("in WorldObjects"), "{err}");

    let err = parse_err(&text.replacen("|\n{\"id\":11,", "\n{\"id\":11,", 1));
    assert!(err.starts_with("unexpected '{' at byte"), "{err}");

    let err = parse_err(&text.replacen("\r@\r{\"id\":5", "|\r@\r{\"id\":5", 1));
    assert!(err.starts_with("unexpected '@' at byte"), "{err}");

    assert_eq!(
        parse_err(&format!("{text}{{\"a\":1}}|")),
        "group 11 ends with '|'"
    );
    assert_eq!(
        parse_err(&text.replacen("{\"terraTokens\"", "[1]", 1)),
        "unexpected '[' at byte 1 in TerraTokens"
    );
}

#[test]
fn values_are_json_objects() {
    let save = parse(&common::minimal());
    let fields = save.single(5).fields();
    assert_eq!(
        Value::Object(fields.clone()),
        json!({ "craftedObjects": 50, "totalSaveFileLoad": 0, "totalSaveFileTime": 3600 })
    );
}
```

- [ ] **Step 5: Run them to verify they fail**

Run: `cargo test -p planet-crafter --test format`
Expected: compile errors, unresolved `planet_crafter::format::{Object, SaveFile}`.

- [ ] **Step 6: Write `plugins/planet-crafter/src/format.rs`**

```rust
//! The Planet Crafter save format (spec §6.1), parsed losslessly.
//!
//! A save is a sequence of groups separated by `@`, and a group holds JSON
//! objects separated by `|`. Everything between objects (whitespace, the
//! separators, a leading byte order mark) is kept verbatim, and an object is
//! written back from its original text unless it was edited. An unmodified
//! save therefore serializes byte for byte as it was read.

use std::mem;

use serde_json::{Map, Value};

/// Groups every save has. What follows `DisplayInfo` differs between game
/// versions, so later groups are only kept, never interpreted.
pub const GROUP_COUNT: usize = 11;

/// Names of the groups the editor reads, in file order, for error messages.
pub const GROUP_NAMES: [&str; 9] = [
    "TerraTokens",
    "Planets",
    "Players",
    "WorldObjects",
    "Inventories",
    "Stats",
    "Messages",
    "StoryEvents",
    "DisplayInfo",
];

/// Groups that hold exactly one object.
const SINGLE_GROUPS: [usize; 3] = [0, 5, 8];

/// Separator for objects appended to a group that has no separator to copy.
const DEFAULT_SEPARATOR: &str = "|\n";

/// One JSON object of a group.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    raw: String,
    fields: Map<String, Value>,
    edited: bool,
}

impl Object {
    /// A new object, written out compactly.
    pub fn new(fields: Map<String, Value>) -> Self {
        Self {
            raw: String::new(),
            fields,
            edited: true,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    pub fn fields(&self) -> &Map<String, Value> {
        &self.fields
    }

    /// Sets `key`, keeping its position. Returns whether anything changed.
    pub fn set(&mut self, key: &str, value: Value) -> bool {
        if self.fields.get(key) == Some(&value) {
            return false;
        }
        self.fields.insert(key.to_owned(), value);
        self.edited = true;
        true
    }

    fn write(&self, out: &mut String) {
        if self.edited {
            out.push_str(&Value::Object(self.fields.clone()).to_string());
        } else {
            out.push_str(&self.raw);
        }
    }
}

/// The objects of one group and the exact text around them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Group {
    /// Text before, between and after the objects: `objects.len() + 1` items.
    gaps: Vec<String>,
    objects: Vec<Object>,
}

impl Group {
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }

    pub fn objects_mut(&mut self) -> &mut [Object] {
        &mut self.objects
    }

    /// Appends `object`, separated the way the group's objects already are.
    pub fn push(&mut self, object: Object) {
        if self.objects.is_empty() {
            self.gaps.push(String::new());
        } else {
            let separator = if self.objects.len() >= 2 {
                self.gaps[1].clone()
            } else {
                DEFAULT_SEPARATOR.to_owned()
            };
            let trailing = self.gaps.pop().unwrap_or_default();
            self.gaps.push(separator);
            self.gaps.push(trailing);
        }
        self.objects.push(object);
    }

    fn write(&self, out: &mut String) {
        for (gap, object) in self.gaps.iter().zip(&self.objects) {
            out.push_str(gap);
            object.write(out);
        }
        if let Some(trailing) = self.gaps.last() {
            out.push_str(trailing);
        }
    }
}

/// A whole save: at least [`GROUP_COUNT`] groups.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveFile {
    groups: Vec<Group>,
}

impl SaveFile {
    /// Parses `text`. `@` and `|` inside JSON strings are part of the string,
    /// not separators.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut groups = Vec::new();
        let mut group = Group::default();
        let mut gap = String::new();
        let mut after_separator = false;
        let mut i = 0;
        if text.starts_with('\u{feff}') {
            gap.push('\u{feff}');
            i = '\u{feff}'.len_utf8();
        }
        while let Some(c) = text[i..].chars().next() {
            match c {
                ' ' | '\t' | '\r' | '\n' => gap.push(c),
                '|' if !group.objects.is_empty() && !after_separator => {
                    gap.push(c);
                    after_separator = true;
                }
                '@' if !after_separator => {
                    group.gaps.push(mem::take(&mut gap));
                    groups.push(mem::take(&mut group));
                }
                '{' if group.objects.is_empty() || after_separator => {
                    let mut stream =
                        serde_json::Deserializer::from_str(&text[i..]).into_iter::<Map<_, _>>();
                    let fields = match stream.next() {
                        Some(Ok(fields)) => fields,
                        Some(Err(e)) => {
                            return Err(format!(
                                "{} object {} is not valid JSON: {e}",
                                group_name(groups.len()),
                                group.objects.len() + 1
                            ));
                        }
                        None => return Err("unexpected end of the save".to_owned()),
                    };
                    let end = i + stream.byte_offset();
                    group.gaps.push(mem::take(&mut gap));
                    group.objects.push(Object {
                        raw: text[i..end].to_owned(),
                        fields,
                        edited: false,
                    });
                    after_separator = false;
                    i = end;
                    continue;
                }
                _ => {
                    return Err(format!(
                        "unexpected {c:?} at byte {i} in {}",
                        group_name(groups.len())
                    ));
                }
            }
            i += c.len_utf8();
        }
        if after_separator {
            return Err(format!("{} ends with '|'", group_name(groups.len())));
        }
        group.gaps.push(gap);
        groups.push(group);

        if groups.len() < GROUP_COUNT {
            return Err(format!(
                "expected at least {GROUP_COUNT} groups separated by '@', found {}",
                groups.len()
            ));
        }
        for index in SINGLE_GROUPS {
            let found = groups[index].objects.len();
            if found != 1 {
                return Err(format!(
                    "{} must hold exactly one object, found {found}",
                    GROUP_NAMES[index]
                ));
            }
        }
        Ok(Self { groups })
    }

    pub fn group(&self, index: usize) -> &Group {
        &self.groups[index]
    }

    pub fn group_mut(&mut self, index: usize) -> &mut Group {
        &mut self.groups[index]
    }

    /// The single object of a [`SINGLE_GROUPS`] group.
    pub fn single(&self, index: usize) -> &Object {
        &self.groups[index].objects[0]
    }

    pub fn single_mut(&mut self, index: usize) -> &mut Object {
        &mut self.groups[index].objects[0]
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (n, group) in self.groups.iter().enumerate() {
            if n > 0 {
                out.push('@');
            }
            group.write(&mut out);
        }
        out
    }
}

fn group_name(index: usize) -> String {
    GROUP_NAMES
        .get(index)
        .map_or_else(|| format!("group {index}"), |name| (*name).to_owned())
}
```

- [ ] **Step 7: Run the tests and lints**

Run: `cargo test -p planet-crafter --test format`
Expected: 14 passed. That includes `sample_save_round_trips_byte_for_byte` (5 players, 7,460 world objects, 750 inventories).

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p planet-crafter --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock plugins/planet-crafter docs/sample-saves/PlanetCrafter-Standard-1.json
git commit -m "feat(planet-crafter): lossless save format"
```

---

### Task 2: Typed save and inventory linkage

**Files:**
- Create: `plugins/planet-crafter/src/save.rs`
- Modify: `plugins/planet-crafter/src/lib.rs` (interim)
- Test: `plugins/planet-crafter/tests/save.rs`

**Interfaces:**
- Consumes: `format::{SaveFile, Object}` and the Task 1 test helpers.
- Produces `planet_crafter::save`:
  - Group constants: `TERRA_TOKENS = 0`, `PLANETS = 1`, `PLAYERS = 2`, `WORLD_OBJECTS = 3`, `INVENTORIES = 4`, `STATS = 5`, `DISPLAY_INFO = 8`.
  - `Item { world_object_id: i64, gid: String }`.
  - `Save` methods:

    | Method | Returns / behaviour |
    |---|---|
    | `parse(&[u8])` | `Result<Save, String>` |
    | `to_bytes()` | `Vec<u8>` |
    | `single(group)` / `single_mut(group)` | the group's one object |
    | `planets()` / `planet_mut(i)` | the planet objects |
    | `players()` / `player_mut(i)` | the player objects |
    | `inventory_items(inventory_id)` | `Option<Vec<Item>>`, `None` when the inventory is missing |
    | `add_item(inventory_id, gid)` | `Option<i64>`, the new world object id |
    | `remove_item(inventory_id, world_object_id)` | `bool` |
    | `gids()` | `Vec<String>`, distinct and sorted |

  - Free functions: `int(Option<&Value>) -> Option<i64>` (whole floats accepted) and `text(Option<&Value>) -> &str`.

- [ ] **Step 1: Write the failing test `plugins/planet-crafter/tests/save.rs`**

```rust
mod common;

use planet_crafter::save::{Item, Save, int};
use serde_json::json;

fn minimal() -> Save {
    Save::parse(common::minimal().as_bytes()).unwrap()
}

fn item(id: i64, gid: &str) -> Item {
    Item {
        world_object_id: id,
        gid: gid.to_owned(),
    }
}

fn wo_ids(save: &Save, inventory: usize) -> String {
    let text = String::from_utf8(save.to_bytes()).unwrap();
    let parsed = planet_crafter::format::SaveFile::parse(&text).unwrap();
    parsed.group(4).objects()[inventory]
        .get("woIds")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_owned()
}

#[test]
fn non_utf8_is_rejected() {
    assert_eq!(
        Save::parse(b"\xff\xfe{}").unwrap_err(),
        "the save is not valid UTF-8"
    );
}

#[test]
fn inventory_items_follow_wo_ids_and_skip_unknown_entries() {
    // Inventory 5 holds "10,11,99,x": 99 has no world object, x is no id.
    assert_eq!(
        minimal().inventory_items(5),
        Some(vec![item(10, "Cobalt"), item(11, "Iron")])
    );
    assert_eq!(minimal().inventory_items(6), Some(vec![]));
}

#[test]
fn missing_inventory_is_none() {
    assert_eq!(minimal().inventory_items(8), None);
}

#[test]
fn wo_ids_split_on_commas_and_pipes() {
    let mut groups = common::GROUPS;
    groups[4] = &[r#"{"id":5,"woIds":" 10 | 12 ,-3","size":4}"#];
    let save = Save::parse(common::build(&groups).as_bytes()).unwrap();
    assert_eq!(
        save.inventory_items(5),
        Some(vec![item(10, "Cobalt"), item(12, "Titanium")])
    );
}

#[test]
fn add_item_creates_a_world_object_with_the_next_id() {
    let mut save = minimal();
    assert_eq!(save.add_item(6, "Iron"), Some(13));
    assert_eq!(save.inventory_items(6), Some(vec![item(13, "Iron")]));
    assert_eq!(wo_ids(&save, 1), "13");
    let text = String::from_utf8(save.to_bytes()).unwrap();
    assert!(text.contains("{\"id\":12,\"gId\":\"Titanium\"}|\n{\"id\":13,\"gId\":\"Iron\"}\r@"));
}

#[test]
fn add_item_rewrites_wo_ids_comma_joined() {
    let mut save = minimal();
    assert_eq!(save.add_item(5, "Titanium"), Some(13));
    assert_eq!(wo_ids(&save, 0), "10,11,99,13");
}

#[test]
fn add_item_to_a_missing_inventory_changes_nothing() {
    let mut save = minimal();
    assert_eq!(save.add_item(8, "Iron"), None);
    assert_eq!(save.to_bytes(), common::minimal().into_bytes());
}

#[test]
fn add_item_to_a_save_without_world_objects_starts_at_one() {
    let mut groups = common::GROUPS;
    groups[3] = &[];
    let mut save = Save::parse(common::build(&groups).as_bytes()).unwrap();
    assert_eq!(save.add_item(6, "Iron"), Some(1));
}

#[test]
fn remove_item_drops_the_id_but_keeps_the_world_object() {
    let mut save = minimal();
    assert!(save.remove_item(5, 11));
    assert_eq!(save.inventory_items(5), Some(vec![item(10, "Cobalt")]));
    assert_eq!(wo_ids(&save, 0), "10,99");
    assert!(save.gids().contains(&"Iron".to_owned()));
}

#[test]
fn remove_item_that_is_not_there_changes_nothing() {
    let mut save = minimal();
    assert!(!save.remove_item(5, 12));
    assert!(!save.remove_item(8, 10));
    assert_eq!(save.to_bytes(), common::minimal().into_bytes());
}

#[test]
fn gids_are_distinct_and_sorted() {
    assert_eq!(minimal().gids(), ["Cobalt", "Iron", "Titanium"]);
}

#[test]
fn int_accepts_whole_floats() {
    assert_eq!(int(Some(&json!(12))), Some(12));
    assert_eq!(int(Some(&json!(12.0))), Some(12));
    assert_eq!(int(Some(&json!(12.5))), None);
    assert_eq!(int(Some(&json!("12"))), None);
}

#[test]
fn sample_save_links_player_inventories() {
    let save = Save::parse(&common::sample()).unwrap();
    let host = &save.players()[0];
    let inventory = int(host.get("inventoryId")).unwrap();
    let items = save.inventory_items(inventory).unwrap();
    assert_ne!(items.len(), 0);
    assert!(items.iter().all(|i| !i.gid.is_empty()));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p planet-crafter --test save`
Expected: compile error, unresolved `planet_crafter::save`.

- [ ] **Step 3: Write `plugins/planet-crafter/src/save.rs`**

```rust
//! Typed access to a parsed save (spec §6.1): players, world objects and
//! the inventory linkage between them (port of the .NET `PlayerViewModel`).

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value};

use crate::format::{Object, SaveFile};

pub const TERRA_TOKENS: usize = 0;
pub const PLANETS: usize = 1;
pub const PLAYERS: usize = 2;
pub const WORLD_OBJECTS: usize = 3;
pub const INVENTORIES: usize = 4;
pub const STATS: usize = 5;
pub const DISPLAY_INFO: usize = 8;

/// An item shown in a player's inventory: its world object id and group id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub world_object_id: i64,
    pub gid: String,
}

/// A parsed save with an index of world objects by id.
#[derive(Debug, Clone)]
pub struct Save {
    file: SaveFile,
    /// World object id → position in the `WorldObjects` group (first wins).
    world_objects: HashMap<i64, usize>,
}

impl Save {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "the save is not valid UTF-8")?;
        let file = SaveFile::parse(text)?;
        let mut world_objects = HashMap::new();
        for (index, object) in file.group(WORLD_OBJECTS).objects().iter().enumerate() {
            if let Some(id) = int(object.get("id")) {
                world_objects.entry(id).or_insert(index);
            }
        }
        Ok(Self {
            file,
            world_objects,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.file.to_text().into_bytes()
    }

    pub fn single(&self, group: usize) -> &Object {
        self.file.single(group)
    }

    pub fn single_mut(&mut self, group: usize) -> &mut Object {
        self.file.single_mut(group)
    }

    /// One object per planet (Prime, Humble, …).
    pub fn planets(&self) -> &[Object] {
        self.file.group(PLANETS).objects()
    }

    pub fn planet_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.file.group_mut(PLANETS).objects_mut().get_mut(index)
    }

    pub fn players(&self) -> &[Object] {
        self.file.group(PLAYERS).objects()
    }

    pub fn player_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.file.group_mut(PLAYERS).objects_mut().get_mut(index)
    }

    /// The items of inventory `inventory_id`, in `woIds` order, skipping ids
    /// with no world object. `None` when the inventory does not exist.
    pub fn inventory_items(&self, inventory_id: i64) -> Option<Vec<Item>> {
        let inventory = self.inventory(inventory_id)?;
        let world = self.file.group(WORLD_OBJECTS).objects();
        let items = wo_ids(inventory)
            .into_iter()
            .filter_map(|id| {
                let object = &world[*self.world_objects.get(&id)?];
                Some(Item {
                    world_object_id: id,
                    gid: text(object.get("gId")).to_owned(),
                })
            })
            .collect();
        Some(items)
    }

    /// Creates world object `{ id: max + 1, gId }` and appends it to the
    /// inventory. Returns the new id, or `None` when the inventory does not
    /// exist.
    pub fn add_item(&mut self, inventory_id: i64, gid: &str) -> Option<i64> {
        let index = self.inventory_index(inventory_id)?;
        let id = self.world_objects.keys().max().map_or(1, |max| max + 1);
        let mut fields = Map::new();
        fields.insert("id".to_owned(), Value::from(id));
        fields.insert("gId".to_owned(), Value::from(gid));
        let world = self.file.group_mut(WORLD_OBJECTS);
        world.push(Object::new(fields));
        self.world_objects.insert(id, world.objects().len() - 1);

        let inventory = &mut self.file.group_mut(INVENTORIES).objects_mut()[index];
        let mut ids = wo_ids(inventory);
        ids.push(id);
        inventory.set("woIds", Value::from(join(&ids)));
        Some(id)
    }

    /// Removes the first `world_object_id` from the inventory's `woIds`. The
    /// world object itself stays, as in .NET. Returns whether it was there.
    pub fn remove_item(&mut self, inventory_id: i64, world_object_id: i64) -> bool {
        let Some(index) = self.inventory_index(inventory_id) else {
            return false;
        };
        let inventory = &mut self.file.group_mut(INVENTORIES).objects_mut()[index];
        let mut ids = wo_ids(inventory);
        let Some(position) = ids.iter().position(|id| *id == world_object_id) else {
            return false;
        };
        ids.remove(position);
        inventory.set("woIds", Value::from(join(&ids)))
    }

    /// Distinct `gId`s of all world objects, sorted.
    pub fn gids(&self) -> Vec<String> {
        self.file
            .group(WORLD_OBJECTS)
            .objects()
            .iter()
            .filter_map(|o| o.get("gId").and_then(Value::as_str))
            .filter(|gid| !gid.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn inventory(&self, id: i64) -> Option<&Object> {
        self.inventory_index(id)
            .map(|index| &self.file.group(INVENTORIES).objects()[index])
    }

    fn inventory_index(&self, id: i64) -> Option<usize> {
        self.file
            .group(INVENTORIES)
            .objects()
            .iter()
            .position(|inventory| int(inventory.get("id")) == Some(id))
    }
}

/// An integer field, also when stored as a whole float (`12.0`).
pub fn int(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    value.as_i64().or_else(|| {
        let float = value.as_f64()?;
        #[allow(clippy::cast_possible_truncation)] // whole and in range
        (float.fract() == 0.0 && float.abs() <= 9_007_199_254_740_992.0).then_some(float as i64)
    })
}

/// A string field, or `""`.
pub fn text(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or_default()
}

/// The ids in an inventory's `woIds`, split on `,` and `|`; entries that are
/// not non-negative integers are dropped.
fn wo_ids(inventory: &Object) -> Vec<i64> {
    text(inventory.get("woIds"))
        .split([',', '|'])
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .filter(|id| *id >= 0)
        .collect()
}

fn join(ids: &[i64]) -> String {
    ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
}
```

Update `plugins/planet-crafter/src/lib.rs`:

```rust
//! Planet Crafter save editor (spec §6.1). Built up over the M5 tasks.

pub mod format;
pub mod save;
```

- [ ] **Step 4: Run the tests and lints**

Run: `cargo test -p planet-crafter`
Expected: format 14 passed, save 13 passed.

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p planet-crafter --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add plugins/planet-crafter
git commit -m "feat(planet-crafter): typed save access and inventory linkage"
```

---

### Task 3: Localization and the editor document

**Files:**
- Create: `plugins/planet-crafter/src/localization.rs`, `plugins/planet-crafter/src/document.rs`
- Modify: `plugins/planet-crafter/src/lib.rs` (interim)
- Test: `plugins/planet-crafter/tests/localization.rs`, `plugins/planet-crafter/tests/document.rs`

**Interfaces:**
- Consumes: `save::*` (Task 2) and `saveforge_sdk::{Document, ui}`.
- Produces:
  - `localization::Names` (`Default` = no names):
    - `parse(&[u8]) -> Names`: lossy UTF-8; lines `GROUP_NAME_{gid}**english*…`; empty names are skipped.
    - `name(&gid) -> &str`: the gid itself when unknown.
    - `known_items(fallback: Vec<String>) -> Vec<(gid, name)>`: sorted by name ignoring case, then by gid. Without names, the fallback gids are offered under their raw ids.
  - `document::PlanetCrafterDocument`:
    - `new(Save, Names)` and `save() -> &Save`; implements `saveforge_sdk::Document`.
    - Exports the widget-id constants `TABS`, `CHARACTER`, `EQUIPMENT`, `INVENTORY`, `TERRA_TOKENS_ID`, `ALL_TIME_TOKENS_ID`, `UNLOCKED_GROUPS_ID`, `DISPLAY_NAME_ID`, `PLANET_ID_ID`, `CRAFTED_OBJECTS_ID`, and `TAB_TITLES`.
  - **Notes shown in the UI:**
    - `This save has no characters.`
    - `This save has no planets.`
    - `This inventory is not in the save.`
    - Add placeholder: `Add item…`

- [ ] **Step 1: Write the failing tests**

`plugins/planet-crafter/tests/localization.rs`:

```rust
use planet_crafter::localization::Names;

const FILE: &str = "\u{feff}LANGUAGES**English*French\r\n\
    GROUP_NAME_Iron**Iron Ore*Minerai de fer\r\n\
    GROUP_NAME_Cobalt** Cobalt *Cobalt\r\n\
    GROUP_DESC_Iron**Used for crafting*Utilisé\r\n\
    GROUP_NAME_Empty***Vide\r\n\
    GROUP_NAME_aluminium**aluminium\r\n\
    not a key line\r\n";

#[test]
fn names_come_from_the_english_column() {
    let names = Names::parse(FILE.as_bytes());
    assert_eq!(names.name("Iron"), "Iron Ore");
    assert_eq!(names.name("Cobalt"), "Cobalt");
    assert_eq!(names.name("aluminium"), "aluminium");
}

#[test]
fn unknown_and_empty_names_fall_back_to_the_gid() {
    let names = Names::parse(FILE.as_bytes());
    assert_eq!(names.name("Empty"), "Empty");
    assert_eq!(names.name("Titanium"), "Titanium");
    assert_eq!(Names::default().name("Iron"), "Iron");
}

#[test]
fn known_items_are_sorted_by_name_ignoring_case() {
    let names = Names::parse(FILE.as_bytes());
    let items = names.known_items(vec!["Titanium".to_owned()]);
    let pairs: Vec<_> = items
        .iter()
        .map(|(gid, name)| (gid.as_str(), name.as_str()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("aluminium", "aluminium"),
            ("Cobalt", "Cobalt"),
            ("Iron", "Iron Ore")
        ]
    );
}

#[test]
fn without_a_file_the_save_gids_are_offered() {
    let items = Names::default().known_items(vec!["iron".to_owned(), "Cobalt".to_owned()]);
    assert_eq!(
        items,
        [
            ("Cobalt".to_owned(), "Cobalt".to_owned()),
            ("iron".to_owned(), "iron".to_owned())
        ]
    );
}

#[test]
fn invalid_utf8_does_not_fail() {
    let mut bytes = b"GROUP_NAME_Iron**Iron\xff*x\n".to_vec();
    bytes.extend_from_slice(b"GROUP_NAME_Gold**Gold\n");
    let names = Names::parse(&bytes);
    assert_eq!(names.name("Gold"), "Gold");
    assert_eq!(names.name("Iron"), "Iron\u{fffd}");
}
```

`plugins/planet-crafter/tests/document.rs`:

```rust
mod common;

use planet_crafter::document::{
    ALL_TIME_TOKENS_ID, CHARACTER, CRAFTED_OBJECTS_ID, DISPLAY_NAME_ID, EQUIPMENT, INVENTORY,
    PLANET_ID_ID, PlanetCrafterDocument, TABS, TERRA_TOKENS_ID, UNLOCKED_GROUPS_ID,
};
use planet_crafter::format::SaveFile;
use planet_crafter::localization::Names;
use planet_crafter::save::Save;
use saveforge_sdk::Document;
use saveforge_sdk::ui::{Event, FieldSpec, Input, UiTree, Widget};
use serde_json::{Value, json};

const NAMES: &str = "GROUP_NAME_Iron**Iron Ore*Fer\nGROUP_NAME_Cobalt**Cobalt*Cobalt\n";

fn open(text: &str) -> PlanetCrafterDocument {
    PlanetCrafterDocument::new(
        Save::parse(text.as_bytes()).unwrap(),
        Names::parse(NAMES.as_bytes()),
    )
}

fn doc() -> PlanetCrafterDocument {
    open(&common::minimal())
}

fn tab(doc: &mut PlanetCrafterDocument, index: u32) -> UiTree {
    doc.update(Event::TabSelected((TABS.to_owned(), index)));
    doc.view()
}

fn field<'a>(tree: &'a UiTree, id: &str) -> &'a FieldSpec {
    tree.nodes
        .iter()
        .find_map(|n| match &n.widget {
            Widget::Field(f) if f.id == id => Some(f),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no field {id}"))
}

fn number(tree: &UiTree, id: &str) -> f64 {
    match &field(tree, id).input {
        Input::Number(n) => n.value,
        other => panic!("{id} is not a number: {other:?}"),
    }
}

fn texts(tree: &UiTree) -> Vec<String> {
    tree.nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

/// Display texts of a list's rows.
fn rows(tree: &UiTree, list_id: &str) -> Vec<String> {
    let list = tree
        .nodes
        .iter()
        .find(|n| matches!(&n.widget, Widget::List(l) if l.id == list_id))
        .unwrap_or_else(|| panic!("no list {list_id}"));
    list.children
        .iter()
        .map(|&i| match &tree.nodes[i as usize].widget {
            Widget::Text(t) => t.text.clone(),
            other => panic!("row is not text: {other:?}"),
        })
        .collect()
}

/// The saved file, parsed again.
fn saved(doc: &PlanetCrafterDocument) -> SaveFile {
    SaveFile::parse(&String::from_utf8(doc.serialize().unwrap()).unwrap()).unwrap()
}

fn num(id: &str, value: f64) -> Event {
    Event::NumberChanged((id.to_owned(), value))
}

#[test]
fn view_shows_four_tabs_and_the_host_first() {
    let tree = doc().view();
    let Widget::Tabs(tabs) = &tree.nodes[0].widget else {
        panic!("root is not tabs");
    };
    assert_eq!(
        tabs.titles,
        ["Characters", "Terra Tokens", "Planet State", "Save Info"]
    );
    let Input::Pick(pick) = &field(&tree, CHARACTER).input else {
        panic!("character is not a pick-list");
    };
    let names: Vec<_> = pick.options.iter().map(|o| o.display.as_str()).collect();
    assert_eq!(names, ["Alice (Host)", "Bob"]);
    assert_eq!(pick.selected.as_deref(), Some("0"));
    assert!((number(&tree, "player.playerGaugeHealth") - 90.0).abs() < f64::EPSILON);
}

#[test]
fn gauges_edit_the_selected_player() {
    let mut doc = doc();
    doc.update(num("player.playerGaugeHealth", 55.5));
    assert!(doc.is_dirty());
    doc.update(Event::Picked((CHARACTER.to_owned(), "1".to_owned())));
    doc.update(num("player.playerGaugeToxic", 12.5));
    let save = saved(&doc);
    let players = save.group(2).objects();
    assert_eq!(players[0].get("playerGaugeHealth"), Some(&json!(55.5)));
    assert_eq!(players[1].get("playerGaugeToxic"), Some(&json!(12.5)));
    assert_eq!(players[1].get("playerGaugeHealth"), Some(&json!(70.0)));
}

#[test]
fn out_of_range_and_unchanged_numbers_are_not_edits() {
    let mut doc = doc();
    doc.update(num("player.playerGaugeHealth", 150.0));
    doc.update(num("player.playerGaugeOxygen", -1.0));
    doc.update(num("player.playerGaugeHealth", f64::NAN));
    doc.update(num("player.playerGaugeHealth", 90.0));
    doc.update(num("player.unknownGauge", 1.0));
    doc.update(num("planet.0.unitPurificationLevel", 1.0));
    doc.update(num("planet.2.unitHeatLevel", 1.0));
    doc.update(num("planet.x.unitHeatLevel", 1.0));
    doc.update(num("planet.unitHeatLevel", 1.0));
    doc.update(num(ALL_TIME_TOKENS_ID, 1.0));
    assert!(!doc.is_dirty());
    assert_eq!(doc.serialize().unwrap(), common::minimal().into_bytes());
}

#[test]
fn oxygen_allows_up_to_370() {
    let mut doc = doc();
    doc.update(num("player.playerGaugeOxygen", 370.0));
    assert_eq!(
        saved(&doc).group(2).objects()[0].get("playerGaugeOxygen"),
        Some(&json!(370.0))
    );
}

#[test]
fn terra_tokens_are_whole_numbers() {
    let mut doc = doc();
    let tree = tab(&mut doc, 1);
    assert!((number(&tree, TERRA_TOKENS_ID) - 100.0).abs() < f64::EPSILON);
    assert!(field(&tree, ALL_TIME_TOKENS_ID).read_only);
    assert!(field(&tree, UNLOCKED_GROUPS_ID).read_only);
    doc.update(num(TERRA_TOKENS_ID, 15000.0));
    let text = String::from_utf8(doc.serialize().unwrap()).unwrap();
    assert!(
        text.contains(r#"{"terraTokens":15000,"allTimeTerraTokens":200,"#),
        "{text}"
    );
}

#[test]
fn planet_levels_edit_each_planet() {
    let mut doc = doc();
    let tree = tab(&mut doc, 2);
    assert!((number(&tree, "planet.0.unitPlantsLevel") - 44_554_280_960.0).abs() < 1.0);
    assert!((number(&tree, "planet.1.unitOxygenLevel") - 166_326.765_625).abs() < 1e-6);
    let titles: Vec<_> = tree
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Section(title) => Some(title.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(titles, ["Prime", "Humble"]);

    doc.update(num("planet.1.unitHeatLevel", 1234.56));
    let save = saved(&doc);
    assert_eq!(
        save.group(1).objects()[1].get("unitHeatLevel"),
        Some(&json!(1234.56))
    );
    assert_eq!(
        save.group(1).objects()[0].get("unitHeatLevel"),
        Some(&json!(2.5))
    );
}

#[test]
fn display_name_is_editable_and_capped_at_64_characters() {
    let mut doc = doc();
    let tree = tab(&mut doc, 3);
    assert!(field(&tree, PLANET_ID_ID).read_only);
    assert!(field(&tree, CRAFTED_OBJECTS_ID).read_only);
    let Input::Text(name) = &field(&tree, CRAFTED_OBJECTS_ID).input else {
        panic!("crafted objects is not text");
    };
    assert_eq!(name.value, "50");

    doc.update(Event::TextChanged((
        DISPLAY_NAME_ID.to_owned(),
        "é".repeat(70),
    )));
    assert_eq!(
        saved(&doc).single(8).get("saveDisplayName"),
        Some(&Value::String("é".repeat(64)))
    );
}

#[test]
fn read_only_text_fields_ignore_edits() {
    let mut doc = doc();
    for id in [PLANET_ID_ID, CRAFTED_OBJECTS_ID, UNLOCKED_GROUPS_ID] {
        doc.update(Event::TextChanged((id.to_owned(), "x".to_owned())));
    }
    assert!(!doc.is_dirty());
}

#[test]
fn inventories_show_localized_names_and_offer_known_items() {
    let tree = doc().view();
    assert_eq!(rows(&tree, INVENTORY), ["Cobalt", "Iron Ore"]);
    assert_eq!(rows(&tree, EQUIPMENT), Vec::<String>::new());
    let list = tree
        .nodes
        .iter()
        .find_map(|n| match &n.widget {
            Widget::List(l) if l.id == INVENTORY => Some(l),
            _ => None,
        })
        .unwrap();
    assert!(list.removable);
    let offered: Vec<_> = list
        .add_options
        .as_ref()
        .unwrap()
        .options
        .iter()
        .map(|o| (o.value.as_str(), o.display.as_str()))
        .collect();
    assert_eq!(offered, [("Cobalt", "Cobalt"), ("Iron", "Iron Ore")]);
}

#[test]
fn adding_and_removing_items() {
    let mut doc = doc();
    doc.update(Event::ListAdd((EQUIPMENT.to_owned(), "Iron".to_owned())));
    assert!(doc.is_dirty());
    assert_eq!(rows(&doc.view(), EQUIPMENT), ["Iron Ore"]);

    // Row 1 is Iron (id 11); the unknown id 99 between does not shift rows.
    doc.update(Event::ListRemove((INVENTORY.to_owned(), 1)));
    assert_eq!(rows(&doc.view(), INVENTORY), ["Cobalt"]);
    let save = saved(&doc);
    assert_eq!(
        save.group(4).objects()[0].get("woIds"),
        Some(&json!("10,99"))
    );
    assert_eq!(save.group(4).objects()[1].get("woIds"), Some(&json!("13")));
}

#[test]
fn removing_past_the_end_or_adding_nothing_is_ignored() {
    let mut doc = doc();
    doc.update(Event::ListRemove((INVENTORY.to_owned(), 2)));
    doc.update(Event::ListAdd((INVENTORY.to_owned(), String::new())));
    doc.update(Event::ListAdd(("other".to_owned(), "Iron".to_owned())));
    assert!(!doc.is_dirty());
}

#[test]
fn a_missing_inventory_shows_a_note_and_ignores_edits() {
    let mut doc = doc();
    doc.update(Event::Picked((CHARACTER.to_owned(), "1".to_owned())));
    let tree = doc.view();
    assert!(texts(&tree).contains(&"This inventory is not in the save.".to_owned()));
    assert_eq!(rows(&tree, INVENTORY), ["Titanium"]);
    doc.update(Event::ListAdd((EQUIPMENT.to_owned(), "Iron".to_owned())));
    doc.update(Event::ListRemove((EQUIPMENT.to_owned(), 0)));
    assert!(!doc.is_dirty());
}

#[test]
fn picking_an_unknown_character_keeps_the_selection() {
    let mut doc = doc();
    doc.update(Event::Picked((CHARACTER.to_owned(), "5".to_owned())));
    doc.update(Event::Picked((CHARACTER.to_owned(), "x".to_owned())));
    let tree = doc.view();
    let Input::Pick(pick) = &field(&tree, CHARACTER).input else {
        panic!("character is not a pick-list");
    };
    assert_eq!(pick.selected.as_deref(), Some("0"));
}

#[test]
fn a_save_without_characters_says_so() {
    let mut groups = common::GROUPS;
    groups[2] = &[];
    let mut doc = open(&common::build(&groups));
    assert!(texts(&doc.view()).contains(&"This save has no characters.".to_owned()));
    doc.update(num("player.playerGaugeHealth", 50.0));
    assert!(!doc.is_dirty());
}

#[test]
fn a_save_without_planets_says_so() {
    let mut groups = common::GROUPS;
    groups[1] = &[];
    let mut doc = open(&common::build(&groups));
    let tree = tab(&mut doc, 2);
    assert!(texts(&tree).contains(&"This save has no planets.".to_owned()));
}

#[test]
fn saving_clears_dirty_and_unmodified_saves_are_identical() {
    let mut doc = doc();
    assert_eq!(doc.serialize().unwrap(), common::minimal().into_bytes());
    doc.update(num(TERRA_TOKENS_ID, 1.0));
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());
}

#[test]
fn sample_save_view_fits_the_host_limits() {
    let doc = PlanetCrafterDocument::new(Save::parse(&common::sample()).unwrap(), Names::default());
    let tree = doc.view();
    assert!(tree.nodes.len() < 2_000, "{} nodes", tree.nodes.len());
    assert!(texts(&tree).iter().all(|t| !t.is_empty()));
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p planet-crafter --test localization --test document`
Expected: compile errors, unresolved `planet_crafter::localization` and `planet_crafter::document`.

- [ ] **Step 3: Write `plugins/planet-crafter/src/localization.rs`**

```rust
//! Item display names from the game's `LocalizationSpaceCrafter.txt`
//! (spec §6.1), delivered as the optional `localization` aux file.
//!
//! Lines look like `GROUP_NAME_Iron**Iron*Fer*Eisen`: the key, `**`, then
//! `*`-separated translations with English first.

use std::collections::HashMap;

const PREFIX: &str = "GROUP_NAME_";

/// Item names by `gId`. Empty when the file is missing.
#[derive(Debug, Clone, Default)]
pub struct Names {
    by_gid: HashMap<String, String>,
}

impl Names {
    pub fn parse(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes);
        let mut by_gid = HashMap::new();
        for line in text.lines() {
            let Some((key, values)) = line.split_once("**") else {
                continue;
            };
            let Some(gid) = key.strip_prefix(PREFIX) else {
                continue;
            };
            let english = values.split('*').next().unwrap_or_default().trim();
            if !gid.is_empty() && !english.is_empty() {
                by_gid.insert(gid.to_owned(), english.to_owned());
            }
        }
        Self { by_gid }
    }

    /// The display name of `gid`, or `gid` itself when unknown.
    pub fn name<'a>(&'a self, gid: &'a str) -> &'a str {
        self.by_gid.get(gid).map_or(gid, String::as_str)
    }

    /// `(gId, name)` pairs to offer when adding items, sorted by name
    /// ignoring case. Without names, `fallback` (the save's own `gId`s) is
    /// offered under their raw ids.
    pub fn known_items(&self, fallback: Vec<String>) -> Vec<(String, String)> {
        let mut items: Vec<(String, String)> = if self.by_gid.is_empty() {
            fallback.into_iter().map(|gid| (gid.clone(), gid)).collect()
        } else {
            self.by_gid
                .iter()
                .map(|(gid, name)| (gid.clone(), name.clone()))
                .collect()
        };
        items.sort_by(|a, b| {
            a.1.to_lowercase()
                .cmp(&b.1.to_lowercase())
                .then_with(|| a.0.cmp(&b.0))
        });
        items
    }
}
```

- [ ] **Step 4: Write `plugins/planet-crafter/src/document.rs`**

```rust
//! The editor (spec §6.1 UI table, parity with `PlanetCrafterEditorView`).

use saveforge_sdk::Document;
use saveforge_sdk::ui::{self, El, Event, PickOption, UiTree};
use serde_json::{Number, Value};

use crate::format::Object;
use crate::localization::Names;
use crate::save::{self, DISPLAY_INFO, Item, STATS, Save, TERRA_TOKENS, int, text};

/// Widget ids.
pub const TABS: &str = "tabs";
pub const CHARACTER: &str = "character";
pub const EQUIPMENT: &str = "equipment";
pub const INVENTORY: &str = "inventory";
pub const TERRA_TOKENS_ID: &str = "tokens.terraTokens";
pub const ALL_TIME_TOKENS_ID: &str = "tokens.allTimeTerraTokens";
pub const UNLOCKED_GROUPS_ID: &str = "tokens.unlockedGroups";
pub const DISPLAY_NAME_ID: &str = "info.saveDisplayName";
pub const PLANET_ID_ID: &str = "info.planetId";
pub const CRAFTED_OBJECTS_ID: &str = "stats.craftedObjects";

pub const TAB_TITLES: [&str; 4] = ["Characters", "Terra Tokens", "Planet State", "Save Info"];

/// Player gauges: key (and widget id after `player.`), label, maximum.
const GAUGES: [(&str, &str, f64); 4] = [
    ("playerGaugeHealth", "Health", 100.0),
    ("playerGaugeOxygen", "Oxygen", 370.0),
    ("playerGaugeThirst", "Thirst", 100.0),
    ("playerGaugeToxic", "Toxic", 100.0),
];

/// Planet levels: key (widget id `planet.{index}.{key}`), label.
const LEVELS: [(&str, &str); 6] = [
    ("unitOxygenLevel", "Oxygen"),
    ("unitHeatLevel", "Heat"),
    ("unitPressureLevel", "Pressure"),
    ("unitPlantsLevel", "Plants"),
    ("unitInsectsLevel", "Insects"),
    ("unitAnimalsLevel", "Animals"),
];

const DISPLAY_NAME_MAX: usize = 64;

#[derive(Debug)]
pub struct PlanetCrafterDocument {
    save: Save,
    names: Names,
    /// Options for the add-item pick-lists, computed once at open.
    known_items: Vec<PickOption>,
    tab: u32,
    player: usize,
    dirty: bool,
}

impl PlanetCrafterDocument {
    pub fn new(save: Save, names: Names) -> Self {
        let known_items = names
            .known_items(save.gids())
            .into_iter()
            .map(|(gid, name)| ui::option(gid, name))
            .collect();
        Self {
            save,
            names,
            known_items,
            tab: 0,
            player: 0,
            dirty: false,
        }
    }

    pub fn save(&self) -> &Save {
        &self.save
    }

    fn characters_page(&self) -> El {
        let players = self.save.players();
        let Some(player) = players.get(self.player) else {
            return ui::column(vec![ui::muted("This save has no characters.")]);
        };
        let options = players
            .iter()
            .enumerate()
            .map(|(i, p)| ui::option(i.to_string(), player_name(p)))
            .collect();
        let gauges = GAUGES
            .iter()
            .map(|(key, label, max)| {
                ui::number(format!("player.{key}"), *label, float(player.get(key)))
                    .range(0.0, *max)
                    .decimals(1)
                    .into()
            })
            .collect();
        ui::column(vec![
            ui::pick(
                CHARACTER,
                "Character",
                options,
                Some(self.player.to_string()),
            )
            .into(),
            ui::section("Stats", gauges),
            ui::section("Equipment", self.items_view(EQUIPMENT, player)),
            ui::section("Inventory", self.items_view(INVENTORY, player)),
        ])
    }

    fn items_view(&self, list_id: &str, player: &Object) -> Vec<El> {
        let Some(items) =
            int(player.get(inventory_key(list_id))).and_then(|id| self.save.inventory_items(id))
        else {
            return vec![ui::muted("This inventory is not in the save.")];
        };
        let rows = items
            .iter()
            .map(|item| ui::text(self.names.name(&item.gid)))
            .collect();
        vec![
            ui::list(list_id, rows)
                .removable(true)
                .add_options(self.known_items.clone(), Some("Add item…".to_owned()))
                .into(),
        ]
    }

    fn terra_tokens_page(&self) -> El {
        let tokens = self.save.single(TERRA_TOKENS);
        ui::column(vec![
            ui::number(
                TERRA_TOKENS_ID,
                "Terra Tokens",
                float(tokens.get("terraTokens")),
            )
            .min(0.0)
            .into(),
            ui::number(
                ALL_TIME_TOKENS_ID,
                "All-Time Terra Tokens",
                float(tokens.get("allTimeTerraTokens")),
            )
            .read_only(true)
            .into(),
            ui::text_input(
                UNLOCKED_GROUPS_ID,
                "Unlocked Groups",
                display(tokens.get("unlockedGroups")),
            )
            .read_only(true)
            .into(),
        ])
    }

    fn planet_page(&self) -> El {
        let planets = self.save.planets();
        if planets.is_empty() {
            return ui::column(vec![ui::muted("This save has no planets.")]);
        }
        ui::column(
            planets
                .iter()
                .enumerate()
                .map(|(index, planet)| {
                    let levels = LEVELS
                        .iter()
                        .map(|(key, label)| {
                            ui::number(
                                format!("planet.{index}.{key}"),
                                *label,
                                float(planet.get(key)),
                            )
                            .min(0.0)
                            .decimals(2)
                            .into()
                        })
                        .collect();
                    let name = text(planet.get("planetId"));
                    let title = if name.is_empty() {
                        format!("Planet {}", index + 1)
                    } else {
                        name.to_owned()
                    };
                    ui::section(title, levels)
                })
                .collect(),
        )
    }

    fn save_info_page(&self) -> El {
        let info = self.save.single(DISPLAY_INFO);
        let stats = self.save.single(STATS);
        ui::column(vec![
            ui::text_input(
                DISPLAY_NAME_ID,
                "Display Name",
                display(info.get("saveDisplayName")),
            )
            .max_len(u32::try_from(DISPLAY_NAME_MAX).unwrap_or(u32::MAX))
            .into(),
            ui::text_input(PLANET_ID_ID, "Planet ID", display(info.get("planetId")))
                .read_only(true)
                .into(),
            ui::text_input(
                CRAFTED_OBJECTS_ID,
                "Crafted Objects",
                display(stats.get("craftedObjects")),
            )
            .read_only(true)
            .into(),
        ])
    }

    fn set_number(&mut self, id: &str, value: f64) -> bool {
        if !value.is_finite() || value < 0.0 {
            return false;
        }
        if let Some(key) = id.strip_prefix("player.") {
            let Some((key, _, max)) = GAUGES.iter().find(|(k, ..)| *k == key) else {
                return false;
            };
            let player = self.player;
            return value <= *max
                && self
                    .save
                    .player_mut(player)
                    .is_some_and(|p| set_float(p, key, value));
        }
        if let Some((index, key)) = id
            .strip_prefix("planet.")
            .and_then(|rest| rest.split_once('.'))
        {
            return LEVELS.iter().any(|(k, _)| *k == key)
                && index
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| self.save.planet_mut(index))
                    .is_some_and(|planet| set_float(planet, key, value));
        }
        if id == TERRA_TOKENS_ID {
            return set_whole(self.save.single_mut(TERRA_TOKENS), "terraTokens", value);
        }
        false
    }

    fn set_text(&mut self, id: &str, value: &str) -> bool {
        if id != DISPLAY_NAME_ID {
            return false;
        }
        let value: String = value.chars().take(DISPLAY_NAME_MAX).collect();
        self.save
            .single_mut(DISPLAY_INFO)
            .set("saveDisplayName", Value::String(value))
    }

    fn selected_inventory(&self, list_id: &str) -> Option<i64> {
        let key = match list_id {
            EQUIPMENT | INVENTORY => inventory_key(list_id),
            _ => return None,
        };
        int(self.save.players().get(self.player)?.get(key))
    }

    fn add_item(&mut self, list_id: &str, gid: &str) -> bool {
        if gid.is_empty() {
            return false;
        }
        self.selected_inventory(list_id)
            .and_then(|inventory| self.save.add_item(inventory, gid))
            .is_some()
    }

    fn remove_item(&mut self, list_id: &str, index: u32) -> bool {
        let Some(inventory) = self.selected_inventory(list_id) else {
            return false;
        };
        let item: Option<Item> = self
            .save
            .inventory_items(inventory)
            .and_then(|items| items.into_iter().nth(index as usize));
        item.is_some_and(|item| self.save.remove_item(inventory, item.world_object_id))
    }
}

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
            Event::Picked((id, value)) => {
                if id == CHARACTER
                    && let Ok(index) = value.parse::<usize>()
                    && index < self.save.players().len()
                {
                    self.player = index;
                }
                false
            }
            Event::NumberChanged((id, value)) => self.set_number(&id, value),
            Event::TextChanged((id, value)) => self.set_text(&id, &value),
            Event::ListAdd((id, gid)) => self.add_item(&id, &gid),
            Event::ListRemove((id, index)) => self.remove_item(&id, index),
            Event::Toggled(_) | Event::Pressed(_) => false,
        };
        self.dirty |= changed;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        Ok(self.save.to_bytes())
    }

    fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

fn inventory_key(list_id: &str) -> &'static str {
    if list_id == EQUIPMENT {
        "equipmentId"
    } else {
        "inventoryId"
    }
}

/// `name`, or `name (Host)` for the host.
fn player_name(player: &Object) -> String {
    let name = text(player.get("name"));
    if player.get("host").and_then(Value::as_bool) == Some(true) {
        format!("{name} (Host)")
    } else {
        name.to_owned()
    }
}

fn float(value: Option<&Value>) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(0.0)
}

/// How a value shows in a text field: strings as-is, numbers as written.
fn display(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// Sets a float field unless it already holds `value`, so numbers the
/// player did not change keep their exact text (`100.0` stays `100.0`).
fn set_float(object: &mut Object, key: &str, value: f64) -> bool {
    #[allow(clippy::float_cmp)] // exactly the same number means no edit
    if object.get(key).and_then(Value::as_f64) == Some(value) {
        return false;
    }
    Number::from_f64(value).is_some_and(|n| object.set(key, Value::Number(n)))
}

/// Sets an integer field, rounding `value` to a whole number.
fn set_whole(object: &mut Object, key: &str, value: f64) -> bool {
    const MAX_EXACT: f64 = 9_007_199_254_740_992.0;
    let value = value.round();
    if value.abs() > MAX_EXACT {
        return false;
    }
    #[allow(clippy::cast_possible_truncation)] // whole and within ±2^53
    let value = value as i64;
    if save::int(object.get(key)) == Some(value) {
        return false;
    }
    object.set(key, Value::from(value))
}
```

Update `plugins/planet-crafter/src/lib.rs`:

```rust
//! Planet Crafter save editor (spec §6.1). Built up over the M5 tasks.

pub mod document;
pub mod format;
pub mod localization;
pub mod save;
```

- [ ] **Step 5: Run the tests and lints**

Run: `cargo test -p planet-crafter`
Expected: document 17, format 14, localization 5, save 13, all passing.

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p planet-crafter --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add plugins/planet-crafter
git commit -m "feat(planet-crafter): editor document with localized inventories"
```

---

### Task 4: Plugin entry point, manifest, real saves

**Files:**
- Modify: `plugins/planet-crafter/src/lib.rs` (final), `.github/workflows/ci.yml`
- Create: `plugins/planet-crafter/manifest.json`, `plugins/planet-crafter/icon.png`
- Test: `plugins/planet-crafter/tests/plugin.rs`, `plugins/planet-crafter/tests/real_save.rs`

**Interfaces:**
- Consumes: `document::PlanetCrafterDocument::new`, `save::Save::parse` and `localization::Names::parse`.
- Produces:
  - `planet_crafter::{PlanetCrafter, PRIORITY = 20, LOCALIZATION_AUX = "localization"}`. `PlanetCrafter` implements `saveforge_sdk::Plugin` and is exported with `export_plugin!`.
  - The component at `target/dev-plugins/planet-crafter/1.0.0/` after `cargo xtask plugins`.

- [ ] **Step 1: Manifest and icon**

`plugins/planet-crafter/manifest.json`:

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
    "windows": "%APPDATA%/MijuGames/The Planet Crafter/Saves",
    "linux": "{steam_compat:1284190}/AppData/Roaming/MijuGames/The Planet Crafter/Saves",
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

```bash
cp ../dotnet/src/SaveForge.Plugin.PlanetCrafter/Assets/icon.png plugins/planet-crafter/icon.png
```

- [ ] **Step 2: Write the failing tests**

`plugins/planet-crafter/tests/plugin.rs`:

```rust
//! Detection and opening, as the host calls them.

mod common;

use planet_crafter::document::INVENTORY;
use planet_crafter::{LOCALIZATION_AUX, PRIORITY, PlanetCrafter};
use saveforge_sdk::ui::Widget;
use saveforge_sdk::{AuxFile, Document, Plugin};

fn header(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.len().min(512)]
}

#[test]
fn detects_planet_crafter_saves() {
    let sample = common::sample();
    assert_eq!(
        PlanetCrafter::detect("Standard-1.json", header(&sample)),
        Some(PRIORITY)
    );
    assert_eq!(
        PlanetCrafter::detect("STANDARD-1.JSON", header(&sample)),
        Some(PRIORITY)
    );
}

#[test]
fn ignores_other_files() {
    let sample = common::sample();
    assert_eq!(
        PlanetCrafter::detect("Standard-1.sav", header(&sample)),
        None
    );
    assert_eq!(
        PlanetCrafter::detect("Farm.json", br#"{"player":{}}"#),
        None
    );
    assert_eq!(PlanetCrafter::detect("json", header(&sample)), None);
}

#[test]
fn opens_with_the_localization_aux_file() {
    let aux = vec![AuxFile {
        id: LOCALIZATION_AUX.to_owned(),
        bytes: b"GROUP_NAME_Cobalt**Cobalt Ore*x\n".to_vec(),
    }];
    let doc = PlanetCrafter::open("s.json", common::minimal().into_bytes(), aux).unwrap();
    let tree = doc.view();
    let list = tree
        .nodes
        .iter()
        .find(|n| matches!(&n.widget, Widget::List(l) if l.id == INVENTORY))
        .unwrap();
    let first = &tree.nodes[list.children[0] as usize].widget;
    assert!(matches!(first, Widget::Text(t) if t.text == "Cobalt Ore"));
}

#[test]
fn opening_a_broken_save_reports_why() {
    let err = PlanetCrafter::open("s.json", b"{}@{}".to_vec(), vec![]).unwrap_err();
    assert_eq!(err, "expected at least 11 groups separated by '@', found 2");
}

#[test]
fn current_game_saves_with_a_byte_order_mark_open_and_round_trip() {
    let mut bytes = "\u{feff}".as_bytes().to_vec();
    bytes.extend(common::minimal().into_bytes());
    let header = &bytes[..bytes.len().min(512)];
    assert_eq!(
        PlanetCrafter::detect("Standard-1.json", header),
        Some(PRIORITY)
    );
    let doc = PlanetCrafter::open("Standard-1.json", bytes.clone(), vec![]).unwrap();
    assert_eq!(doc.serialize().unwrap(), bytes);
}

#[test]
fn unmodified_sample_serializes_byte_for_byte() {
    let sample = common::sample();
    let doc = PlanetCrafter::open("Standard-1.json", sample.clone(), vec![]).unwrap();
    assert!(!doc.is_dirty());
    assert_eq!(doc.serialize().unwrap(), sample);
}
```

`plugins/planet-crafter/tests/real_save.rs`:

```rust
//! Real saves (spec §8): `SAVEFORGE_PC_SAVE=<path> cargo test -p
//! planet-crafter -- --ignored`.

use planet_crafter::save::{Save, int};

fn real_save() -> Vec<u8> {
    let path = std::env::var("SAVEFORGE_PC_SAVE")
        .expect("set SAVEFORGE_PC_SAVE to a Planet Crafter save to run this test");
    std::fs::read(path).unwrap()
}

#[test]
#[ignore = "needs SAVEFORGE_PC_SAVE"]
fn real_save_round_trips_byte_for_byte() {
    let bytes = real_save();
    assert_eq!(Save::parse(&bytes).unwrap().to_bytes(), bytes);
}

#[test]
#[ignore = "needs SAVEFORGE_PC_SAVE"]
fn real_save_players_have_inventories() {
    let save = Save::parse(&real_save()).unwrap();
    assert_ne!(save.players().len(), 0);
    for player in save.players() {
        let inventory = int(player.get("inventoryId")).unwrap();
        assert!(save.inventory_items(inventory).is_some());
    }
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test -p planet-crafter --test plugin`
Expected: compile errors, unresolved `planet_crafter::{PlanetCrafter, PRIORITY, LOCALIZATION_AUX}`.

- [ ] **Step 4: Write the final `plugins/planet-crafter/src/lib.rs`**

```rust
//! Planet Crafter save editor (spec §6.1). The save is parsed losslessly
//! ([`format`]), read and edited through [`save`], and shown by
//! [`document::PlanetCrafterDocument`].

pub mod document;
pub mod format;
pub mod localization;
pub mod save;

use std::path::Path;

use saveforge_sdk::{AuxFile, Plugin, export_plugin};

use crate::document::PlanetCrafterDocument;
use crate::localization::Names;
use crate::save::Save;

#[derive(Debug)]
pub struct PlanetCrafter;

/// Detection priority; above the generic sample plugin's 5.
pub const PRIORITY: u32 = 20;

/// Id of the optional localization aux file in `manifest.json`.
pub const LOCALIZATION_AUX: &str = "localization";

impl Plugin for PlanetCrafter {
    type Doc = PlanetCrafterDocument;

    fn detect(file_name: &str, header: &[u8]) -> Option<u32> {
        let is_json = Path::new(file_name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        let header = &header[..header.len().min(512)];
        let marker = br#""terraTokens""#;
        (is_json && header.windows(marker.len()).any(|w| w == marker)).then_some(PRIORITY)
    }

    fn open(
        _file_name: &str,
        bytes: Vec<u8>,
        aux: Vec<AuxFile>,
    ) -> Result<PlanetCrafterDocument, String> {
        let save = Save::parse(&bytes)?;
        let names = aux
            .iter()
            .find(|file| file.id == LOCALIZATION_AUX)
            .map_or_else(Names::default, |file| Names::parse(&file.bytes));
        Ok(PlanetCrafterDocument::new(save, names))
    }
}

export_plugin!(PlanetCrafter);
```

- [ ] **Step 5: Run the tests, build the component, lint**

Run: `cargo test -p planet-crafter`
Expected: document 17, format 14, localization 5, plugin 6 and save 13 pass, with 2 real_save tests ignored.

When a real save is on this machine, run the ignored tests against it. They only read the file.

```bash
SAVEFORGE_PC_SAVE="$USERPROFILE/AppData/LocalLow/MijuGames/Planet Crafter/Standard-1.json" cargo test -p planet-crafter --test real_save -- --ignored
```

Expected: 2 passed. Record the result in the ledger; skip this step on a machine without saves.

Run: `cargo xtask plugins`
Expected: prints `planet-crafter -> …/target/dev-plugins/planet-crafter/1.0.0` (with `manifest.json`, `plugin.wasm` and `icon.png` inside), then `sample-stardew` and `test-plugin`.

In `.github/workflows/ci.yml`, change the wasm clippy step to:

```yaml
      - run: cargo clippy -p saveforge-sdk -p test-plugin -p sample-stardew -p planet-crafter --target wasm32-wasip2 -- -D warnings
```

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p saveforge-sdk -p test-plugin -p sample-stardew -p planet-crafter --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add .github plugins/planet-crafter
git commit -m "feat(planet-crafter): plugin entry point, manifest and icon"
```

---

### Task 5: End-to-end through the wasm build, and a smoke run

**Files:**
- Test: `crates/saveforge-app/tests/planet_crafter_plugin.rs`

**Interfaces:**
- Consumes:
  - `target/dev-plugins/planet-crafter/1.0.0` (Task 4) and `docs/sample-saves/PlanetCrafter-Standard-1.json` (Task 1)
  - `saveforge_app::{documents, editor::Editor, render}` and `common::World`

- [ ] **Step 1: Write the test `crates/saveforge-app/tests/planet_crafter_plugin.rs`**

```rust
//! End to end through the real Planet Crafter wasm component built by
//! `cargo xtask plugins` into `target/dev-plugins`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use iced_test::simulator;
use saveforge_app::documents::{DetectOutcome, detect_file, open_document, save_document};
use saveforge_app::editor::Editor;
use saveforge_app::render::{UiMessage, render};
use saveforge_host::{Event, PluginRegistry};

fn dev_plugins() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace.join("target"), PathBuf::from);
    let dir = target.join("dev-plugins");
    assert!(
        dir.join("planet-crafter").is_dir(),
        "Planet Crafter plugin not built: run `cargo xtask plugins` first ({})",
        dir.display()
    );
    dir
}

fn sample_bytes() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/sample-saves/PlanetCrafter-Standard-1.json"),
    )
    .unwrap()
}

fn open_sample(world: &common::World) -> (Editor, PathBuf) {
    let engine = &world.services.engine;
    let registry = PluginRegistry::scan(engine, &dev_plugins());
    let path = world.dir.path().join("Standard-1.json");
    fs::write(&path, sample_bytes()).unwrap();

    let DetectOutcome::Open(plugin) = detect_file(engine, &registry, &path).unwrap().outcome else {
        panic!("the sample save should match the Planet Crafter plugin");
    };
    assert_eq!(plugin.manifest.id, "planet-crafter");
    let opened = open_document(engine, &plugin, &path, &world.services.tokens).unwrap();
    (Editor::new(opened), path)
}

#[test]
fn planet_crafter_saves_an_unmodified_save_byte_for_byte() {
    let world = common::World::new();
    let (editor, path) = open_sample(&world);
    {
        let mut ui = simulator(render(&editor.tree, &editor.ui));
        assert!(ui.find("Characters").is_ok());
        assert!(ui.find("Health").is_ok());
    }
    assert!(!editor.dirty);

    save_document(&editor.session(), &path, &world.dir.path().join("backups")).unwrap();
    assert_eq!(fs::read(&path).unwrap(), sample_bytes());
}

#[test]
fn planet_crafter_edits_terra_tokens_and_keeps_everything_else() {
    let world = common::World::new();
    let (mut editor, path) = open_sample(&world);
    editor.handle(UiMessage::Plugin(Event::TabSelected((
        "tabs".to_owned(),
        1,
    ))));
    {
        let mut ui = simulator(render(&editor.tree, &editor.ui));
        assert!(ui.find("All-Time Terra Tokens").is_ok());
    }
    editor.handle(UiMessage::NumberEdited {
        id: "tokens.terraTokens".to_owned(),
        text: "15000".to_owned(),
    });
    assert!(editor.dirty);

    let backup =
        save_document(&editor.session(), &path, &world.dir.path().join("backups")).unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    let original = String::from_utf8(sample_bytes()).unwrap();
    assert!(
        saved.starts_with("\r{\"terraTokens\":15000,"),
        "{}",
        &saved[..60]
    );
    let after_tokens = |text: &str| text[text.find('@').unwrap()..].to_owned();
    assert_eq!(after_tokens(&saved), after_tokens(&original));
    assert_eq!(fs::read(backup).unwrap(), sample_bytes());
}
```

- [ ] **Step 2: Prove the test depends on the build**

Run: `rm -rf target/dev-plugins && cargo test -p saveforge-app --test planet_crafter_plugin`
Expected: both tests FAIL with `Planet Crafter plugin not built: run \`cargo xtask plugins\` first`. This is the RED step. The behaviour itself already exists from Tasks 1–4, so this test locks in the integration through the host and the wasm sandbox.

- [ ] **Step 3: Build and run**

Run: `cargo xtask plugins && cargo test -p saveforge-app --test planet_crafter_plugin`
Expected: 2 passed. An unmodified save written through the whole stack equals the sample byte for byte. An edit changes only the TerraTokens group, and the backup equals the original.

- [ ] **Step 4: Smoke-run the app**

Run `cargo build -p saveforge-app`, launch `target/debug/saveforge.exe`, wait about 8 s, capture the window with `PrintWindow` (flag 2), then stop the process:

```powershell
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public class Win { [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
public struct R { public int L, T, Rt, B; } }
'@
$p = Start-Process target\debug\saveforge.exe -PassThru; Start-Sleep 8; $p.Refresh(); $h = $p.MainWindowHandle
$r = New-Object Win+R; [Win]::GetWindowRect($h, [ref]$r) | Out-Null
$bmp = New-Object System.Drawing.Bitmap ($r.Rt - $r.L), ($r.B - $r.T)
$g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc(); [Win]::PrintWindow($h, $dc, 2) | Out-Null; $g.ReleaseHdc($dc)
$bmp.Save("$env:TEMP\m5-smoke.png"); Stop-Process $p -Force
```

Expected:
- The rail shows two plugin buttons: the Planet Crafter icon and **S**.
- With Planet Crafter selected, the save panel shows `%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter`. It lists the saves there (`Backup.json`, `Standard-1.json`) on a machine that has the game, or "No save folder" otherwise.

Don't open or save a real save from the app in this step. Record what the screenshot shows in the ledger.

If the window still points at an old scratch path, rebuild with `cargo clean -p saveforge-core -p saveforge-app` first. `saveforge-core` bakes in `CARGO_MANIFEST_DIR`.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

```bash
git add crates/saveforge-app/tests/planet_crafter_plugin.rs
git commit -m "test(app): open, edit and save through the Planet Crafter wasm plugin"
```

---

### Task 6: Spec corrections, full gate, roadmap

**Files:**
- Modify: `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md`, `docs/superpowers/plans/2026-10-10-roadmap.md`

- [ ] **Step 1: Correct the spec to the real saves**

In `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md`, make these replacements (exact old text → new text).

§4.2 manifest example:

```text
    "windows": "%APPDATA%/MijuGames/The Planet Crafter/Saves",
    "linux": "{steam_compat:1284190}/AppData/Roaming/MijuGames/The Planet Crafter/Saves",
```
→
```text
    "windows": "%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter",
    "linux": "{steam_compat:1284190}/AppData/LocalLow/MijuGames/Planet Crafter",
```

§5.3 settings example:

```text
  "pluginSaveDirectories": { "planet-crafter": "%APPDATA%/MijuGames/The Planet Crafter/Saves" }
```
→
```text
  "pluginSaveDirectories": { "planet-crafter": "%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter" }
```

§5.4 table: add this row after the `Localization probed hardcoded…` row:

```text
| Planet Crafter default save dir was `%APPDATA%/MijuGames/The Planet Crafter/Saves`, where the game does not save | `%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter` (checked against game version 2.008) |
```

§6.1 format bullet. Replace the whole `- **Format:** …` bullet, including its numbered group list, with:

```text
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
```

§6.1 lossless bullet. Replace

```text
- **Lossless round-trip.** Each object is stored as `serde_json::Value` with `preserve_order` and `arbitrary_precision`. The exact separator text between objects in each group and the trailing content are recorded and replayed.
```
→
```text
- **Lossless round-trip.** Each object keeps its original text and is stored as a `serde_json` map with `preserve_order` and `arbitrary_precision`. Only objects that an edit changed are re-serialized (compact, key order and number text kept); the rest are written from their original text. The exact text between objects (whitespace, separators, BOM) is recorded and replayed.
```

§6.1 UI table. Replace the Planet State row

```text
  | Planet State | Oxygen / Heat / Pressure / Plants / Insects / Animals | group 1 `unit{Oxygen,Heat,Pressure,Plants,Insects,Animals}Level` | number | ≥ 0, 2 decimals |
```
→
```text
  | Planet State | Oxygen / Heat / Pressure / Plants / Insects / Animals, one section per planet titled with its `planetId` | group 1 `unit{Oxygen,Heat,Pressure,Plants,Insects,Animals}Level` | number | ≥ 0, 2 decimals |
```

§6.1 localization bullet. Append this sentence to the end of the `- **Localization:** …` bullet:

```text
 Game version 2.008 no longer ships `LocalizationSpaceCrafter.txt` (names live in the Unity assets), so in practice the raw-GId fallback is what users see; the aux file stays optional for older installs.
```

- [ ] **Step 2: Run the full gate exactly as CI does**

Run:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p saveforge-sdk -p test-plugin -p sample-stardew -p planet-crafter --target wasm32-wasip2 -- -D warnings
cargo xtask plugins
cargo test --workspace
cargo test -p saveforge-sdk -p sample-stardew
cargo deny check
cargo machete
```

Expected:
- fmt prints nothing.
- Both clippy runs report 0 warnings.
- `cargo test --workspace`: 211 from M1–M4 plus 55 new (planet-crafter: document 17, format 14, localization 5, plugin 6, save 13) plus 2 app tests = **268 passed**, with 2 ignored (`real_save`).
- `cargo test -p saveforge-sdk -p sample-stardew` passes without `arbitrary_precision` in play: 39 SDK + 6 Stardew = **45 passed**.
- `cargo deny check` prints `advisories ok, bans ok, licenses ok, sources ok`.
- machete reports no unused dependencies.

Compare against the actual sums; any failure stops the gate. If `cargo deny` reports anything new, stop and report the id or license and the crate to the user. No new crates are expected, since `arbitrary_precision` only adds code inside `serde_json`.

- [ ] **Step 3: Mark M5 done in the roadmap**

In `docs/superpowers/plans/2026-10-10-roadmap.md`, change the M5 row to:

```markdown
| 5 | Planet Crafter plugin (lossless round-trip) | `2026-10-16-m5-planet-crafter.md` | done |
```

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md docs/superpowers/plans/2026-10-10-roadmap.md Cargo.lock
git commit -m "docs: correct Planet Crafter spec to real saves; pass full M5 gate"
```

- [ ] **Step 5: Hand off**

Use superpowers:finishing-a-development-branch.
