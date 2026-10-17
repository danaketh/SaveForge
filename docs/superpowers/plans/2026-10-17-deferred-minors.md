# Deferred Minors (M5–M7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix every minor that the M5, M6 and M7 final reviews deferred.

**Architecture:** Small, independent fixes, grouped into five tasks by crate:
- Planet Crafter edits change only what was edited.
- The store's install path survives double failures and antivirus locks.
- The app's update-check feedback is completed.
- `xtask new-plugin` becomes all-or-nothing.
- `xtask package` and the release workflow get safer outputs.

**Tech Stack:** Rust 1.99 (edition 2024, MSRV 1.96), iced 0.14, wasmtime 49, zip 9, Pillow (only to shrink an icon once). No new crates.

**Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md`. Per-item context is in the M5/M6/M7 plans under `docs/superpowers/plans/`.

## Inventory

| From | # | Minor | Task |
|---|---|---|---|
| M5 | 1 | `max + 1` world-object ids can pass `i32::MAX` (the game reads ids as C# `int`) | 1 |
| M5 | 2 | removing a row whose id appears twice in `woIds` removes the first occurrence, not the chosen row | 1 |
| M5 | 3 | add/remove rewrites the whole `woIds` string (spacing, `\|` separators, junk entries lost) | 1 |
| M5 | 4 | editing one field re-serializes the whole object, so other values lose their escape spelling (`\u00e9` → `é`) | 1 |
| M5 | 5 | duplicate keys in an object collapse into one when the object is edited | 1 |
| M5 | 6 | JSON error positions are relative to the object, not the file | 1 |
| M5 | 7 | `icon.png` is 1024×1024 and 1.29 MB | 1 |
| M5 | 8 | open question: does the game accept `max + 1` ids? | not code: reported to the user (no authoritative source found) |
| M6 | 7 | if the final rename and the restore both fail, the dropped holder deletes the old version | 2 |
| M6 | 8 | a failed first install can leave an empty `{id}` dir (shows as a Failed plugin) | 2 |
| M6 | 9 | `file_url` does not escape `%`, so such paths don't round-trip | 2 |
| M6 | 10 | `saveforge-store` depends on `saveforge-plugin-api` (spec: core + host only) | 2 |
| M6 | 11 | "Check now" with no updates gives no feedback; the info bar is never cleared | 3 |
| M6 | 12 | a job started while plugins are loading reports nothing | 3 |
| M6 | 13 | a save can be opened for a plugin whose removal is running | 3 |
| M6 | 14 | no retry when antivirus briefly locks the final rename (Windows) | 2 |
| M6 | 15 | the number of zip entries is not capped | 2 |
| M6 | 16 | danger-style "Remove" text has low contrast | 3 |
| M7 | 3 | ids that clash (workspace members, `fixtures/`, Rust keywords, SDK type names) break the workspace after `Cargo.toml` is edited | 4 |
| M7 | 4 | scaffolding isn't atomic, and its output isn't validated (`--name ""`, `--ext ""`) | 4 |
| M7 | 6 | Linux/macOS release binaries lose `+x`; no download note | 5 |
| M7 | 7 | a failed `xtask package` leaves `target/packages` without `catalog.json` | 5 |
| M7 | 8 | the README copies the CI wasm clippy line (goes stale); the authoring guide promises an icon the scaffold doesn't have | 5 |
| M7 | rec. | the release must fail while `RELEASE_CATALOG_URL` / `REPOSITORY_URL` still say `YOUR_ORG/YOUR_REPO` | 5 |

## Global Constraints

- **Builds on the dev machine:** run every cargo command with `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6`, from a shell at below-normal priority (`(Get-Process -Id $PID).PriorityClass = 'BelowNormal'`). Do **not** write this into any `config.toml`.
- **Branch:** `fix/deferred-minors`, never `main`.
- **Lints:**
  - clippy pedantic `-D warnings`, native and wasm. MSRV 1.96 enables the `Duration` unit lint.
  - Use `assert_eq!`/`assert_ne!` instead of `assert!(x.is_empty())`.
- **Planet Crafter byte-identity:** an unedited save must still round-trip byte for byte, and an edit must change only the edited value's text. The existing round-trip tests stay green unchanged.
- **New world-object ids:** `max + 1`, and only while that is `≤ i32::MAX` (2 147 483 647). Otherwise `add_item` returns `None` and changes nothing.
- **Exact user-facing strings:**

  | Situation | Text |
  |---|---|
  | Manual "Check now", no updates | `All plugins are up to date.` (status) |
  | Job requested while plugins load | `Plugins are still loading.` (status) |
  | Opening a save of a plugin being removed | `{name} is being removed.` (status) |
  | Too many zip entries | `StoreError::BadPackage("more than 10000 entries")` |
  | Scaffold id clashes with a workspace package | `'{id}' is already used by {path}` |
  | Scaffold id is a reserved word | `'{id}' is a reserved word in Rust` |
  | Placeholder URLs at release | `::error::Replace YOUR_ORG/YOUR_REPO in RELEASE_CATALOG_URL and REPOSITORY_URL before releasing` |

- **Limits:** `MAX_ZIP_ENTRIES = 10_000`. Rename retry: 5 attempts, 200 ms apart, only on `PermissionDenied`.

## Review Focus

1. **A Planet Crafter object edited after it was read with odd formatting** (escapes, duplicate keys, spaces around `:`, an empty object). Expected: only the edited value's text changes, and the game reads the result. Tests: `edits_keep_the_spelling_of_untouched_values`, `duplicate_keys_survive_and_the_last_one_is_edited`, `new_keys_are_appended` (Task 1).
2. **An inventory whose `woIds` holds duplicates, junk entries or `|` separators.** Expected: add and remove touch only the chosen token. Tests: `removing_a_duplicated_id_removes_the_chosen_row`, `woids_edits_keep_the_rest_verbatim` (Task 1).
3. **A disk or antivirus failure in the middle of installing an update.** Expected: the old version survives every failure combination, and no empty plugin dir remains. Tests: `failed_restore_keeps_the_old_copy`, `failed_first_install_leaves_no_empty_plugin_dir`, `denied_renames_are_retried` (Task 2).
4. **`new-plugin` with an id that clashes, or with empty or odd flags.** Expected: an error, and the workspace untouched. Tests: `ids_that_clash_are_rejected`, `bad_names_and_extensions_are_rejected`, `scaffold_is_all_or_nothing` (Task 4).
5. **A packaging run that fails half-way.** Expected: the previous `target/packages` (with its `catalog.json`) is still intact. Test: `failed_packaging_keeps_the_previous_output` (Task 5).

---

### Task 1: Planet Crafter: minimal edits, safe ids, absolute errors, smaller icon

**Files:**
- Modify: `plugins/planet-crafter/src/format.rs`, `src/save.rs`, `src/document.rs`, `icon.png`
- Test: `plugins/planet-crafter/tests/format.rs`, `tests/save.rs`, `tests/plugin.rs`

**Interfaces:**
- **`format::Object`:**
  - Always writes `raw`.
  - `set(key, value)` splices the new value's compact JSON into `raw`, in place of the **last** top-level occurrence of `key`. A missing key is appended before the closing `}`, as `,"key":value` (no comma for `{}`). `fields` stays in step.
  - `Object::new(fields)` builds `raw` from the compact JSON.
  - A private scanner, `fn members(raw: &str) -> Vec<(String, Range<usize>)>`, returns the decoded key and value byte range of each top-level member. It skips strings with their escapes, and nested objects and arrays.
- **`format::SaveFile::parse` errors:** report positions in the file. The message is `{group} object {n} is not valid JSON at line {L} column {C}: {reason}`. `L`/`C` are counted from the start of the file (1-based, as serde reports them). `reason` is serde's message without its own ` at line … column …` suffix.
- **`save::Save`:**
  - `add_item(inventory_id, gid) -> Option<i64>`:
    - The new id is `max + 1`, or `None` when that exceeds `i32::MAX`.
    - `woIds` gets `{sep}{id}` appended, where `sep` is `,` if the string has one, else `|` if it has one, else `,`. An empty string becomes `{id}`.
  - `remove_item_at(inventory_id, index: usize) -> bool` replaces `remove_item`:
    - `index` counts the rows `inventory_items` returns.
    - It removes exactly that token, plus its preceding separator (its following one when it is first).
    - Every other character of `woIds` stays.
  - A private `fn wo_id_tokens(s: &str) -> Vec<(Range<usize>, Option<i64>)>` splits on `,`/`|`, keeping ranges.
- **`document.rs`:** `remove_item` calls `Save::remove_item_at`.
- **`icon.png`:** resized to 256×256 RGB PNG (Pillow `Image.LANCZOS`, `optimize=True`).

- [ ] **Step 1: Create the branch**

```bash
git checkout -b fix/deferred-minors
```

- [ ] **Step 2: Write the failing tests**

`tests/format.rs`:

```rust
#[test] fn edits_keep_the_spelling_of_untouched_values()   // save whose TerraTokens object is {"a":"caf\u00e9","terraTokens":1} (escape written literally);
                                                           // single_mut(0).set("terraTokens", 2) → to_text contains {"a":"caf\u00e9","terraTokens":2}
#[test] fn duplicate_keys_survive_and_the_last_one_is_edited() // {"x":1,"x":2}: get("x")==2; set("x",3) → raw {"x":1,"x":3}
#[test] fn new_keys_are_appended()                         // { "a": 1 } set a=5 → { "a": 5 }; {"a":1} set b=true → {"a":1,"b":true}; {} set b=true → {"b":true}
#[test] fn errors_report_positions_in_the_file()          // a save whose WorldObjects 2nd object (on line 7) is {"id":1,} → Err contains "WorldObjects object 2 is not valid JSON at line 7 column"
```

`tests/save.rs`:

```rust
#[test] fn removing_a_duplicated_id_removes_the_chosen_row() // woIds "5,7,5" (both world objects exist) → remove_item_at(inv, 2) → woIds "5,7"
#[test] fn woids_edits_keep_the_rest_verbatim()           // woIds "5, 7|x,9" → add_item → "5, 7|x,9,<new>"; then remove_item_at(inv, 1) (the " 7") → "5|x,9,<new>"
#[test] fn new_ids_stay_within_i32()                       // world object id 2147483647 → add_item == None; woIds and WorldObjects unchanged
```

Update `add_item_rewrites_wo_ids_comma_joined`: its old expectation (normalizing `woIds`) is the M5 behaviour being fixed. It now asserts that the original text is kept and `,{id}` is appended. Record this as a ruling.

`tests/plugin.rs`:

```rust
#[test] fn icon_is_at_most_256_pixels()                    // read icon.png bytes 16..24 (IHDR width, height, big-endian) → both <= 256; file < 200_000 bytes
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test -p planet-crafter`
Expected: compile errors for `remove_item_at`, then the new assertions fail.

- [ ] **Step 4: Implement it, and resize the icon**

```bash
python -c "from PIL import Image; im=Image.open('plugins/planet-crafter/icon.png'); im.resize((256,256), Image.LANCZOS).save('plugins/planet-crafter/icon.png', optimize=True)"
```

- [ ] **Step 5: Run the tests and lints**

Run:
- `cargo test -p planet-crafter`
- `cargo xtask plugins && cargo test -p saveforge-app --test planet_crafter_plugin`
- clippy native and wasm for `planet-crafter`

Expected: all pass, including the unchanged byte-identity round trips; no warnings.

- [ ] **Step 6: Commit**

`git commit -m "fix(planet-crafter): minimal edits, exact woIds changes, i32 ids, file-relative errors, smaller icon"`

---

### Task 2: Store and core: resilient install, `%` in file URLs, dependency rule

**Files:**
- Modify:
  - `crates/saveforge-core/src/settings.rs`
  - `crates/saveforge-host/src/lib.rs` (re-export `API_VERSION`)
  - `crates/saveforge-store/Cargo.toml` (drop `saveforge-plugin-api`)
  - `crates/saveforge-store/src/install.rs`
- Test: `crates/saveforge-core/tests/settings.rs`, `crates/saveforge-store/tests/install.rs`

**Interfaces:**
- **`file_url`** escapes `%` as `%25`, so `path_from_file_url(file_url(p)) == p` for paths containing `%`.
- **`saveforge_host::API_VERSION`**: `pub use saveforge_plugin_api::API_VERSION;`. The store uses it.
- **`saveforge_store::install`:**
  - `MAX_ZIP_ENTRIES: usize = 10_000`. `extract_zip` fails with `BadPackage("more than 10000 entries")` when `archive.len()` exceeds it.
  - `move_into_place(staged: &Path, target: &Path, staging: &Path, rename: &mut dyn FnMut(&Path, &Path) -> io::Result<()>) -> Result<(), StoreError>`:
    - The final step of `install`; `install` passes `retry_rename`.
    - Moves an existing `target` aside into staging, then renames `staged` to `target`.
    - On failure:
      - moves the old copy back; if that also fails, **keeps** it (`TempDir::keep`) and logs its path at `error`
      - removes `target`'s parent `{id}` dir if it is empty
  - `retry_rename(from: &Path, to: &Path) -> io::Result<()>` makes up to 5 attempts, 200 ms apart, retrying only on `ErrorKind::PermissionDenied`.

- [ ] **Step 1: Write the failing tests**

```rust
// settings.rs
#[test] fn percent_signs_round_trip()                     // p = "C:/Users/a%20b/100%/save.json" (and a Unix one) → file_url contains "100%25", round-trips to the same PathBuf
// install.rs
#[test] fn too_many_entries_are_rejected()               // zip of 10_001 empty stored entries → extract_zip Err BadPackage == "more than 10000 entries"
#[test] fn failed_first_install_leaves_no_empty_plugin_dir() // move_into_place with a rename that always fails, target plugins/demo/1.0.0 absent → Err; plugins/demo does not exist
#[test] fn failed_restore_keeps_the_old_copy()           // existing target with old.txt; rename: 1st Ok (real fs::rename), 2nd Err, 3rd Err → Err; a file named old.txt still exists under staging
#[test] fn denied_renames_are_retried()                  // retry(5, 1 ms, op): op fails PermissionDenied twice then Ok → Ok after 3 calls;
                                                         // op fails NotFound → Err after 1 call; op always PermissionDenied → Err after 5 calls
```

`retry` is `pub fn retry<T>(attempts: u32, delay: Duration, op: impl FnMut() -> io::Result<T>) -> io::Result<T>` in `install.rs`; `retry_rename` calls it with `5` and `Duration::from_millis(200)`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p saveforge-core --test settings` and `cargo test -p saveforge-store --test install`
Expected: compile errors (`move_into_place`, `retry`, `MAX_ZIP_ENTRIES`), then a failure on `100%25`.

- [ ] **Step 3: Implement it, and move the store off `saveforge-plugin-api`**

- [ ] **Step 4: Run tests, lints and the dependency check**

Run:
- `cargo test -p saveforge-core -p saveforge-store`
- `cargo tree -p saveforge-store -e normal --depth 1`
- `cargo machete`

Expected: all pass; the tree has no `saveforge-plugin-api` line; machete is clean.

- [ ] **Step 5: Commit**

`git commit -m "fix(store): keep the old version on double failure, retry locked renames, cap zip entries; escape % in file URLs"`

---

### Task 3: App: update-check feedback, job and open guards, readable danger buttons

**Files:**
- Modify: `crates/saveforge-app/src/app/mod.rs`, `src/theme.rs`, `src/app/store_view.rs`, `src/app/view.rs`
- Test: `crates/saveforge-app/tests/store.rs`, `tests/settings_about.rs` (the `UpdatesChecked` literals gain a field)

**Interfaces:**
- `Message::UpdatesChecked { url: String, manual: bool, result }`: `CheckUpdatesNow` sends `manual: true`, and startup sends `false`.
- **On `Ok`:**
  - 0 updates: clears `info`. A manual check sets the status `All plugins are up to date.`; otherwise it clears `Checking…` as before.
  - Some updates: sets the info bar as before.
- **After a successful `Installed` job:** if `info` is showing and the catalog is `Ready`, recount with `catalog.updates(&registry.installed_versions(), &HOST_VERSION)`. Zero clears `info`; otherwise it is reworded with the new count.
- **`start_job` while `Plugins::Loading`:** sets the status `Plugins are still loading.` and starts nothing.
- **`start_open`:** refuses when a running job is a `Remove` of that plugin, with the status `{name} is being removed.`. It clears `pending_open` and `busy`.
- **`theme::danger_button(theme: &Theme, status: button::Status) -> button::Style`:**
  - background `RED`, lightened 10 % on hover
  - text `TEXT`
  - disabled: background `BG3`, text `TEXT_FAINT`

  It replaces `button::danger` in `store_view.rs` and `view.rs`.

- [ ] **Step 1: Write the failing tests (`tests/store.rs`)**

```rust
#[test] fn manual_check_with_no_updates_says_so()          // CheckUpdatesNow; feed UpdatesChecked{url, manual:true, Ok((load, vec![]))} → status "All plugins are up to date."
#[test] fn clean_check_clears_the_info_bar()               // UpdatesChecked with 1 update → info Some; then manual:false with 0 → info None
#[test] fn updating_the_last_outdated_plugin_clears_the_info_bar() // catalog lists test-plugin 0.2.0; info from 1 update; install fixture as test-plugin 0.2.0 (rewrite manifest version);
                                                           // UpdatePlugin("test-plugin") then JobFinished Installed{entry: load_entry(..), replaced: Some(0.1.0)} → info None
#[test] fn jobs_wait_for_plugins_to_load()                 // App::new without PluginsLoaded; feed CatalogLoaded; InstallPlugin("other") → job None; status "Plugins are still loading."
#[test] fn opening_a_save_of_a_plugin_being_removed_is_refused() // RemovePlugin("test-plugin") (job runs); OpenSave + Detected(Open test-plugin) → editor None; status "Test Plugin is being removed."
#[test] fn danger_buttons_have_readable_text()             // theme::danger_button(&theme::theme(), button::Status::Active): text_color == theme::TEXT, background == Some(Background::Color(theme::RED))
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p saveforge-app --test store`
Expected: compile errors (`manual`, `danger_button`), then assertion failures.

- [ ] **Step 3: Implement it**

- [ ] **Step 4: Run the tests and lints**

Run: `cargo test -p saveforge-app` and the lint gate.
Expected: all pass; no warnings.

- [ ] **Step 5: Commit**

`git commit -m "fix(app): update-check feedback, guard jobs during startup and opens during removal, readable danger buttons"`

---

### Task 4: `xtask new-plugin`: clash checks, validated output, all-or-nothing

**Files:**
- Modify: `xtask/src/scaffold.rs`, `xtask/src/main.rs`, `xtask/templates/lib.rs.tmpl`, `xtask/templates/test.rs.tmpl`
- Test: `xtask/tests/scaffold.rs`

**Interfaces:**
- **`NewPlugin::new`:** also rejects Rust keywords, including those reserved in edition 2024 (`gen`), plus `self`, `super`, `crate`, `std`, `core`, `alloc`, `test` and `proc_macro`, with `'{id}' is a reserved word in Rust`. Compare the crate name, i.e. `-` → `_`.
- **`NewPlugin::validate(&self) -> anyhow::Result<()>`:**
  - `name` and `author` are non-empty and contain no `"`, `\` or newline.
  - `extension` matches `^\.[A-Za-z0-9]+$`.

  The checks move out of `main.rs`.
- **`scaffold`, in order:**
  1. Validate.
  2. Refuse an id used by a workspace member: any `members` entry whose last path component equals the id, or an existing `plugins/{id}` or `fixtures/{id}`. The error is `'{id}' is already used by {path}`.
  3. Compute the new `Cargo.toml` (and `ci.yml`).
  4. Render all files, and run `PluginManifest::parse` on the rendered manifest.
  5. Write them into `tempfile::Builder::new().prefix(".new-").tempdir_in(plugins/)` and rename that dir to `plugins/{id}`.
  6. Then write `Cargo.toml` and `ci.yml`.

  Any error before the rename leaves the workspace untouched.
- **Templates:** use fully qualified `saveforge_sdk::` paths and anonymous trait imports (`use saveforge_sdk::{Document as _, Plugin as _};`). The plugin's type name then cannot clash with an SDK name: ids `plugin`, `form`, `document`, `widget` and `json-codec` all compile.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn ids_that_clash_are_rejected()                   // NewPlugin::new("async"), ("gen"), ("std") → Err "'async' is a reserved word in Rust";
                                                           // temp workspace (members crates/a, plugins/b, xtask; dir fixtures/test-plugin): scaffold "xtask" → Err "'xtask' is already used by xtask";
                                                           // "b" → Err mentions "plugins/b"; "test-plugin" → Err mentions "fixtures/test-plugin"; Cargo.toml unchanged
#[test] fn bad_names_and_extensions_are_rejected()         // name "", name "a\nb", author "", extension "", ".", "a b", ".s v" → scaffold Err; plugins/my-game absent; Cargo.toml unchanged
#[test] fn scaffold_is_all_or_nothing()                    // a *file* named plugins in the temp workspace → scaffold Err; Cargo.toml and ci.yml unchanged; no .new-* left anywhere
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p xtask --test scaffold`
Expected: compile errors (`validate`), then the clash and validation assertions fail.

- [ ] **Step 3: Implement it**

- [ ] **Step 4: Run the tests, then prove that clashing type names compile**

Scaffold each of these into the real workspace, then run its tests and clippy for native and wasm, then remove it:
- `plugin`
- `form --ext .Sav --name "Form Editor"`

Revert with `git checkout -- Cargo.toml Cargo.lock .github/workflows/ci.yml` and delete `plugins/plugin`, `plugins/form` and their `target/dev-plugins` entries.
Expected: 2 tests pass for each; clippy is clean; `git status` is clean afterwards.

- [ ] **Step 5: Commit**

`git commit -m "fix(xtask): reject clashing ids, validate flags and the rendered manifest, scaffold atomically"`

---

### Task 5: `xtask package` staging, release tarballs and URL guard, doc drifts

**Files:**
- Modify: `xtask/src/package.rs`, `xtask/src/main.rs`, `.github/workflows/release.yml`, `README.md`, `docs/PLUGIN_AUTHORING.md`
- Test: `xtask/tests/package.rs`, `xtask/tests/release.rs`

**Interfaces:**
- **`package::write_packages(out: &Path, plugins: &[(PathBuf, PathBuf)], base_url: Option<&str>) -> anyhow::Result<(Vec<Package>, String)>`:**
  - `plugins` holds `(crate_dir, wasm)` pairs.
  - Builds every zip and `catalog.json` into `{out}.tmp` (cleared first). Only when everything succeeds does it remove `out` and rename `{out}.tmp` to `out`.
  - Returns the packages and the catalog JSON.
  - The default base URL (`file_url(out)` + `/`) uses the **final** `out` path.
  - `main.rs` calls it.
- **`release.yml`:**
  - The Linux and macOS matrix entries ship `saveforge-linux-x64.tar.gz` and `saveforge-macos-arm64.tar.gz`, built with `tar -czf dist/{asset} -C target/release saveforge`, which keeps `+x`. Windows stays `.exe`.
  - The matrix gets a `pack` key (`tar` / `copy`) used by the packaging step.
  - The `plugins` job gets a first step, run only on tag pushes (`if: github.event_name == 'push'`), that fails with the exact `::error::` text from Global Constraints when `grep -q "YOUR_ORG/YOUR_REPO"` matches `crates/saveforge-core/src/settings.rs` or `crates/saveforge-app/src/app/mod.rs`.
- **`README.md`:**
  - A "Downloading a release" note: unpack the `.tar.gz`. On macOS, the unsigned binary needs `xattr -d com.apple.quarantine saveforge` or right-click → Open.
  - The quality-gate block names the wasm clippy step by reference ("the wasm clippy line in `.github/workflows/ci.yml`") instead of copying its package list.
- **`docs/PLUGIN_AUTHORING.md` §8:** "the component and the icon, if the manifest names one".

- [ ] **Step 1: Write the failing tests**

```rust
// package.rs
#[test] fn failed_packaging_keeps_the_previous_output()    // out has marker.txt; write_packages(out, [ok crate, crate whose wasm is missing]) → Err; marker.txt still there; no {out}.tmp left
#[test] fn write_packages_replaces_the_output()            // out has marker.txt; write_packages(out, [ok crate], None) → Ok; out has demo-1.2.3.zip and catalog.json, no marker.txt;
                                                           // catalog packageUrl starts with file_url(out) (not ".tmp")
// release.rs
#[test] fn unix_binaries_ship_as_tarballs()                // contains "saveforge-linux-x64.tar.gz", "saveforge-macos-arm64.tar.gz", "tar -czf"
#[test] fn releases_refuse_placeholder_urls()              // contains "YOUR_ORG/YOUR_REPO" guard with the exact ::error:: text and "exit 1"
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p xtask`
Expected: compile errors (`write_packages`), then the two release assertions fail.

- [ ] **Step 3: Implement it, update the workflow and docs**

- [ ] **Step 4: Verify**

Run:
- `cargo test -p xtask`
- `cargo xtask package`
- the PyYAML check (`release.yml` parses; the jobs are `app`, `plugins`, `publish`; the matrix has 3 entries)
- `rg -n -i "dotnet|TODO|-p saveforge-sdk -p" README.md docs/*.md`

Expected:
- tests pass
- `package` prints two zips with the same SHA-256s as before (`921d08b2…`, `fd646e67…`), unless Task 1 changed the Planet Crafter zip — it does, through the icon and code
- the YAML check prints `ok`
- rg finds no matches

- [ ] **Step 5: Full gate**

Run: fmt check, clippy native + wasm, `cargo xtask plugins`, `cargo test --workspace`, `cargo deny check`, `cargo machete`.
Expected: clean; **386 passed** (360 + 26), 2 ignored. The 26 are Task 1: 8, Task 2: 5, Task 3: 6, Task 4: 3, Task 5: 4.

- [ ] **Step 6: Commit**

`git commit -m "fix: stage xtask package output, ship unix binaries as tarballs, guard placeholder URLs at release, doc drifts"`

Then use superpowers:finishing-a-development-branch.
