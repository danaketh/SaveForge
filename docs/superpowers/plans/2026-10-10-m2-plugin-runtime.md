# Milestone 2: Plugin Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the plugin runtime end to end:
- the WIT contract
- host bindings plus UI-tree validation (`saveforge-plugin-api`)
- the guest SDK with UI builders (`saveforge-sdk`)
- a fixture plugin that can provoke every fault
- `cargo xtask plugins`
- `saveforge-host`, which loads, sandboxes and runs plugins. It handles open, view, update, serialize, timeouts, memory limits, aux files, the registry and detection.

**Architecture:** One `wit/plugin.wit` is shared by both sides.
- **Host side** (`saveforge-plugin-api`, `saveforge-host`): wasmtime 49's `bindgen!`, a WASI p2 context with no capabilities, and epoch interruption driven by a 10 ms ticker thread. Each open document gets its own `Store` with `StoreLimits`.
- **Guest side** (`saveforge-sdk`): wit-bindgen 0.62. Plugin authors implement the SDK's `Plugin`/`Document` traits. `export_plugin!` wires them through a generic adapter.
- **Fixture:** a fixture plugin, built to `wasm32-wasip2` by `xtask`, is the real component the host tests run against.

**Tech Stack:** Rust 1.99 (edition 2024), wasmtime 49.0.2, wasmtime-wasi 49.0.2, wit-bindgen 0.62.0, tokio 1.53.2 (only for the `AsyncWrite` trait used by the stdout adapter), anyhow (xtask only).

**Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md` (§3.1–3.4, §4.1–4.6, §7 plugin faults, §8 `saveforge-host`/`saveforge-sdk` rows, §9 CI, §10 `xtask plugins`)

**Prior milestone:** M1 is merged. `saveforge-core` provides:
- `manifest::{PluginManifest, AuxFileSpec}`, `PluginManifest::{load, check_compat, handles_extension}`
- `tokens::{TokenEnv, Expanded, expand}`
- `paths::AppPaths`
- `CoreError::{Io, Json, Manifest, Incompatible}`

## Global Constraints

- **Pinned versions:** `wasmtime = 49.0.2` with `default-features = false` and features `runtime, cranelift, component-model, std, parallel-compilation, cache`. `wasmtime-wasi = 49.0.2`, `wit-bindgen = 0.62.0`, `tokio = 1.53.2` (`default-features = false`), `anyhow = 1.0.104`.
- **WIT package:** `saveforge:plugin@0.1.0`. `API_VERSION = 0.1.0`. Manifests use `"apiVersion": "0.1"`.
- **Host version:** `HOST_VERSION` is `saveforge-host`'s `CARGO_PKG_VERSION`, currently `0.1.0`. Fixture manifests use `"minHostVersion": "0.1.0"`.
- **Default `HostLimits`:** `ui_call = 250 ms` (view/update/is-dirty/mark-saved/drop), `io_call = 10 s` (instantiate/detect/open/serialize), `memory_bytes = 512 MiB`. The epoch tick is `10 ms`.
- **WASI context:** no preopens, no env, no args, no sockets. Guest stdout/stderr go to `tracing::debug!` with fields `plugin` and `stream`.
- **UI tree cap:** 20,000 nodes.
- **`detect` header:** at most 512 bytes. Detection only calls plugins whose manifest `handles_extension(path)`.
- **Session faults:** after 3 faults in one session, the registry marks the plugin `Failed`.
- **Compile cache:** wasmtime's built-in cache (`Config::cache`), keyed by wasmtime version and module hash, in a directory the caller passes (the app passes `AppPaths::wasm_cache_dir()` = `<data_dir>/cache/wasmtime`).
  - **Deviation from spec §3.4:** the spec says `.cwasm` next to `plugin.wasm`. The built-in cache gives the same invalidation without `unsafe` `Component::deserialize`.
- **Lints:** `unsafe_code = "forbid"` stays workspace-wide; the spike showed generated bindings do not trip it. Generated binding modules carry `#[allow(clippy::all, clippy::pedantic)]`, because wit-bindgen output trips `clippy::same_length_and_capacity`.
- **Windows builds** in Git Bash need `CC`/`CXX` unset (`env -u CC -u CXX cargo …`). The user's shell exports a MinGW `CC`.
- **Tests:** tests live in `<crate>/tests/*.rs`. Host tests need the fixture built first: `cargo xtask plugins`.
- **Branch:** work on `feat/m2-plugin-runtime`, never on `main`.
- **Clippy 1.99:** `assert!(x.is_empty())` is rejected (`assert_is_empty`). Use `assert_eq!(x, …)`.

## Review Focus

1. **Dropping a session after a timeout.** Expected: `Drop` does not call back into the poisoned guest, so it returns immediately. Covered in Task 6 (`dropping_poisoned_session_is_immediate`).
2. **A header longer than 512 bytes passed to detection.** Expected: it is truncated before the plugin sees it. The fixture refuses headers over 512 bytes, so a match proves truncation. Covered in Task 8 (`long_header_is_truncated_to_512_bytes`).
3. **Version folders that aren't SemVer** (`latest`, `1.0`) or dot-folders (`.staging`). Expected: ignored, never a crash. Covered in Task 8 (`non_semver_and_dot_folders_are_ignored`).
4. **A plugin that keeps crashing.** Expected: on the 3rd fault the registry disables it and detection stops offering it. Covered in Task 8 (`third_fault_disables_plugin`).
5. **An aux file bigger than `maxBytes`.** Expected: that candidate is skipped and the next one is tried. If none fits, a required file is "missing". Covered in Task 7 (`oversized_candidate_falls_through`).

---

## File Structure

```
rust/
  Cargo.toml                                  # + members, workspace deps
  .cargo/config.toml                          # cargo xtask alias
  .github/workflows/ci.yml                    # + wasm target, xtask step, wasm clippy
  wit/plugin.wit                              # the contract (spec §4.4 + mark-saved)
  crates/saveforge-core/src/paths.rs          # + wasm_cache_dir()
  crates/saveforge-plugin-api/
    src/lib.rs                                # bindgen!, re-exports, API_VERSION
    src/tree.rs                               # validate_tree (spec §4.5)
    tests/tree.rs
  sdk/saveforge-sdk/
    src/lib.rs                                # bindings, Plugin/Document traits, export_plugin!
    src/adapter.rs                            # generic Guest impl
    src/log.rs                                # log() + info!/warn!/error!/debug!
    src/ui.rs                                 # El builders, col!/row!, into_tree
    tests/export.rs
    tests/ui.rs
  fixtures/test-plugin/
    Cargo.toml  manifest.json  src/lib.rs     # fault-provoking fixture
  xtask/
    src/lib.rs  src/main.rs                   # cargo xtask plugins
    tests/xtask.rs
  crates/saveforge-host/
    src/lib.rs
    src/error.rs                              # HostError, PluginFault, classify
    src/engine.rs                             # HostLimits, PluginEngine, LoadedPlugin, State
    src/stdio.rs                              # guest stdout/stderr -> tracing
    src/session.rs                            # DocumentSession, detect/open
    src/aux.rs                                # resolve_aux_files
    src/registry.rs                           # PluginRegistry scan, faults
    src/detect.rs                             # detection voting
    tests/common/mod.rs
    tests/engine.rs  tests/session.rs  tests/aux.rs  tests/registry.rs  tests/detect.rs
```

---

### Task 1: WIT contract + `saveforge-plugin-api` with tree validation

**Files:**
- Create: `wit/plugin.wit`
- Create: `crates/saveforge-plugin-api/Cargo.toml`, `src/lib.rs`, `src/tree.rs`
- Modify: `Cargo.toml` (members, workspace deps)
- Test: `crates/saveforge-plugin-api/tests/tree.rs`

**Interfaces:**
- Produces:
  - `saveforge_plugin_api::{PluginWorld, PluginWorldPre}` (generated `Plugin`/`PluginPre`)
  - `AuxFile { id: String, bytes: Vec<u8> }`
  - `Level { Trace, Debug, Info, Warn, Error }`
  - traits `LogHost` (`fn log(&mut self, level: Level, msg: String)`) and `UiHost` (empty)
  - module `ui` with `UiTree { nodes: Vec<Node> }`, `Node { widget: Widget, children: Vec<u32> }`, `Widget::{Column, Row, Scrollable, Divider, Section(String), Tabs(Tabs), Text(TextSpan), Field(Field), Button(Button), List(ListSpec)}`, `Event::{TextChanged((String,String)), NumberChanged((String,f64)), Toggled((String,bool)), Picked((String,String)), Pressed(String), TabSelected((String,u32)), ListAdd((String,String)), ListRemove((String,u32))}` and the record types. All derive `Clone, Debug, PartialEq`.
  - `API_VERSION: semver::Version = 0.1.0`
  - `MAX_TREE_NODES: usize = 20_000`
  - `validate_tree(&ui::UiTree) -> Result<(), String>`

- [ ] **Step 1: Create the branch and add the wasm target**

```bash
git checkout -b feat/m2-plugin-runtime
rustup target add wasm32-wasip2
```

- [ ] **Step 2: Write `wit/plugin.wit`**

```wit
package saveforge:plugin@0.1.0;

interface ui {
  /// WIT has no recursive types: the tree is a flat arena. nodes[0] is the root.
  type node-id = u32;

  record ui-tree { nodes: list<node> }
  record node { widget: widget, children: list<node-id> }

  enum text-style { body, muted, heading, error }
  record text-span { text: string, style: text-style }

  /// Plugin owns the selected tab; the node's single child is the active page.
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

- [ ] **Step 3: Update workspace `Cargo.toml`**

Replace the `members` line and add the dependencies, keeping the existing entries sorted:

```toml
[workspace]
resolver = "3"
members = ["crates/saveforge-core", "crates/saveforge-plugin-api"]
```

```toml
[workspace.dependencies]
anyhow = "1.0.104"
directories = "6.0.0"
filetime = "0.2.29"
saveforge-core = { path = "crates/saveforge-core" }
saveforge-plugin-api = { path = "crates/saveforge-plugin-api" }
semver = { version = "1.0.28", features = ["serde"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
tempfile = "3.27.0"
thiserror = "2.0.21"
time = { version = "0.3.55", features = ["formatting", "macros"] }
tokio = { version = "1.53.2", default-features = false }
tracing = "0.1.44"
wasmtime = { version = "49.0.2", default-features = false, features = [
    "runtime",
    "cranelift",
    "component-model",
    "std",
    "parallel-compilation",
    "cache",
] }
wasmtime-wasi = "49.0.2"
winreg = "0.56.0"
wit-bindgen = "0.62.0"
```

- [ ] **Step 4: Write `crates/saveforge-plugin-api/Cargo.toml`**

```toml
[package]
name = "saveforge-plugin-api"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
semver.workspace = true
wasmtime.workspace = true

[lints]
workspace = true
```

- [ ] **Step 5: Write the failing tests `crates/saveforge-plugin-api/tests/tree.rs`**

```rust
use saveforge_plugin_api::ui::{
    Button, ButtonStyle, Node, Tabs, TextSpan, TextStyle, UiTree, Widget,
};
use saveforge_plugin_api::{API_VERSION, MAX_TREE_NODES, validate_tree};

fn node(widget: Widget, children: &[u32]) -> Node {
    Node {
        widget,
        children: children.to_vec(),
    }
}

fn text(s: &str) -> Widget {
    Widget::Text(TextSpan {
        text: s.to_owned(),
        style: TextStyle::Body,
    })
}

fn button(id: &str) -> Widget {
    Widget::Button(Button {
        id: id.to_owned(),
        label: id.to_owned(),
        style: ButtonStyle::Primary,
        enabled: true,
    })
}

fn tabs(id: &str, titles: usize, selected: u32) -> Widget {
    Widget::Tabs(Tabs {
        id: id.to_owned(),
        titles: (0..titles).map(|i| format!("Tab {i}")).collect(),
        selected,
    })
}

fn tree(nodes: Vec<Node>) -> UiTree {
    UiTree { nodes }
}

fn err(t: &UiTree) -> String {
    validate_tree(t).expect_err("tree should be invalid")
}

#[test]
fn accepts_well_formed_tree() {
    // 0 column -> [1 text, 2 button a, 3 tabs t -> [4 column -> [5 button b]]]
    let t = tree(vec![
        node(Widget::Column, &[1, 2, 3]),
        node(text("hi"), &[]),
        node(button("a"), &[]),
        node(tabs("t", 2, 1), &[4]),
        node(Widget::Column, &[5]),
        node(button("b"), &[]),
    ]);
    assert_eq!(validate_tree(&t), Ok(()));
}

#[test]
fn rejects_empty_tree() {
    assert!(err(&tree(vec![])).contains("root"));
}

#[test]
fn rejects_bad_child_indices() {
    assert!(err(&tree(vec![node(Widget::Column, &[5])])).contains("invalid child"));
    assert!(err(&tree(vec![node(Widget::Column, &[0])])).contains("invalid child"));
}

#[test]
fn rejects_node_with_two_parents() {
    let t = tree(vec![
        node(Widget::Column, &[1, 2]),
        node(Widget::Row, &[3]),
        node(Widget::Row, &[3]),
        node(text("x"), &[]),
    ]);
    assert!(err(&t).contains("two parents"));
}

#[test]
fn rejects_orphans_and_detached_cycles() {
    let orphan = tree(vec![node(Widget::Column, &[]), node(text("lost"), &[])]);
    assert!(err(&orphan).contains("not reachable"));
    let cycle = tree(vec![
        node(Widget::Column, &[]),
        node(Widget::Row, &[2]),
        node(Widget::Row, &[1]),
    ]);
    assert!(err(&cycle).contains("not reachable"));
}

#[test]
fn enforces_child_counts() {
    let leaf_with_child = tree(vec![node(Widget::Column, &[1]), node(text("x"), &[2]), node(text("y"), &[])]);
    assert!(err(&leaf_with_child).contains("children"));
    let tabs_without_page = tree(vec![node(tabs("t", 1, 0), &[])]);
    assert!(err(&tabs_without_page).contains("children"));
    let scroll_two = tree(vec![
        node(Widget::Scrollable, &[1, 2]),
        node(text("a"), &[]),
        node(text("b"), &[]),
    ]);
    assert!(err(&scroll_two).contains("children"));
}

#[test]
fn rejects_duplicate_ids() {
    let t = tree(vec![
        node(Widget::Column, &[1, 2]),
        node(button("same"), &[]),
        node(button("same"), &[]),
    ]);
    assert!(err(&t).contains("duplicate"));
}

#[test]
fn rejects_bad_tabs() {
    let out_of_range = tree(vec![node(tabs("t", 2, 2), &[1]), node(text("p"), &[])]);
    assert!(err(&out_of_range).contains("tabs"));
    let no_titles = tree(vec![node(tabs("t", 0, 0), &[1]), node(text("p"), &[])]);
    assert!(err(&no_titles).contains("tabs"));
}

#[test]
fn node_cap_is_inclusive() {
    let at_cap: Vec<u32> = (1..u32::try_from(MAX_TREE_NODES).unwrap()).collect();
    let mut nodes = vec![node(Widget::Column, &at_cap)];
    nodes.extend((1..MAX_TREE_NODES).map(|_| node(Widget::Divider, &[])));
    assert_eq!(nodes.len(), MAX_TREE_NODES);
    assert_eq!(validate_tree(&tree(nodes.clone())), Ok(()));

    nodes.push(node(Widget::Divider, &[]));
    nodes[0].children.push(u32::try_from(MAX_TREE_NODES).unwrap());
    assert!(err(&tree(nodes)).contains("max"));
}

#[test]
fn api_version_matches_wit_package() {
    let wit = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../wit/plugin.wit")).unwrap();
    let expected = format!("package saveforge:plugin@{API_VERSION};");
    assert!(wit.contains(&expected), "wit/plugin.wit must declare {expected}");
}
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `env -u CC -u CXX cargo test -p saveforge-plugin-api --test tree`
Expected: compile error, `src/lib.rs` missing (no targets) or unresolved `saveforge_plugin_api`.

- [ ] **Step 7: Write `crates/saveforge-plugin-api/src/lib.rs`**

```rust
//! Host-side bindings for the `saveforge:plugin` WIT world (spec §4.4) and
//! UI-tree validation (spec §4.5).

mod tree;

use semver::Version;

#[allow(clippy::all, clippy::pedantic)]
mod bindings {
    wasmtime::component::bindgen!({
        world: "plugin",
        path: "../../wit",
        additional_derives: [PartialEq],
    });
}

pub use bindings::exports::saveforge::plugin::editor::AuxFile;
pub use bindings::saveforge::plugin::host::{Host as LogHost, Level};
pub use bindings::saveforge::plugin::ui::{self, Host as UiHost};
pub use bindings::{Plugin as PluginWorld, PluginPre as PluginWorldPre};
pub use tree::{MAX_TREE_NODES, validate_tree};

/// Version of the WIT package this host implements (`saveforge:plugin@0.1.0`).
pub const API_VERSION: Version = Version::new(0, 1, 0);
```

- [ ] **Step 8: Write `crates/saveforge-plugin-api/src/tree.rs`**

```rust
//! Structural checks on a plugin's UI tree (spec §4.5).

use std::collections::HashSet;

use crate::ui::{UiTree, Widget};

pub const MAX_TREE_NODES: usize = 20_000;

/// Checks that `tree` is a well-formed tree the renderer can trust. Returns a
/// human-readable reason on failure.
///
/// Rules: non-empty and at most [`MAX_TREE_NODES`] nodes. Every child index is
/// in range and not the root. Every non-root node has exactly one parent and is
/// reachable from the root, which rules out cycles. Child counts fit the widget
/// (containers: any, `scrollable`/`tabs`: exactly one, leaves: none). Widget ids
/// are unique. `tabs` has at least one title and `selected` is in range.
pub fn validate_tree(tree: &UiTree) -> Result<(), String> {
    let nodes = &tree.nodes;
    if nodes.is_empty() {
        return Err("tree has no root node".to_owned());
    }
    if nodes.len() > MAX_TREE_NODES {
        return Err(format!(
            "tree has {} nodes (max {MAX_TREE_NODES})",
            nodes.len()
        ));
    }

    let mut parent: Vec<Option<usize>> = vec![None; nodes.len()];
    let mut ids = HashSet::new();
    for (index, node) in nodes.iter().enumerate() {
        check_child_count(index, &node.widget, node.children.len())?;
        if let Some(id) = widget_id(&node.widget)
            && !ids.insert(id)
        {
            return Err(format!("duplicate widget id '{id}'"));
        }
        if let Widget::Tabs(tabs) = &node.widget
            && (tabs.titles.is_empty() || tabs.selected as usize >= tabs.titles.len())
        {
            return Err(format!(
                "tabs '{}' has {} titles but selected index {}",
                tabs.id,
                tabs.titles.len(),
                tabs.selected
            ));
        }
        for &child in &node.children {
            let child = child as usize;
            if child == 0 || child >= nodes.len() {
                return Err(format!("node {index} has invalid child index {child}"));
            }
            if let Some(first) = parent[child] {
                return Err(format!("node {child} has two parents ({first} and {index})"));
            }
            parent[child] = Some(index);
        }
    }

    let mut seen = vec![false; nodes.len()];
    let mut stack = vec![0usize];
    while let Some(index) = stack.pop() {
        if std::mem::replace(&mut seen[index], true) {
            continue;
        }
        stack.extend(nodes[index].children.iter().map(|&c| c as usize));
    }
    if let Some(orphan) = seen.iter().position(|reached| !reached) {
        return Err(format!("node {orphan} is not reachable from the root"));
    }
    Ok(())
}

fn check_child_count(index: usize, widget: &Widget, count: usize) -> Result<(), String> {
    let ok = match widget {
        Widget::Column | Widget::Row | Widget::Section(_) | Widget::List(_) => true,
        Widget::Scrollable | Widget::Tabs(_) => count == 1,
        Widget::Divider | Widget::Text(_) | Widget::Field(_) | Widget::Button(_) => count == 0,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "node {index} ({}) has {count} children",
            widget_kind(widget)
        ))
    }
}

fn widget_id(widget: &Widget) -> Option<&str> {
    match widget {
        Widget::Tabs(tabs) => Some(&tabs.id),
        Widget::Field(field) => Some(&field.id),
        Widget::Button(button) => Some(&button.id),
        Widget::List(list) => Some(&list.id),
        Widget::Column
        | Widget::Row
        | Widget::Scrollable
        | Widget::Divider
        | Widget::Section(_)
        | Widget::Text(_) => None,
    }
}

fn widget_kind(widget: &Widget) -> &'static str {
    match widget {
        Widget::Column => "column",
        Widget::Row => "row",
        Widget::Scrollable => "scrollable",
        Widget::Divider => "divider",
        Widget::Section(_) => "section",
        Widget::Tabs(_) => "tabs",
        Widget::Text(_) => "text",
        Widget::Field(_) => "field",
        Widget::Button(_) => "button",
        Widget::List(_) => "list",
    }
}
```

- [ ] **Step 9: Run tests**

Run: `env -u CC -u CXX cargo test -p saveforge-plugin-api --test tree`
Expected: all 10 tests pass.

- [ ] **Step 10: Lint gate**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings. If clippy flags `as usize` on `u32` (`cast_possible_truncation` does not apply to u32→usize, but `cast_lossless` might), replace each `x as usize` with `usize::try_from(x).unwrap_or(usize::MAX)` and keep the semantics. An index of `usize::MAX` is always out of range.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock wit crates/saveforge-plugin-api
git commit -m "feat(plugin-api): WIT contract, host bindings and UI-tree validation"
```

---

### Task 2: `saveforge-sdk` core — traits, adapter, export macro, logging

**Files:**
- Create: `sdk/saveforge-sdk/Cargo.toml`, `src/lib.rs`, `src/adapter.rs`, `src/log.rs`, `src/ui.rs` (type re-exports only; builders come in Task 3)
- Modify: `Cargo.toml` (member `sdk/saveforge-sdk`, workspace dep `saveforge-sdk`)
- Test: `sdk/saveforge-sdk/tests/export.rs`

**Interfaces:**
- Consumes: `wit/plugin.wit`.
- Produces:
  - `trait Plugin: 'static { type Doc: Document; fn detect(file_name: &str, header: &[u8]) -> Option<u32>; fn open(file_name: &str, bytes: Vec<u8>, aux: Vec<AuxFile>) -> Result<Self::Doc, String>; }`
  - `trait Document: 'static { fn view(&self) -> ui::UiTree; fn update(&mut self, event: ui::Event); fn is_dirty(&self) -> bool; fn serialize(&self) -> Result<Vec<u8>, String>; fn mark_saved(&mut self); }`
  - `saveforge_sdk::export_plugin!(Type)`
  - `saveforge_sdk::{AuxFile, Level, log}`; macros `info!`, `warn!`, `error!`, `debug!`
  - `#[doc(hidden)] saveforge_sdk::{Adapter<P>, DocAdapter, bindings}`
  - `saveforge_sdk::ui::{UiTree, Node, Widget, Event, TextSpan, TextStyle, Tabs, FieldSpec, Input, TextInput, NumberInput, PickList, PickOption, ButtonSpec, ButtonStyle, ListSpec}` (all `Clone, Debug, PartialEq`)

- [ ] **Step 1: Add the crate to the workspace**

In `Cargo.toml`:

```toml
members = ["crates/saveforge-core", "crates/saveforge-plugin-api", "sdk/saveforge-sdk"]
```

and under `[workspace.dependencies]` (sorted):

```toml
saveforge-sdk = { path = "sdk/saveforge-sdk" }
```

- [ ] **Step 2: Write `sdk/saveforge-sdk/Cargo.toml`**

```toml
[package]
name = "saveforge-sdk"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
wit-bindgen.workspace = true

[lints]
workspace = true
```

- [ ] **Step 3: Write the failing test `sdk/saveforge-sdk/tests/export.rs`**

```rust
//! Native-target checks: the export macro expands, the adapter forwards to the
//! plugin, and logging works off-wasm. The full wasm round trip is covered by
//! the host tests against the fixture plugin.

use saveforge_sdk::bindings::exports::saveforge::plugin::editor::Guest;
use saveforge_sdk::ui::{Event, UiTree};
use saveforge_sdk::{Adapter, AuxFile, Document, Level, Plugin, log};

struct Echo;

struct EchoDoc;

impl Document for EchoDoc {
    fn view(&self) -> UiTree {
        UiTree { nodes: Vec::new() }
    }
    fn update(&mut self, _event: Event) {}
    fn is_dirty(&self) -> bool {
        false
    }
    fn serialize(&self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
    fn mark_saved(&mut self) {}
}

impl Plugin for Echo {
    type Doc = EchoDoc;

    fn detect(file_name: &str, header: &[u8]) -> Option<u32> {
        (file_name.ends_with(".echo") && !header.is_empty()).then_some(3)
    }

    fn open(_file_name: &str, _bytes: Vec<u8>, _aux: Vec<AuxFile>) -> Result<EchoDoc, String> {
        Ok(EchoDoc)
    }
}

saveforge_sdk::export_plugin!(Echo);

#[test]
fn adapter_forwards_detect() {
    assert_eq!(<Adapter<Echo> as Guest>::detect("a.echo".to_owned(), vec![1]), Some(3));
    assert_eq!(<Adapter<Echo> as Guest>::detect("a.echo".to_owned(), vec![]), None);
    assert_eq!(<Adapter<Echo> as Guest>::detect("a.txt".to_owned(), vec![1]), None);
}

#[test]
fn logging_works_off_wasm() {
    log(Level::Info, "plain message");
    saveforge_sdk::info!("formatted {}", 1);
    saveforge_sdk::warn!("w");
    saveforge_sdk::error!("e");
    saveforge_sdk::debug!("d");
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `env -u CC -u CXX cargo test -p saveforge-sdk --test export`
Expected: compile error, no `src/lib.rs` / unresolved `saveforge_sdk`.

- [ ] **Step 5: Write `sdk/saveforge-sdk/src/lib.rs`**

```rust
//! Guest-side SDK for `SaveForge` plugins (spec §4.6).
//!
//! Implement [`Plugin`] and [`Document`], then call [`export_plugin!`] once in
//! the plugin crate. Build with `cargo xtask plugins`.

#[doc(hidden)]
#[allow(clippy::all, clippy::pedantic)]
pub mod bindings {
    wit_bindgen::generate!({
        world: "plugin",
        path: "../../wit",
        pub_export_macro: true,
        default_bindings_module: "saveforge_sdk::bindings",
        additional_derives: [PartialEq],
    });
}

mod adapter;
mod log;
pub mod ui;

#[doc(hidden)]
pub use adapter::{Adapter, DocAdapter};
pub use bindings::exports::saveforge::plugin::editor::AuxFile;
pub use log::{Level, log};

/// Entry point of a plugin. Implemented on a unit struct; never instantiated.
pub trait Plugin: 'static {
    type Doc: Document;

    /// `Some(priority)` when this plugin can open the file. `header` holds at
    /// most the first 512 bytes.
    fn detect(file_name: &str, header: &[u8]) -> Option<u32>;

    /// Parses the save. `aux` holds the manifest's aux files that were found.
    fn open(file_name: &str, bytes: Vec<u8>, aux: Vec<AuxFile>) -> Result<Self::Doc, String>;
}

/// An open save file. The host calls `view` after every `update`.
pub trait Document: 'static {
    fn view(&self) -> ui::UiTree;
    fn update(&mut self, event: ui::Event);
    fn is_dirty(&self) -> bool;
    /// Must not change the dirty state; the host calls `mark_saved` after the
    /// bytes are on disk.
    fn serialize(&self) -> Result<Vec<u8>, String>;
    fn mark_saved(&mut self);
}

/// Exports `$plugin` (a type implementing [`Plugin`]) as the component's
/// `editor` interface. Use once per plugin crate, at module level.
#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        type __SaveforgeExport = $crate::Adapter<$plugin>;
        $crate::bindings::export!(__SaveforgeExport with_types_in $crate::bindings);
    };
}
```

- [ ] **Step 6: Write `sdk/saveforge-sdk/src/adapter.rs`**

```rust
//! Bridges the SDK traits to the generated wit-bindgen `Guest` traits.

use std::cell::RefCell;
use std::marker::PhantomData;

use crate::bindings::exports::saveforge::plugin::editor::{self as wit, AuxFile};
use crate::ui::{Event, UiTree};
use crate::{Document, Plugin};

#[doc(hidden)]
pub struct Adapter<P>(PhantomData<P>);

#[doc(hidden)]
pub struct DocAdapter(RefCell<Box<dyn Document>>);

impl<P: Plugin> wit::Guest for Adapter<P> {
    type Document = DocAdapter;

    fn detect(file_name: String, header: Vec<u8>) -> Option<u32> {
        P::detect(&file_name, &header)
    }

    fn open(
        file_name: String,
        bytes: Vec<u8>,
        aux: Vec<AuxFile>,
    ) -> Result<wit::Document, String> {
        let doc = P::open(&file_name, bytes, aux)?;
        Ok(wit::Document::new(DocAdapter(RefCell::new(Box::new(doc)))))
    }
}

impl wit::GuestDocument for DocAdapter {
    fn view(&self) -> UiTree {
        self.0.borrow().view()
    }

    fn update(&self, ev: Event) {
        self.0.borrow_mut().update(ev);
    }

    fn is_dirty(&self) -> bool {
        self.0.borrow().is_dirty()
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        self.0.borrow().serialize()
    }

    fn mark_saved(&self) {
        self.0.borrow_mut().mark_saved();
    }
}
```

- [ ] **Step 7: Write `sdk/saveforge-sdk/src/log.rs`**

```rust
//! Logging to the host. On wasm this calls the `host.log` import; on native
//! targets (unit tests) it prints to stderr.

pub use crate::bindings::saveforge::plugin::host::Level;

pub fn log(level: Level, message: &str) {
    #[cfg(target_arch = "wasm32")]
    crate::bindings::saveforge::plugin::host::log(level, message);
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("[{level:?}] {message}");
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { $crate::log($crate::Level::Debug, &::std::format!($($arg)*)) };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::log($crate::Level::Info, &::std::format!($($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { $crate::log($crate::Level::Warn, &::std::format!($($arg)*)) };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { $crate::log($crate::Level::Error, &::std::format!($($arg)*)) };
}
```

- [ ] **Step 8: Write `sdk/saveforge-sdk/src/ui.rs` (types only for now)**

```rust
//! UI-tree types sent to the host (spec §4.4). Builders are added in Task 3.

pub use crate::bindings::saveforge::plugin::ui::{
    Button as ButtonSpec, ButtonStyle, Event, Field as FieldSpec, Input, ListSpec, Node,
    NumberInput, PickList, PickOption, Tabs, TextInput, TextSpan, TextStyle, UiTree, Widget,
};
```

- [ ] **Step 9: Run tests (native and wasm build)**

Run: `env -u CC -u CXX cargo test -p saveforge-sdk --test export`
Expected: 2 tests pass.

Run: `env -u CC -u CXX cargo build -p saveforge-sdk --target wasm32-wasip2`
Expected: builds (this exercises the `cfg(target_arch = "wasm32")` branch of `log`).

- [ ] **Step 10: Lint gate (native and wasm)**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings && env -u CC -u CXX cargo clippy -p saveforge-sdk --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock sdk
git commit -m "feat(sdk): plugin traits, export macro and host logging"
```

---

### Task 3: SDK UI builders

**Files:**
- Modify: `sdk/saveforge-sdk/src/ui.rs`
- Test: `sdk/saveforge-sdk/tests/ui.rs`

**Interfaces:**
- Consumes: the `ui` re-exports from Task 2.
- Produces, all in `saveforge_sdk::ui`:
  - `El` (`Clone, Debug, PartialEq`) with `El::into_tree(self) -> UiTree`. Pre-order, root at index 0.
  - containers: `column(Vec<El>)`, `row(Vec<El>)`, `scrollable(impl Into<El>)`, `divider()`, `section(impl Into<String>, Vec<El>)`, `tabs(id, titles: Vec<String>, selected: u32, page: impl Into<El>)`
  - text: `text(s)`, `muted(s)`, `heading(s)`, `error_text(s)`
  - fields: `text_input(id, label, value) -> FieldEl`, `number(id, label, f64) -> FieldEl`, `checkbox(id, label, bool) -> FieldEl`, `pick(id, label, Vec<PickOption>, Option<String>) -> FieldEl`, `option(value, display) -> PickOption`
  - `FieldEl` methods: `help`, `error`, `read_only(bool)`, `placeholder`, `max_len(u32)`, `range(f64, f64)`, `min(f64)`, `max(f64)`, `decimals(u8)`, `step(f64)`. A method that does not fit the input kind is a no-op.
  - `button(id, label) -> ButtonEl` with `.style(ButtonStyle)` and `.enabled(bool)`
  - `list(id, rows: Vec<El>) -> ListEl` with `.removable(bool)` and `.add_options(Vec<PickOption>, placeholder: Option<String>)`
  - `From<FieldEl | ButtonEl | ListEl> for El`
  - crate-root macros `saveforge_sdk::col![…]` and `saveforge_sdk::row![…]`, which accept anything `Into<El>`

- [ ] **Step 1: Write the failing tests `sdk/saveforge-sdk/tests/ui.rs`**

```rust
use saveforge_sdk::ui::{
    ButtonSpec, ButtonStyle, El, FieldSpec, Input, ListSpec, Node, NumberInput, PickList,
    TextInput, TextSpan, TextStyle, UiTree, Widget, button, checkbox, column, divider, heading,
    list, muted, number, option, pick, scrollable, section, tabs, text, text_input,
};
use saveforge_sdk::{col, row};

fn span(text: &str, style: TextStyle) -> Widget {
    Widget::Text(TextSpan {
        text: text.to_owned(),
        style,
    })
}

#[test]
fn flattens_pre_order_with_root_first() {
    let tree = col![
        heading("Title"),
        section("Stats", vec![text("a").into(), muted("b").into()]),
        divider(),
    ]
    .into_tree();
    assert_eq!(
        tree,
        UiTree {
            nodes: vec![
                Node { widget: Widget::Column, children: vec![1, 2, 5] },
                Node { widget: span("Title", TextStyle::Heading), children: vec![] },
                Node { widget: Widget::Section("Stats".to_owned()), children: vec![3, 4] },
                Node { widget: span("a", TextStyle::Body), children: vec![] },
                Node { widget: span("b", TextStyle::Muted), children: vec![] },
                Node { widget: Widget::Divider, children: vec![] },
            ]
        }
    );
}

#[test]
fn number_field_builder() {
    let el: El = number("hp", "Health", 42.5)
        .range(0.0, 100.0)
        .decimals(1)
        .step(0.5)
        .help("0-100")
        .into();
    let tree = el.into_tree();
    assert_eq!(
        tree.nodes[0].widget,
        Widget::Field(FieldSpec {
            id: "hp".to_owned(),
            label: "Health".to_owned(),
            help: Some("0-100".to_owned()),
            error: None,
            read_only: false,
            input: Input::Number(NumberInput {
                value: 42.5,
                min: Some(0.0),
                max: Some(100.0),
                decimals: 1,
                step: Some(0.5),
            }),
        })
    );
}

#[test]
fn text_field_builder_and_inapplicable_methods_are_noops() {
    let el: El = text_input("name", "Name", "Bob")
        .placeholder("type")
        .max_len(64)
        .range(1.0, 2.0)
        .read_only(true)
        .error("bad")
        .into();
    let Widget::Field(field) = &el.into_tree().nodes[0].widget else {
        panic!("expected field");
    };
    assert!(field.read_only);
    assert_eq!(field.error.as_deref(), Some("bad"));
    assert_eq!(
        field.input,
        Input::Text(TextInput {
            value: "Bob".to_owned(),
            placeholder: Some("type".to_owned()),
            max_len: Some(64),
        })
    );
}

#[test]
fn checkbox_pick_and_button_builders() {
    let tree = row![
        checkbox("god", "God mode", true),
        pick("char", "Character", vec![option("0", "Ann"), option("1", "Bo")], Some("1".to_owned()))
            .placeholder("choose"),
        button("save", "Save").style(ButtonStyle::Danger).enabled(false),
    ]
    .into_tree();
    assert_eq!(tree.nodes[0].widget, Widget::Row);
    let Widget::Field(cb) = &tree.nodes[1].widget else { panic!() };
    assert_eq!(cb.input, Input::Checkbox(true));
    let Widget::Field(pk) = &tree.nodes[2].widget else { panic!() };
    let Input::Pick(PickList { options, selected, placeholder }) = &pk.input else { panic!() };
    assert_eq!(options.len(), 2);
    assert_eq!(selected.as_deref(), Some("1"));
    assert_eq!(placeholder.as_deref(), Some("choose"));
    assert_eq!(
        tree.nodes[3].widget,
        Widget::Button(ButtonSpec {
            id: "save".to_owned(),
            label: "Save".to_owned(),
            style: ButtonStyle::Danger,
            enabled: false,
        })
    );
}

#[test]
fn tabs_and_scrollable_wrap_exactly_one_child() {
    let tree = tabs(
        "main",
        vec!["A".to_owned(), "B".to_owned()],
        1,
        scrollable(column(vec![text("page").into()])),
    )
    .into_tree();
    assert_eq!(tree.nodes[0].children, vec![1]);
    assert_eq!(tree.nodes[1].widget, Widget::Scrollable);
    assert_eq!(tree.nodes[1].children, vec![2]);
    assert_eq!(tree.nodes.len(), 4);
}

#[test]
fn list_builder() {
    let tree: UiTree = El::from(
        list("inv", vec![text("Iron").into(), text("Ice").into()])
            .removable(true)
            .add_options(vec![option("iron", "Iron")], Some("Add item".to_owned())),
    )
    .into_tree();
    assert_eq!(tree.nodes[0].children, vec![1, 2]);
    assert_eq!(
        tree.nodes[0].widget,
        Widget::List(ListSpec {
            id: "inv".to_owned(),
            removable: true,
            add_options: Some(PickList {
                options: vec![option("iron", "Iron")],
                selected: None,
                placeholder: Some("Add item".to_owned()),
            }),
        })
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `env -u CC -u CXX cargo test -p saveforge-sdk --test ui`
Expected: compile errors, unresolved imports `El`, `button`, `col`, …

- [ ] **Step 3: Replace `sdk/saveforge-sdk/src/ui.rs`**

```rust
//! UI-tree types and builders (spec §4.4, §4.6).
//!
//! Build a nested [`El`] with the functions here (or the crate-root `col!` /
//! `row!` macros) and call [`El::into_tree`] in `Document::view`.

pub use crate::bindings::saveforge::plugin::ui::{
    Button as ButtonSpec, ButtonStyle, Event, Field as FieldSpec, Input, ListSpec, Node,
    NumberInput, PickList, PickOption, Tabs, TextInput, TextSpan, TextStyle, UiTree, Widget,
};

/// A widget with its children, before flattening into the host's arena form.
#[derive(Debug, Clone, PartialEq)]
pub struct El {
    widget: Widget,
    children: Vec<El>,
}

impl El {
    fn leaf(widget: Widget) -> Self {
        Self {
            widget,
            children: Vec::new(),
        }
    }

    /// Flattens to the arena form: pre-order, root at index 0.
    pub fn into_tree(self) -> UiTree {
        let mut nodes = Vec::new();
        push(self, &mut nodes);
        UiTree { nodes }
    }
}

fn push(el: El, nodes: &mut Vec<Node>) -> u32 {
    let index = nodes.len();
    nodes.push(Node {
        widget: el.widget,
        children: Vec::new(),
    });
    let children = el.children.into_iter().map(|c| push(c, nodes)).collect();
    nodes[index].children = children;
    u32::try_from(index).expect("UI tree exceeds u32::MAX nodes")
}

pub fn column(children: Vec<El>) -> El {
    El {
        widget: Widget::Column,
        children,
    }
}

pub fn row(children: Vec<El>) -> El {
    El {
        widget: Widget::Row,
        children,
    }
}

pub fn scrollable(child: impl Into<El>) -> El {
    El {
        widget: Widget::Scrollable,
        children: vec![child.into()],
    }
}

pub fn divider() -> El {
    El::leaf(Widget::Divider)
}

pub fn section(title: impl Into<String>, children: Vec<El>) -> El {
    El {
        widget: Widget::Section(title.into()),
        children,
    }
}

/// Tab strip; `page` is the content of the `selected` tab only.
pub fn tabs(id: impl Into<String>, titles: Vec<String>, selected: u32, page: impl Into<El>) -> El {
    El {
        widget: Widget::Tabs(Tabs {
            id: id.into(),
            titles,
            selected,
        }),
        children: vec![page.into()],
    }
}

fn styled(text: impl Into<String>, style: TextStyle) -> El {
    El::leaf(Widget::Text(TextSpan {
        text: text.into(),
        style,
    }))
}

pub fn text(text: impl Into<String>) -> El {
    styled(text, TextStyle::Body)
}

pub fn muted(text: impl Into<String>) -> El {
    styled(text, TextStyle::Muted)
}

pub fn heading(text: impl Into<String>) -> El {
    styled(text, TextStyle::Heading)
}

pub fn error_text(text: impl Into<String>) -> El {
    styled(text, TextStyle::Error)
}

pub fn option(value: impl Into<String>, display: impl Into<String>) -> PickOption {
    PickOption {
        value: value.into(),
        display: display.into(),
    }
}

/// Labelled input under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEl(FieldSpec);

fn field(id: impl Into<String>, label: impl Into<String>, input: Input) -> FieldEl {
    FieldEl(FieldSpec {
        id: id.into(),
        label: label.into(),
        help: None,
        error: None,
        read_only: false,
        input,
    })
}

pub fn text_input(id: impl Into<String>, label: impl Into<String>, value: impl Into<String>) -> FieldEl {
    field(
        id,
        label,
        Input::Text(TextInput {
            value: value.into(),
            placeholder: None,
            max_len: None,
        }),
    )
}

pub fn number(id: impl Into<String>, label: impl Into<String>, value: f64) -> FieldEl {
    field(
        id,
        label,
        Input::Number(NumberInput {
            value,
            min: None,
            max: None,
            decimals: 0,
            step: None,
        }),
    )
}

pub fn checkbox(id: impl Into<String>, label: impl Into<String>, checked: bool) -> FieldEl {
    field(id, label, Input::Checkbox(checked))
}

pub fn pick(
    id: impl Into<String>,
    label: impl Into<String>,
    options: Vec<PickOption>,
    selected: Option<String>,
) -> FieldEl {
    field(
        id,
        label,
        Input::Pick(PickList {
            options,
            selected,
            placeholder: None,
        }),
    )
}

impl FieldEl {
    #[must_use]
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.0.help = Some(help.into());
        self
    }

    #[must_use]
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.0.error = Some(error.into());
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.0.read_only = read_only;
        self
    }

    /// Text and pick inputs only.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        match &mut self.0.input {
            Input::Text(t) => t.placeholder = Some(placeholder.into()),
            Input::Pick(p) => p.placeholder = Some(placeholder.into()),
            Input::Number(_) | Input::Checkbox(_) => {}
        }
        self
    }

    /// Text inputs only.
    #[must_use]
    pub fn max_len(mut self, max_len: u32) -> Self {
        if let Input::Text(t) = &mut self.0.input {
            t.max_len = Some(max_len);
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn range(self, min: f64, max: f64) -> Self {
        self.min(min).max(max)
    }

    /// Number inputs only.
    #[must_use]
    pub fn min(mut self, min: f64) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.min = Some(min);
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn max(mut self, max: f64) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.max = Some(max);
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn decimals(mut self, decimals: u8) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.decimals = decimals;
        }
        self
    }

    /// Number inputs only.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        if let Input::Number(n) = &mut self.0.input {
            n.step = Some(step);
        }
        self
    }
}

impl From<FieldEl> for El {
    fn from(field: FieldEl) -> Self {
        El::leaf(Widget::Field(field.0))
    }
}

/// Button under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonEl(ButtonSpec);

pub fn button(id: impl Into<String>, label: impl Into<String>) -> ButtonEl {
    ButtonEl(ButtonSpec {
        id: id.into(),
        label: label.into(),
        style: ButtonStyle::Primary,
        enabled: true,
    })
}

impl ButtonEl {
    #[must_use]
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.0.style = style;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.enabled = enabled;
        self
    }
}

impl From<ButtonEl> for El {
    fn from(button: ButtonEl) -> Self {
        El::leaf(Widget::Button(button.0))
    }
}

/// Repeated rows under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct ListEl {
    spec: ListSpec,
    rows: Vec<El>,
}

pub fn list(id: impl Into<String>, rows: Vec<El>) -> ListEl {
    ListEl {
        spec: ListSpec {
            id: id.into(),
            removable: false,
            add_options: None,
        },
        rows,
    }
}

impl ListEl {
    #[must_use]
    pub fn removable(mut self, removable: bool) -> Self {
        self.spec.removable = removable;
        self
    }

    #[must_use]
    pub fn add_options(mut self, options: Vec<PickOption>, placeholder: Option<String>) -> Self {
        self.spec.add_options = Some(PickList {
            options,
            selected: None,
            placeholder,
        });
        self
    }
}

impl From<ListEl> for El {
    fn from(list: ListEl) -> Self {
        El {
            widget: Widget::List(list.spec),
            children: list.rows,
        }
    }
}

/// `col![a, b, …]`: a column of anything convertible into [`El`].
#[macro_export]
macro_rules! col {
    ($($child:expr),* $(,)?) => {
        $crate::ui::column(::std::vec![$($crate::ui::El::from($child)),*])
    };
}

/// `row![a, b, …]`: a row of anything convertible into [`El`].
#[macro_export]
macro_rules! row {
    ($($child:expr),* $(,)?) => {
        $crate::ui::row(::std::vec![$($crate::ui::El::from($child)),*])
    };
}
```

- [ ] **Step 4: Run tests**

Run: `env -u CC -u CXX cargo test -p saveforge-sdk`
Expected: `export` 2 pass, `ui` 6 pass.

- [ ] **Step 5: Lint gate (native and wasm)**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings && env -u CC -u CXX cargo clippy -p saveforge-sdk --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add sdk
git commit -m "feat(sdk): UI tree builders"
```

---

### Task 4: Fixture plugin, `cargo xtask plugins`, CI

**Files:**
- Create: `fixtures/test-plugin/Cargo.toml`, `fixtures/test-plugin/manifest.json`, `fixtures/test-plugin/src/lib.rs`
- Create: `xtask/Cargo.toml`, `xtask/src/lib.rs`, `xtask/src/main.rs`, `.cargo/config.toml`
- Modify: `Cargo.toml` (members), `.github/workflows/ci.yml`
- Test: `xtask/tests/xtask.rs`

**Interfaces:**
- Consumes: `saveforge_sdk::{Plugin, Document, AuxFile, export_plugin!, info!, col!}`, `saveforge_sdk::ui::{Event, Node, UiTree, Widget, text, text_input}`, `saveforge_core::manifest::PluginManifest::load`.
- Produces:
  - **Fixture behaviour**, which the host tests rely on:
    - `detect`: `"spin-detect.fixture"` spins forever. A header longer than 512 bytes returns `None`. A header starting with `no-match` returns `None`. Anything else returns `Some(5)`.
    - `open`: bytes `panic` panic, `hog` allocates until out of memory, `reject` returns `Err("rejected by fixture")`. Otherwise the doc text is the UTF-8 content plus ` +{id}={aux text}` for each aux file.
    - `view`: text `spin-view` spins, `bad-tree` returns an invalid tree. Otherwise `col![text(t), text_input("text","Text",t)]`.
    - `update`: `TextChanged(("text", v))` sets the text and marks the doc dirty. `Pressed("spin")` spins. `Pressed("panic")` panics.
    - `serialize`: text `unserializable` returns `Err("cannot serialize")`, otherwise the text bytes.
  - **xtask library:**
    - `WASM_TARGET`
    - `PluginKind { Plugin, Fixture }`
    - `PluginCrate { kind, package: String, dir: PathBuf }`
    - `find_plugin_crates(workspace: &Path) -> Vec<PluginCrate>`
    - `wasm_artifact(target_dir, package) -> PathBuf`
    - `dev_plugin_dir(target_dir, &PluginManifest) -> PathBuf`
    - `fixture_wasm(target_dir, package) -> PathBuf`
    - `install(target_dir, &PluginCrate) -> anyhow::Result<PathBuf>`
  - **Outputs:**
    - plugins go to `target/dev-plugins/{id}/{version}/` (manifest + component + icon)
    - fixtures go to `target/fixtures/{package}.wasm`; the test fixture lands at `target/fixtures/test-plugin.wasm`

- [ ] **Step 1: Add members**

```toml
members = [
    "crates/saveforge-core",
    "crates/saveforge-plugin-api",
    "fixtures/test-plugin",
    "sdk/saveforge-sdk",
    "xtask",
]
```

- [ ] **Step 2: Write `fixtures/test-plugin/Cargo.toml`**

```toml
[package]
name = "test-plugin"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
saveforge-sdk.workspace = true

[lints]
workspace = true
```

- [ ] **Step 3: Write `fixtures/test-plugin/manifest.json`**

```json
{
  "id": "test-plugin",
  "name": "Test Plugin",
  "version": "0.1.0",
  "author": "SaveForge",
  "description": "Fixture used by saveforge-host tests.",
  "apiVersion": "0.1",
  "minHostVersion": "0.1.0",
  "maxHostVersion": null,
  "component": "plugin.wasm",
  "fileExtensions": [".fixture"]
}
```

- [ ] **Step 4: Write `fixtures/test-plugin/src/lib.rs`**

```rust
//! Fixture plugin for `saveforge-host` tests. Every fault the host must survive
//! can be provoked through its inputs (see each match arm).

use saveforge_sdk::ui::{Event, Node, UiTree, Widget, text, text_input};
use saveforge_sdk::{AuxFile, Document, Plugin, col, export_plugin, info};

pub struct Fixture;

pub struct FixtureDoc {
    text: String,
    dirty: bool,
}

impl Plugin for Fixture {
    type Doc = FixtureDoc;

    fn detect(file_name: &str, header: &[u8]) -> Option<u32> {
        if file_name == "spin-detect.fixture" {
            spin();
        }
        // The host must truncate headers to 512 bytes; refusing longer ones
        // lets a test prove that it did.
        if header.len() > 512 || header.starts_with(b"no-match") {
            return None;
        }
        Some(5)
    }

    fn open(file_name: &str, bytes: Vec<u8>, aux: Vec<AuxFile>) -> Result<FixtureDoc, String> {
        info!("opening {file_name}");
        match bytes.as_slice() {
            b"panic" => panic!("fixture panic in open"),
            b"hog" => hog(),
            b"reject" => return Err("rejected by fixture".to_owned()),
            _ => {}
        }
        let mut text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        for file in aux {
            text.push_str(&format!(
                " +{}={}",
                file.id,
                String::from_utf8_lossy(&file.bytes)
            ));
        }
        Ok(FixtureDoc { text, dirty: false })
    }
}

impl Document for FixtureDoc {
    fn view(&self) -> UiTree {
        match self.text.as_str() {
            "spin-view" => spin(),
            "bad-tree" => UiTree {
                nodes: vec![Node {
                    widget: Widget::Column,
                    children: vec![7],
                }],
            },
            _ => col![text(self.text.clone()), text_input("text", "Text", self.text.clone())]
                .into_tree(),
        }
    }

    fn update(&mut self, event: Event) {
        match event {
            Event::TextChanged((id, value)) if id == "text" => {
                self.text = value;
                self.dirty = true;
            }
            Event::Pressed(id) if id == "spin" => spin(),
            Event::Pressed(id) if id == "panic" => panic!("fixture panic in update"),
            _ => {}
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        if self.text == "unserializable" {
            Err("cannot serialize".to_owned())
        } else {
            Ok(self.text.clone().into_bytes())
        }
    }

    fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

fn spin() -> ! {
    loop {
        std::hint::spin_loop();
    }
}

fn hog() -> ! {
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    loop {
        blocks.push(vec![1u8; 16 * 1024 * 1024]);
        std::hint::black_box(&blocks);
    }
}

export_plugin!(Fixture);
```

- [ ] **Step 5: Write `xtask/Cargo.toml` and `.cargo/config.toml`**

```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
anyhow.workspace = true
saveforge-core.workspace = true

[dev-dependencies]
tempfile.workspace = true

[lints]
workspace = true
```

```toml
# .cargo/config.toml
[alias]
xtask = "run --package xtask --"
```

- [ ] **Step 6: Write the failing tests `xtask/tests/xtask.rs`**

```rust
use std::fs;
use std::path::Path;

use saveforge_core::manifest::PluginManifest;
use xtask::{
    PluginCrate, PluginKind, dev_plugin_dir, find_plugin_crates, fixture_wasm, install,
    wasm_artifact,
};

const MANIFEST: &str = r#"{
  "id": "demo", "name": "Demo", "version": "1.2.3", "author": "a",
  "apiVersion": "0.1", "minHostVersion": "0.1.0",
  "component": "plugin.wasm", "icon": "icon.png", "fileExtensions": [".sav"]
}"#;

fn make_crate(root: &Path, sub: &str, name: &str, manifest: bool) {
    let dir = root.join(sub).join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
    if manifest {
        fs::write(dir.join("manifest.json"), MANIFEST).unwrap();
        fs::write(dir.join("icon.png"), b"png").unwrap();
    }
}

#[test]
fn finds_plugin_and_fixture_crates_with_manifests() {
    let root = tempfile::tempdir().unwrap();
    make_crate(root.path(), "plugins", "demo", true);
    make_crate(root.path(), "plugins", "no-manifest", false);
    make_crate(root.path(), "fixtures", "test-plugin", true);
    let found = find_plugin_crates(root.path());
    let summary: Vec<_> = found.iter().map(|c| (c.kind, c.package.as_str())).collect();
    assert_eq!(
        summary,
        [(PluginKind::Plugin, "demo"), (PluginKind::Fixture, "test-plugin")]
    );
}

#[test]
fn output_paths() {
    let target = Path::new("/t");
    assert_eq!(
        wasm_artifact(target, "planet-crafter"),
        Path::new("/t/wasm32-wasip2/release/planet_crafter.wasm")
    );
    assert_eq!(fixture_wasm(target, "test-plugin"), Path::new("/t/fixtures/test-plugin.wasm"));
    let manifest = PluginManifest::parse(MANIFEST).unwrap();
    assert_eq!(dev_plugin_dir(target, &manifest), Path::new("/t/dev-plugins/demo/1.2.3"));
}

#[test]
fn installs_plugin_with_manifest_component_and_icon() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    make_crate(root.path(), "plugins", "demo", true);
    let wasm = wasm_artifact(&target, "demo");
    fs::create_dir_all(wasm.parent().unwrap()).unwrap();
    fs::write(&wasm, b"\0asm").unwrap();
    let krate = PluginCrate {
        kind: PluginKind::Plugin,
        package: "demo".to_owned(),
        dir: root.path().join("plugins/demo"),
    };

    let dest = install(&target, &krate).unwrap();
    assert_eq!(dest, target.join("dev-plugins/demo/1.2.3"));
    assert_eq!(fs::read(dest.join("plugin.wasm")).unwrap(), b"\0asm");
    assert_eq!(fs::read(dest.join("icon.png")).unwrap(), b"png");
    assert!(dest.join("manifest.json").is_file());

    // Re-install replaces stale files.
    fs::write(dest.join("stale.txt"), b"x").unwrap();
    install(&target, &krate).unwrap();
    assert!(!dest.join("stale.txt").exists());
}

#[test]
fn installs_fixture_wasm_only() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    make_crate(root.path(), "fixtures", "test-plugin", true);
    let wasm = wasm_artifact(&target, "test-plugin");
    fs::create_dir_all(wasm.parent().unwrap()).unwrap();
    fs::write(&wasm, b"\0asm").unwrap();
    let krate = PluginCrate {
        kind: PluginKind::Fixture,
        package: "test-plugin".to_owned(),
        dir: root.path().join("fixtures/test-plugin"),
    };
    let dest = install(&target, &krate).unwrap();
    assert_eq!(dest, target.join("fixtures/test-plugin.wasm"));
    assert_eq!(fs::read(dest).unwrap(), b"\0asm");
}

#[test]
fn install_fails_when_build_output_missing() {
    let root = tempfile::tempdir().unwrap();
    make_crate(root.path(), "plugins", "demo", true);
    let krate = PluginCrate {
        kind: PluginKind::Plugin,
        package: "demo".to_owned(),
        dir: root.path().join("plugins/demo"),
    };
    let err = install(&root.path().join("target"), &krate).unwrap_err();
    assert!(err.to_string().contains("build output missing"), "{err}");
}
```

- [ ] **Step 7: Run tests to verify they fail**

Run: `env -u CC -u CXX cargo test -p xtask --test xtask`
Expected: compile error, unresolved crate `xtask` items.

- [ ] **Step 8: Write `xtask/src/lib.rs`**

```rust
//! Helpers behind `cargo xtask` (spec §10).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, ensure};
use saveforge_core::manifest::PluginManifest;

pub const WASM_TARGET: &str = "wasm32-wasip2";

/// Where a plugin crate lives: `plugins/` ships to users, `fixtures/` feeds tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Plugin,
    Fixture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCrate {
    pub kind: PluginKind,
    /// Cargo package name; by convention equal to the directory name.
    pub package: String,
    pub dir: PathBuf,
}

/// Every crate under `plugins/` and `fixtures/` with both `Cargo.toml` and
/// `manifest.json`, sorted by package name.
pub fn find_plugin_crates(workspace: &Path) -> Vec<PluginCrate> {
    let mut found = Vec::new();
    for (sub, kind) in [("plugins", PluginKind::Plugin), ("fixtures", PluginKind::Fixture)] {
        let Ok(entries) = fs::read_dir(workspace.join(sub)) else {
            continue;
        };
        for entry in entries.flatten() {
            let dir = entry.path();
            if dir.join("Cargo.toml").is_file() && dir.join("manifest.json").is_file() {
                found.push(PluginCrate {
                    kind,
                    package: entry.file_name().to_string_lossy().into_owned(),
                    dir,
                });
            }
        }
    }
    found.sort_by(|a, b| a.package.cmp(&b.package));
    found
}

/// The component cargo writes for `package` on the wasm target.
pub fn wasm_artifact(target_dir: &Path, package: &str) -> PathBuf {
    target_dir
        .join(WASM_TARGET)
        .join("release")
        .join(format!("{}.wasm", package.replace('-', "_")))
}

/// `target/dev-plugins/{id}/{version}`, the debug app's plugin directory layout.
pub fn dev_plugin_dir(target_dir: &Path, manifest: &PluginManifest) -> PathBuf {
    target_dir
        .join("dev-plugins")
        .join(&manifest.id)
        .join(manifest.version.to_string())
}

/// `target/fixtures/{package}.wasm`, read by host tests.
pub fn fixture_wasm(target_dir: &Path, package: &str) -> PathBuf {
    target_dir.join("fixtures").join(format!("{package}.wasm"))
}

/// Copies a built crate into place and returns the destination. A plugin
/// directory is replaced wholesale so stale files never linger.
pub fn install(target_dir: &Path, krate: &PluginCrate) -> anyhow::Result<PathBuf> {
    let manifest = PluginManifest::load(&krate.dir)
        .with_context(|| format!("invalid manifest in {}", krate.dir.display()))?;
    let wasm = wasm_artifact(target_dir, &krate.package);
    ensure!(wasm.is_file(), "build output missing: {}", wasm.display());

    match krate.kind {
        PluginKind::Fixture => {
            let dest = fixture_wasm(target_dir, &krate.package);
            fs::create_dir_all(target_dir.join("fixtures"))?;
            fs::copy(&wasm, &dest)?;
            Ok(dest)
        }
        PluginKind::Plugin => {
            let dest = dev_plugin_dir(target_dir, &manifest);
            if dest.exists() {
                fs::remove_dir_all(&dest)?;
            }
            fs::create_dir_all(&dest)?;
            fs::copy(krate.dir.join("manifest.json"), dest.join("manifest.json"))?;
            copy_into(&wasm, &dest.join(&manifest.component))?;
            if let Some(icon) = &manifest.icon {
                copy_into(&krate.dir.join(icon), &dest.join(icon))?;
            }
            Ok(dest)
        }
    }
}

fn copy_into(from: &Path, to: &Path) -> anyhow::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to).with_context(|| format!("copy {} -> {}", from.display(), to.display()))?;
    Ok(())
}
```

- [ ] **Step 9: Write `xtask/src/main.rs`**

```rust
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use saveforge_core::manifest::PluginManifest;
use xtask::{WASM_TARGET, find_plugin_crates, install};

const USAGE: &str = "usage: cargo xtask plugins [-p <package-or-plugin-id>]";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("plugins") => plugins(&args[1..]),
        _ => bail!(USAGE),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives inside the workspace")
        .to_path_buf()
}

fn plugins(args: &[String]) -> Result<()> {
    let filter = match args {
        [] => None,
        [flag, name] if flag == "-p" => Some(name.as_str()),
        _ => bail!(USAGE),
    };
    let root = workspace_root();
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| root.join("target"), PathBuf::from);

    let crates: Vec<_> = find_plugin_crates(&root)
        .into_iter()
        .filter(|c| {
            filter.is_none_or(|f| {
                c.package == f || PluginManifest::load(&c.dir).is_ok_and(|m| m.id == f)
            })
        })
        .collect();
    if crates.is_empty() {
        bail!("no plugin crate matched");
    }

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&root)
        .args(["build", "--release", "--target", WASM_TARGET]);
    for c in &crates {
        cmd.args(["-p", &c.package]);
    }
    let status = cmd.status().context("failed to run cargo")?;
    if !status.success() {
        bail!("cargo build for {WASM_TARGET} failed");
    }

    for c in &crates {
        let dest = install(&target_dir, c)?;
        println!("{} -> {}", c.package, dest.display());
    }
    Ok(())
}
```

- [ ] **Step 10: Run xtask tests, then the real build**

Run: `env -u CC -u CXX cargo test -p xtask --test xtask`
Expected: 5 tests pass.

Run: `env -u CC -u CXX cargo xtask plugins`
Expected: ends with `test-plugin -> …/target/fixtures/test-plugin.wasm`. Check with `ls -la target/fixtures/test-plugin.wasm` (roughly 100 KB).

- [ ] **Step 11: Update `.github/workflows/ci.yml`**

In the `lint` job, replace the toolchain step with one that also installs clippy and the wasm target, then add a wasm clippy step after `cargo fmt --all --check`:

```yaml
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
          targets: wasm32-wasip2
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy -p saveforge-sdk -p test-plugin --target wasm32-wasip2 -- -D warnings
```

In the `test` job, add the wasm target and build plugins before testing:

```yaml
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
          targets: wasm32-wasip2
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo xtask plugins
      - run: cargo test --workspace
```

- [ ] **Step 12: Lint gate (native and wasm)**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings && env -u CC -u CXX cargo clippy -p saveforge-sdk -p test-plugin --target wasm32-wasip2 -- -D warnings`
Expected: no warnings. The fixture's `format!` inside `push_str` may trip `clippy::format_push_string` (pedantic). If it does, use `write!(text, " +{}={}", …)` with `use std::fmt::Write as _;` and `.expect("writing to a String cannot fail")`.

- [ ] **Step 13: Commit**

```bash
git add Cargo.toml Cargo.lock .cargo .github fixtures xtask
git commit -m "feat: fault-provoking fixture plugin and cargo xtask plugins"
```

---

### Task 5: Host engine — limits, compile cache, loading

**Files:**
- Create: `crates/saveforge-host/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/engine.rs`, `src/stdio.rs`
- Create: `crates/saveforge-host/tests/common/mod.rs`
- Modify: `crates/saveforge-core/src/paths.rs` (add `wasm_cache_dir`), `crates/saveforge-core/tests/settings.rs` (assert it), `Cargo.toml` (member)
- Test: `crates/saveforge-host/tests/engine.rs`

**Interfaces:**
- Consumes: `saveforge_plugin_api::{API_VERSION, PluginWorld, PluginWorldPre, LogHost, UiHost, Level}`, `saveforge_core::{CoreError, manifest::PluginManifest}`.
- Produces:
  - `HostError` enum (`Debug`, `thiserror`) with variants:
    - `Core(CoreError)`
    - `Compile { path: PathBuf, reason: String }`
    - `Instantiate(String)`
    - `Fault(PluginFault)`
    - `Poisoned`
    - `Rejected(String)`
    - `SerializeFailed(String)`
    - `MissingAux(String)`
    - `Engine(String)`
  - `PluginFault { Timeout, Trap(String), InvalidUi(String) }` (`Clone, Debug, PartialEq, Eq`)
  - `HOST_VERSION: LazyLock<Version>`, `TICK: Duration = 10ms`
  - `HostLimits { ui_call: Duration, io_call: Duration, memory_bytes: usize }` (`Default` per Global Constraints)
  - `PluginEngine::new(HostLimits, cache_dir: Option<&Path>) -> Result<PluginEngine, HostError>`, `PluginEngine::limits() -> HostLimits`, `PluginEngine::load(&self, dir: &Path) -> Result<LoadedPlugin, HostError>`
  - `LoadedPlugin { pub manifest: PluginManifest, pub dir: PathBuf, /* private pre */ }`
  - crate-internal: `PluginEngine::{new_store(&str) -> Store<State>, instantiate(&LoadedPlugin) -> Result<(Store<State>, PluginWorld), HostError>, ui_ticks() -> u64, io_ticks() -> u64}`, `error::classify(&wasmtime::Error) -> PluginFault`
  - core: `AppPaths::wasm_cache_dir() -> PathBuf` = `<data_dir>/cache/wasmtime`

- [ ] **Step 1: Add the member and write `crates/saveforge-host/Cargo.toml`**

Workspace `members` gains `"crates/saveforge-host"` (keep sorted).

```toml
[package]
name = "saveforge-host"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
saveforge-core.workspace = true
saveforge-plugin-api.workspace = true
semver.workspace = true
thiserror.workspace = true
tokio.workspace = true
tracing.workspace = true
wasmtime.workspace = true
wasmtime-wasi.workspace = true

[dev-dependencies]
serde_json.workspace = true
tempfile.workspace = true

[lints]
workspace = true
```

- [ ] **Step 2: Add `wasm_cache_dir` to core (test first)**

Append to `crates/saveforge-core/tests/settings.rs`, inside `app_paths_layout` after the `crash_log` assertion:

```rust
    assert_eq!(p.wasm_cache_dir(), PathBuf::from("/data/cache/wasmtime"));
```

Run: `env -u CC -u CXX cargo test -p saveforge-core --test settings`
Expected: compile error, no method `wasm_cache_dir`.

Add to `impl AppPaths` in `crates/saveforge-core/src/paths.rs`:

```rust
    /// Wasmtime's compiled-component cache (keyed by wasmtime version and module hash).
    pub fn wasm_cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache").join("wasmtime")
    }
```

Run: `env -u CC -u CXX cargo test -p saveforge-core --test settings`
Expected: 10 pass.

- [ ] **Step 3: Write the shared test helpers `crates/saveforge-host/tests/common/mod.rs`**

```rust
#![allow(dead_code)] // each test file uses a subset

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use saveforge_host::{HostLimits, PluginEngine};
use serde_json::Value;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The fixture component built by `cargo xtask plugins`.
pub fn fixture_wasm() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace().join("target"), PathBuf::from);
    let wasm = target.join("fixtures").join("test-plugin.wasm");
    assert!(
        wasm.is_file(),
        "fixture not built: run `cargo xtask plugins` first ({})",
        wasm.display()
    );
    wasm
}

/// Limits short enough to keep fault tests fast.
pub fn test_limits() -> HostLimits {
    HostLimits {
        ui_call: Duration::from_millis(200),
        io_call: Duration::from_secs(1),
        memory_bytes: 64 * 1024 * 1024,
    }
}

pub fn engine() -> PluginEngine {
    PluginEngine::new(test_limits(), None).expect("engine")
}

/// Installs the fixture as `{root}/{id}/{version}/` with its manifest's `id`
/// and `version` rewritten, then applies `edit` to the manifest JSON.
pub fn install_fixture(
    root: &Path,
    id: &str,
    version: &str,
    edit: impl FnOnce(&mut Value),
) -> PathBuf {
    let source = workspace().join("fixtures/test-plugin/manifest.json");
    let mut manifest: Value = serde_json::from_str(&fs::read_to_string(source).unwrap()).unwrap();
    manifest["id"] = Value::from(id);
    manifest["version"] = Value::from(version);
    edit(&mut manifest);

    let dir = root.join(id).join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    fs::copy(fixture_wasm(), dir.join("plugin.wasm")).unwrap();
    dir
}
```

- [ ] **Step 4: Write the failing tests `crates/saveforge-host/tests/engine.rs`**

```rust
mod common;

use std::fs;

use saveforge_core::CoreError;
use saveforge_host::{HOST_VERSION, HostError, HostLimits, PluginEngine};
use serde_json::Value;

#[test]
fn default_limits_match_spec() {
    let l = HostLimits::default();
    assert_eq!(l.ui_call.as_millis(), 250);
    assert_eq!(l.io_call.as_secs(), 10);
    assert_eq!(l.memory_bytes, 512 * 1024 * 1024);
    assert_eq!(HOST_VERSION.to_string(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn loads_fixture() {
    let root = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "test-plugin", "0.1.0", |_| {});
    let plugin = common::engine().load(&dir).unwrap();
    assert_eq!(plugin.manifest.id, "test-plugin");
    assert_eq!(plugin.dir, dir);
}

#[test]
fn rejects_incompatible_api_and_host_versions() {
    let root = tempfile::tempdir().unwrap();
    let engine = common::engine();

    let api = common::install_fixture(root.path(), "api", "0.1.0", |m| {
        m["apiVersion"] = Value::from("0.2");
    });
    assert!(matches!(
        engine.load(&api),
        Err(HostError::Core(CoreError::Incompatible(_)))
    ));

    let host = common::install_fixture(root.path(), "host", "0.1.0", |m| {
        m["minHostVersion"] = Value::from("99.0.0");
    });
    assert!(matches!(
        engine.load(&host),
        Err(HostError::Core(CoreError::Incompatible(_)))
    ));
}

#[test]
fn missing_or_invalid_component_is_compile_error() {
    let root = tempfile::tempdir().unwrap();
    let engine = common::engine();

    let missing = common::install_fixture(root.path(), "missing", "0.1.0", |m| {
        m["component"] = Value::from("nope.wasm");
    });
    assert!(matches!(engine.load(&missing), Err(HostError::Compile { .. })));

    let garbage = common::install_fixture(root.path(), "garbage", "0.1.0", |_| {});
    fs::write(garbage.join("plugin.wasm"), b"not wasm").unwrap();
    assert!(matches!(engine.load(&garbage), Err(HostError::Compile { .. })));
}

#[test]
fn component_without_editor_export_is_compile_error() {
    let root = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "empty", "0.1.0", |_| {});
    // A valid but empty component: magic, component-model version/layer.
    fs::write(dir.join("plugin.wasm"), [0u8, 0x61, 0x73, 0x6d, 0x0d, 0, 1, 0]).unwrap();
    let err = common::engine().load(&dir).unwrap_err();
    assert!(matches!(err, HostError::Compile { .. }), "{err:?}");
}

#[test]
fn compile_cache_is_written() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "test-plugin", "0.1.0", |_| {});
    let engine = PluginEngine::new(common::test_limits(), Some(cache.path())).unwrap();
    engine.load(&dir).unwrap();
    assert!(fs::read_dir(cache.path()).unwrap().count() > 0);
}
```

- [ ] **Step 5: Build the fixture and run tests to verify they fail**

Run: `env -u CC -u CXX cargo xtask plugins && env -u CC -u CXX cargo test -p saveforge-host --test engine`
Expected: compile error, unresolved crate items (`saveforge_host::PluginEngine`, …).

- [ ] **Step 6: Write `crates/saveforge-host/src/error.rs`**

```rust
use std::path::PathBuf;

use saveforge_core::CoreError;
use wasmtime::Trap;

/// Why a plugin call went wrong in a way the plugin did not choose.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PluginFault {
    #[error("timed out")]
    Timeout,
    #[error("{0}")]
    Trap(String),
    #[error("returned an invalid UI: {0}")]
    InvalidUi(String),
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("could not compile plugin component {}: {reason}", path.display())]
    Compile { path: PathBuf, reason: String },
    #[error("could not instantiate plugin: {0}")]
    Instantiate(String),
    #[error("plugin crashed: {0}")]
    Fault(PluginFault),
    #[error("plugin is no longer usable after a crash")]
    Poisoned,
    #[error("plugin could not open the file: {0}")]
    Rejected(String),
    #[error("plugin could not save the document: {0}")]
    SerializeFailed(String),
    #[error("required file '{0}' was not found")]
    MissingAux(String),
    #[error("could not start the plugin engine: {0}")]
    Engine(String),
}

/// Maps a failed guest call to a fault: epoch interrupts are timeouts,
/// everything else is a trap.
pub(crate) fn classify(error: &wasmtime::Error) -> PluginFault {
    match error.downcast_ref::<Trap>() {
        Some(Trap::Interrupt) => PluginFault::Timeout,
        Some(trap) => PluginFault::Trap(trap.to_string()),
        None => PluginFault::Trap(error.to_string()),
    }
}
```

- [ ] **Step 7: Write `crates/saveforge-host/src/stdio.rs`**

```rust
//! Routes guest stdout/stderr to `tracing` (spec §3.4).

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};

#[derive(Clone)]
pub(crate) struct LogStream {
    plugin: Arc<str>,
    stream: &'static str,
}

impl LogStream {
    pub(crate) fn new(plugin: Arc<str>, stream: &'static str) -> Self {
        Self { plugin, stream }
    }
}

impl IsTerminal for LogStream {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for LogStream {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl AsyncWrite for LogStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let text = String::from_utf8_lossy(buf);
        let text = text.trim_end();
        if !text.is_empty() {
            tracing::debug!(plugin = &*self.plugin, stream = self.stream, "{text}");
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
```

- [ ] **Step 8: Write `crates/saveforge-host/src/engine.rs`**

```rust
//! Wasmtime engine, per-store state and plugin loading (spec §3.4).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::Duration;

use saveforge_core::manifest::PluginManifest;
use saveforge_plugin_api::{API_VERSION, Level, LogHost, PluginWorld, PluginWorldPre, UiHost};
use semver::Version;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Cache, CacheConfig, Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::error::HostError;
use crate::stdio::LogStream;

/// This host's version, checked against `minHostVersion`/`maxHostVersion`.
pub static HOST_VERSION: LazyLock<Version> = LazyLock::new(|| {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("crate version is valid semver")
});

/// Epoch tick length. Call deadlines are counted in ticks.
pub const TICK: Duration = Duration::from_millis(10);

/// Per-call deadlines and per-instance memory cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostLimits {
    /// `view`, `update`, `is-dirty`, `mark-saved`, resource drop.
    pub ui_call: Duration,
    /// instantiate, `detect`, `open`, `serialize`.
    pub io_call: Duration,
    pub memory_bytes: usize,
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            ui_call: Duration::from_millis(250),
            io_call: Duration::from_secs(10),
            memory_bytes: 512 * 1024 * 1024,
        }
    }
}

/// Per-`Store` data: an empty WASI context plus limits and the plugin id for logs.
pub(crate) struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    plugin_id: Arc<str>,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl UiHost for State {}

impl LogHost for State {
    fn log(&mut self, level: Level, msg: String) {
        let plugin = &*self.plugin_id;
        match level {
            Level::Trace => tracing::trace!(plugin, "{msg}"),
            Level::Debug => tracing::debug!(plugin, "{msg}"),
            Level::Info => tracing::info!(plugin, "{msg}"),
            Level::Warn => tracing::warn!(plugin, "{msg}"),
            Level::Error => tracing::error!(plugin, "{msg}"),
        }
    }
}

/// A compiled, type-checked plugin ready to instantiate.
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub dir: PathBuf,
    pre: PluginWorldPre<State>,
}

impl fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadedPlugin")
            .field("id", &self.manifest.id)
            .field("version", &self.manifest.version)
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

/// One wasmtime engine for the app, with an epoch ticker thread that stops
/// when the engine is dropped.
pub struct PluginEngine {
    engine: Engine,
    linker: Linker<State>,
    limits: HostLimits,
    ticker_stop: Arc<AtomicBool>,
}

impl fmt::Debug for PluginEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginEngine")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl PluginEngine {
    /// `cache_dir` (absolute) enables wasmtime's on-disk compile cache.
    pub fn new(limits: HostLimits, cache_dir: Option<&Path>) -> Result<Self, HostError> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        if let Some(dir) = cache_dir {
            let mut cache = CacheConfig::new();
            cache.with_directory(dir);
            config.cache(Some(Cache::new(cache).map_err(engine_error)?));
        }
        let engine = Engine::new(&config).map_err(engine_error)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(engine_error)?;
        PluginWorld::add_to_linker::<State, HasSelf<State>>(&mut linker, |state| state)
            .map_err(engine_error)?;

        let ticker_stop = Arc::new(AtomicBool::new(false));
        spawn_ticker(engine.clone(), Arc::clone(&ticker_stop));
        Ok(Self {
            engine,
            linker,
            limits,
            ticker_stop,
        })
    }

    pub fn limits(&self) -> HostLimits {
        self.limits
    }

    /// Reads and validates the manifest, checks compatibility, compiles the
    /// component and type-checks it against the `plugin` world.
    pub fn load(&self, dir: &Path) -> Result<LoadedPlugin, HostError> {
        let manifest = PluginManifest::load(dir)?;
        manifest.check_compat(&HOST_VERSION, &API_VERSION)?;
        let path = dir.join(&manifest.component);
        let component = Component::from_file(&self.engine, &path).map_err(|e| {
            HostError::Compile {
                path: path.clone(),
                reason: e.to_string(),
            }
        })?;
        let pre = self
            .linker
            .instantiate_pre(&component)
            .and_then(PluginWorldPre::new)
            .map_err(|e| HostError::Compile {
                path,
                reason: e.to_string(),
            })?;
        Ok(LoadedPlugin {
            manifest,
            dir: dir.to_path_buf(),
            pre,
        })
    }

    pub(crate) fn new_store(&self, plugin_id: &str) -> Store<State> {
        let plugin_id: Arc<str> = Arc::from(plugin_id);
        let wasi = WasiCtx::builder()
            .stdout(LogStream::new(Arc::clone(&plugin_id), "stdout"))
            .stderr(LogStream::new(Arc::clone(&plugin_id), "stderr"))
            .build();
        let mut store = Store::new(
            &self.engine,
            State {
                wasi,
                table: ResourceTable::new(),
                limits: StoreLimitsBuilder::new()
                    .memory_size(self.limits.memory_bytes)
                    .build(),
                plugin_id,
            },
        );
        store.limiter(|state| &mut state.limits);
        store
    }

    /// Fresh store + instance for `plugin`, within the io-call deadline.
    pub(crate) fn instantiate(
        &self,
        plugin: &LoadedPlugin,
    ) -> Result<(Store<State>, PluginWorld), HostError> {
        let mut store = self.new_store(&plugin.manifest.id);
        store.set_epoch_deadline(self.io_ticks());
        let world = plugin
            .pre
            .instantiate(&mut store)
            .map_err(|e| HostError::Instantiate(e.to_string()))?;
        Ok((store, world))
    }

    pub(crate) fn ui_ticks(&self) -> u64 {
        ticks(self.limits.ui_call)
    }

    pub(crate) fn io_ticks(&self) -> u64 {
        ticks(self.limits.io_call)
    }
}

impl Drop for PluginEngine {
    fn drop(&mut self) {
        self.ticker_stop.store(true, Ordering::Relaxed);
    }
}

fn spawn_ticker(engine: Engine, stop: Arc<AtomicBool>) {
    thread::Builder::new()
        .name("saveforge-epoch".to_owned())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(TICK);
                engine.increment_epoch();
            }
        })
        .expect("spawn epoch ticker thread");
}

fn ticks(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis().div_ceil(TICK.as_millis()))
        .unwrap_or(u64::MAX)
        .max(1)
}

fn engine_error(error: wasmtime::Error) -> HostError {
    HostError::Engine(error.to_string())
}
```

- [ ] **Step 9: Write `crates/saveforge-host/src/lib.rs`**

```rust
//! Runs `SaveForge` plugin components in a sandbox (spec §3.4, §4).

mod engine;
mod error;
mod stdio;

pub use engine::{HOST_VERSION, HostLimits, LoadedPlugin, PluginEngine, TICK};
pub use error::{HostError, PluginFault};
```

- [ ] **Step 10: Run tests**

Run: `env -u CC -u CXX cargo test -p saveforge-host --test engine`
Expected: 6 tests pass.

- [ ] **Step 11: Lint gate**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings. `classify` and the `pub(crate)` engine helpers are not used until Task 6. If `dead_code` fires, add `#[allow(dead_code)] // used from Task 6` to each, and remove the attribute in Task 6.

- [ ] **Step 12: Commit**

```bash
git add Cargo.toml Cargo.lock crates
git commit -m "feat(host): wasmtime engine with limits, compile cache and plugin loading"
```

---

### Task 6: Document sessions — detect, open, view, update, serialize, faults

**Files:**
- Create: `crates/saveforge-host/src/session.rs`
- Modify: `crates/saveforge-host/src/lib.rs` (`mod session;` + export)
- Test: `crates/saveforge-host/tests/session.rs`

**Interfaces:**
- Consumes:
  - `PluginEngine::{instantiate, ui_ticks, io_ticks}`
  - `error::classify`
  - `saveforge_plugin_api::{AuxFile, PluginWorld, validate_tree, ui::{UiTree, Event}}`
- Produces:
  - `PluginEngine::detect(&self, &LoadedPlugin, file_name: &str, header: &[u8]) -> Result<Option<u32>, HostError>`
  - `PluginEngine::open(&self, &LoadedPlugin, file_name: &str, bytes: &[u8], aux: &[AuxFile]) -> Result<DocumentSession, HostError>`
  - `DocumentSession` with methods:
    - `plugin_id() -> &str`
    - `view() -> Result<UiTree, HostError>`
    - `update(&Event) -> Result<(), HostError>`
    - `is_dirty() -> Result<bool, HostError>`
    - `serialize() -> Result<Vec<u8>, HostError>`
    - `mark_saved() -> Result<(), HostError>`
    - `is_poisoned() -> bool`
  - **Fault rules:** any `Fault` poisons the session, and later calls return `Poisoned`. A `serialize` `Err` becomes `SerializeFailed` and does *not* poison.
  - re-export `saveforge_host::{AuxFile, Event, UiTree}` for callers

- [ ] **Step 1: Write the failing tests `crates/saveforge-host/tests/session.rs`**

```rust
mod common;

use std::time::{Duration, Instant};

use saveforge_host::{
    AuxFile, DocumentSession, Event, HostError, LoadedPlugin, PluginEngine, PluginFault,
};
use saveforge_plugin_api::ui::{TextStyle, Widget};

fn setup() -> (tempfile::TempDir, PluginEngine, LoadedPlugin) {
    let root = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "test-plugin", "0.1.0", |_| {});
    let engine = common::engine();
    let plugin = engine.load(&dir).unwrap();
    (root, engine, plugin)
}

fn open(engine: &PluginEngine, plugin: &LoadedPlugin, bytes: &[u8]) -> DocumentSession {
    engine.open(plugin, "a.fixture", bytes, &[]).unwrap()
}

fn shown_text(session: &mut DocumentSession) -> String {
    let tree = session.view().unwrap();
    match &tree.nodes[1].widget {
        Widget::Text(span) if span.style == TextStyle::Body => span.text.clone(),
        other => panic!("unexpected widget {other:?}"),
    }
}

fn set_text(session: &mut DocumentSession, value: &str) {
    session
        .update(&Event::TextChanged(("text".to_owned(), value.to_owned())))
        .unwrap();
}

#[test]
fn round_trip_view_update_serialize_mark_saved() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"hello");
    assert_eq!(s.plugin_id(), "test-plugin");
    assert_eq!(shown_text(&mut s), "hello");
    assert!(!s.is_dirty().unwrap());

    set_text(&mut s, "bye");
    assert!(s.is_dirty().unwrap());
    assert_eq!(shown_text(&mut s), "bye");
    assert_eq!(s.serialize().unwrap(), b"bye");
    assert!(s.is_dirty().unwrap(), "serialize must not clear dirty");
    s.mark_saved().unwrap();
    assert!(!s.is_dirty().unwrap());
}

#[test]
fn aux_files_reach_the_plugin() {
    let (_root, engine, plugin) = setup();
    let aux = [AuxFile {
        id: "greeting".to_owned(),
        bytes: b"hi".to_vec(),
    }];
    let mut s = engine.open(&plugin, "a.fixture", b"x", &aux).unwrap();
    assert_eq!(shown_text(&mut s), "x +greeting=hi");
}

#[test]
fn plugin_rejection_is_reported() {
    let (_root, engine, plugin) = setup();
    let err = engine.open(&plugin, "a.fixture", b"reject", &[]).unwrap_err();
    assert!(matches!(err, HostError::Rejected(ref m) if m == "rejected by fixture"), "{err:?}");
}

#[test]
fn panic_in_open_is_a_trap() {
    let (_root, engine, plugin) = setup();
    let err = engine.open(&plugin, "a.fixture", b"panic", &[]).unwrap_err();
    assert!(matches!(err, HostError::Fault(PluginFault::Trap(_))), "{err:?}");
}

#[test]
fn memory_hog_is_contained_and_engine_stays_usable() {
    let (_root, engine, plugin) = setup();
    let err = engine.open(&plugin, "a.fixture", b"hog", &[]).unwrap_err();
    assert!(matches!(err, HostError::Fault(PluginFault::Trap(_))), "{err:?}");
    let mut s = open(&engine, &plugin, b"still fine");
    assert_eq!(shown_text(&mut s), "still fine");
}

#[test]
fn timeout_in_update_poisons_session() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    let started = Instant::now();
    let err = s.update(&Event::Pressed("spin".to_owned())).unwrap_err();
    assert!(matches!(err, HostError::Fault(PluginFault::Timeout)), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(s.is_poisoned());
    assert!(matches!(s.view(), Err(HostError::Poisoned)));
}

#[test]
fn timeout_in_view() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    set_text(&mut s, "spin-view");
    assert!(matches!(s.view(), Err(HostError::Fault(PluginFault::Timeout))));
}

#[test]
fn invalid_tree_poisons_session() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    set_text(&mut s, "bad-tree");
    assert!(matches!(s.view(), Err(HostError::Fault(PluginFault::InvalidUi(_)))));
    assert!(matches!(s.is_dirty(), Err(HostError::Poisoned)));
}

#[test]
fn serialize_error_keeps_session_alive() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    set_text(&mut s, "unserializable");
    let err = s.serialize().unwrap_err();
    assert!(matches!(err, HostError::SerializeFailed(ref m) if m == "cannot serialize"), "{err:?}");
    assert!(!s.is_poisoned());
    assert_eq!(shown_text(&mut s), "unserializable");
}

#[test]
fn dropping_poisoned_session_is_immediate() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    let _ = s.update(&Event::Pressed("spin".to_owned()));
    assert!(s.is_poisoned());
    let started = Instant::now();
    drop(s);
    assert!(started.elapsed() < Duration::from_millis(100));
}

#[test]
fn detect_priority_none_and_timeout() {
    let (_root, engine, plugin) = setup();
    assert_eq!(engine.detect(&plugin, "a.fixture", b"x").unwrap(), Some(5));
    assert_eq!(engine.detect(&plugin, "a.fixture", b"no-match!").unwrap(), None);
    let err = engine.detect(&plugin, "spin-detect.fixture", b"").unwrap_err();
    assert!(matches!(err, HostError::Fault(PluginFault::Timeout)), "{err:?}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `env -u CC -u CXX cargo test -p saveforge-host --test session`
Expected: compile errors, unresolved `DocumentSession`, `Event`, `AuxFile`, no method `open`/`detect`.

- [ ] **Step 3: Write `crates/saveforge-host/src/session.rs`**

```rust
//! A live document: one store + instance + `document` resource (spec §3.4).

use std::fmt;

use saveforge_plugin_api::ui::{Event, UiTree};
use saveforge_plugin_api::{AuxFile, PluginWorld, validate_tree};
use wasmtime::Store;
use wasmtime::component::ResourceAny;

use crate::engine::{LoadedPlugin, PluginEngine, State};
use crate::error::{HostError, PluginFault, classify};

impl PluginEngine {
    /// Asks `plugin` whether it can open the file. Uses a throwaway instance.
    pub fn detect(
        &self,
        plugin: &LoadedPlugin,
        file_name: &str,
        header: &[u8],
    ) -> Result<Option<u32>, HostError> {
        let (mut store, world) = self.instantiate(plugin)?;
        store.set_epoch_deadline(self.io_ticks());
        world
            .saveforge_plugin_editor()
            .call_detect(&mut store, file_name, header)
            .map_err(|e| HostError::Fault(classify(&e)))
    }

    /// Opens a save in a fresh instance owned by the returned session.
    pub fn open(
        &self,
        plugin: &LoadedPlugin,
        file_name: &str,
        bytes: &[u8],
        aux: &[AuxFile],
    ) -> Result<DocumentSession, HostError> {
        let (mut store, world) = self.instantiate(plugin)?;
        store.set_epoch_deadline(self.io_ticks());
        let doc = world
            .saveforge_plugin_editor()
            .call_open(&mut store, file_name, bytes, aux)
            .map_err(|e| HostError::Fault(classify(&e)))?
            .map_err(HostError::Rejected)?;
        Ok(DocumentSession {
            plugin_id: plugin.manifest.id.clone(),
            store,
            world,
            doc,
            ui_ticks: self.ui_ticks(),
            io_ticks: self.io_ticks(),
            poisoned: false,
        })
    }
}

/// An open document. After any [`PluginFault`] the session is poisoned and
/// every later call returns [`HostError::Poisoned`].
pub struct DocumentSession {
    plugin_id: String,
    store: Store<State>,
    world: PluginWorld,
    doc: ResourceAny,
    ui_ticks: u64,
    io_ticks: u64,
    poisoned: bool,
}

impl fmt::Debug for DocumentSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentSession")
            .field("plugin_id", &self.plugin_id)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl DocumentSession {
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn view(&mut self) -> Result<UiTree, HostError> {
        let tree = self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_view(s, d)
        })?;
        if let Err(reason) = validate_tree(&tree) {
            self.poisoned = true;
            return Err(HostError::Fault(PluginFault::InvalidUi(reason)));
        }
        Ok(tree)
    }

    pub fn update(&mut self, event: &Event) -> Result<(), HostError> {
        self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_update(s, d, event)
        })
    }

    pub fn is_dirty(&mut self) -> Result<bool, HostError> {
        self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_is_dirty(s, d)
        })
    }

    /// The plugin's bytes for the current state. A plugin-reported error is
    /// [`HostError::SerializeFailed`] and leaves the session usable.
    pub fn serialize(&mut self) -> Result<Vec<u8>, HostError> {
        self.call(self.io_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_serialize(s, d)
        })?
        .map_err(HostError::SerializeFailed)
    }

    pub fn mark_saved(&mut self) -> Result<(), HostError> {
        self.call(self.ui_ticks, |w, s, d| {
            w.saveforge_plugin_editor().document().call_mark_saved(s, d)
        })
    }

    fn call<R>(
        &mut self,
        ticks: u64,
        f: impl FnOnce(&PluginWorld, &mut Store<State>, ResourceAny) -> wasmtime::Result<R>,
    ) -> Result<R, HostError> {
        if self.poisoned {
            return Err(HostError::Poisoned);
        }
        self.store.set_epoch_deadline(ticks);
        f(&self.world, &mut self.store, self.doc).map_err(|e| {
            self.poisoned = true;
            HostError::Fault(classify(&e))
        })
    }
}

impl Drop for DocumentSession {
    fn drop(&mut self) {
        // A poisoned instance may be mid-trap; never re-enter it.
        if self.poisoned {
            return;
        }
        self.store.set_epoch_deadline(self.ui_ticks);
        if let Err(e) = self.doc.resource_drop(&mut self.store) {
            tracing::debug!(plugin = %self.plugin_id, error = %e, "document drop failed");
        }
    }
}
```

- [ ] **Step 4: Export from `crates/saveforge-host/src/lib.rs`**

```rust
//! Runs `SaveForge` plugin components in a sandbox (spec §3.4, §4).

mod engine;
mod error;
mod session;
mod stdio;

pub use engine::{HOST_VERSION, HostLimits, LoadedPlugin, PluginEngine, TICK};
pub use error::{HostError, PluginFault};
pub use saveforge_plugin_api::AuxFile;
pub use saveforge_plugin_api::ui::{Event, UiTree};
pub use session::DocumentSession;
```

If Task 5 added `#[allow(dead_code)]` attributes, remove them now.

- [ ] **Step 5: Run tests**

Run: `env -u CC -u CXX cargo test -p saveforge-host --test session`
Expected: 11 tests pass (the timeout tests take about 0.2–1 s each).

If `call_update` expects the event by value rather than `&Event`, change the closure to `call_update(s, d, event)` with `event: &Event` dereferenced/cloned per the compiler message, and keep the public `update(&Event)` signature. Ledger this as a ruling.

Integration tests can import `saveforge_plugin_api` without a dev-dependency because it is already a normal dependency of `saveforge-host`.

- [ ] **Step 6: Lint gate**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/saveforge-host Cargo.lock
git commit -m "feat(host): document sessions with timeouts, traps and poisoning"
```

---

### Task 7: Aux-file resolution

**Files:**
- Create: `crates/saveforge-host/src/aux.rs`
- Modify: `crates/saveforge-host/src/lib.rs`
- Test: `crates/saveforge-host/tests/aux.rs`

**Interfaces:**
- Consumes: `saveforge_core::{manifest::{PluginManifest, AuxFileSpec}, tokens::{TokenEnv, Expanded, expand}}`, `AuxFile`, `HostError::MissingAux`.
- Produces: `saveforge_host::resolve_aux_files(manifest: &PluginManifest, env: &TokenEnv) -> Result<Vec<AuxFile>, HostError>`.
  - Within each spec the candidates are tried in order. The first one that expands to an existing file of at most `max_bytes` wins.
  - A missing optional file is omitted. A missing required file returns `MissingAux(id)`.

- [ ] **Step 1: Write the failing tests `crates/saveforge-host/tests/aux.rs`**

```rust
use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::manifest::PluginManifest;
use saveforge_core::platform::Os;
use saveforge_core::tokens::TokenEnv;
use saveforge_host::{HostError, resolve_aux_files};
use serde_json::json;

fn env(appdata: &Path) -> TokenEnv {
    TokenEnv {
        os: Os::current(),
        appdata: Some(appdata.to_path_buf()),
        local_appdata: None,
        home: None,
        temp: std::env::temp_dir(),
        steam: None,
    }
}

fn manifest(aux: serde_json::Value) -> PluginManifest {
    PluginManifest::parse(
        &json!({
            "id": "p", "name": "P", "version": "0.1.0", "author": "a",
            "apiVersion": "0.1", "minHostVersion": "0.1.0",
            "component": "plugin.wasm", "fileExtensions": [".x"],
            "auxFiles": aux
        })
        .to_string(),
    )
    .unwrap()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn first_existing_candidate_wins() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "second.txt", b"two");
    write(dir.path(), "third.txt", b"three");
    let m = manifest(json!([{ "id": "loc",
        "candidates": ["%APPDATA%/first.txt", "%APPDATA%/second.txt", "%APPDATA%/third.txt"] }]));
    let files = resolve_aux_files(&m, &env(dir.path())).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].id, "loc");
    assert_eq!(files[0].bytes, b"two");
}

#[test]
fn optional_missing_is_omitted_required_missing_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let optional = manifest(json!([{ "id": "opt", "optional": true, "candidates": ["%APPDATA%/none.txt"] }]));
    assert_eq!(resolve_aux_files(&optional, &env(dir.path())).unwrap(), vec![]);

    let required = manifest(json!([{ "id": "req", "candidates": ["%APPDATA%/none.txt"] }]));
    let err = resolve_aux_files(&required, &env(dir.path())).unwrap_err();
    assert!(matches!(err, HostError::MissingAux(ref id) if id == "req"), "{err:?}");
}

#[test]
fn oversized_candidate_falls_through() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "big.txt", &[b'x'; 10]);
    write(dir.path(), "small.txt", b"ok");
    let m = manifest(json!([{ "id": "loc", "maxBytes": 5,
        "candidates": ["%APPDATA%/big.txt", "%APPDATA%/small.txt"] }]));
    assert_eq!(resolve_aux_files(&m, &env(dir.path())).unwrap()[0].bytes, b"ok");

    let only_big = manifest(json!([{ "id": "loc", "maxBytes": 5, "candidates": ["%APPDATA%/big.txt"] }]));
    assert!(matches!(
        resolve_aux_files(&only_big, &env(dir.path())),
        Err(HostError::MissingAux(_))
    ));
}

#[test]
fn unresolvable_and_directory_candidates_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("adir")).unwrap();
    write(dir.path(), "real.txt", b"r");
    let m = manifest(json!([{ "id": "loc",
        "candidates": ["{steam_app:1}/x.txt", "relative.txt", "%APPDATA%/adir", "%APPDATA%/real.txt"] }]));
    assert_eq!(resolve_aux_files(&m, &env(dir.path())).unwrap()[0].bytes, b"r");
}

#[test]
fn preserves_spec_order_for_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", b"A");
    write(dir.path(), "b.txt", b"B");
    let m = manifest(json!([
        { "id": "b", "candidates": ["%APPDATA%/b.txt"] },
        { "id": "a", "candidates": ["%APPDATA%/a.txt"] }
    ]));
    let ids: Vec<_> = resolve_aux_files(&m, &env(dir.path()))
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(ids, ["b", "a"]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `env -u CC -u CXX cargo test -p saveforge-host --test aux`
Expected: compile error, unresolved import `resolve_aux_files`.

- [ ] **Step 3: Write `crates/saveforge-host/src/aux.rs`**

```rust
//! Locates the manifest's auxiliary files for `open` (spec §4.2, §4.5).

use saveforge_core::manifest::{AuxFileSpec, PluginManifest};
use saveforge_core::tokens::{Expanded, TokenEnv, expand};
use saveforge_plugin_api::AuxFile;

use crate::error::HostError;

/// Reads each declared aux file from its first usable candidate, in manifest
/// order. A missing optional file is omitted; a missing required one is an error.
pub fn resolve_aux_files(
    manifest: &PluginManifest,
    env: &TokenEnv,
) -> Result<Vec<AuxFile>, HostError> {
    let mut files = Vec::new();
    for spec in &manifest.aux_files {
        match read_first_candidate(spec, env) {
            Some(bytes) => files.push(AuxFile {
                id: spec.id.clone(),
                bytes,
            }),
            None if spec.optional => {
                tracing::debug!(plugin = %manifest.id, aux = %spec.id, "optional aux file not found");
            }
            None => return Err(HostError::MissingAux(spec.id.clone())),
        }
    }
    Ok(files)
}

fn read_first_candidate(spec: &AuxFileSpec, env: &TokenEnv) -> Option<Vec<u8>> {
    spec.candidates.iter().find_map(|raw| {
        let Expanded::Resolved(path) = expand(raw, env) else {
            return None;
        };
        let meta = std::fs::metadata(&path).ok()?;
        if !meta.is_file() {
            return None;
        }
        if meta.len() > spec.max_bytes {
            tracing::warn!(aux = %spec.id, path = %path.display(), size = meta.len(), "aux file too large");
            return None;
        }
        let bytes = std::fs::read(&path).ok()?;
        // The file may have grown since the metadata call.
        (u64::try_from(bytes.len()).unwrap_or(u64::MAX) <= spec.max_bytes).then_some(bytes)
    })
}
```

- [ ] **Step 4: Export from `lib.rs`**

Add `mod aux;` (keep modules sorted) and `pub use aux::resolve_aux_files;`.

- [ ] **Step 5: Run tests**

Run: `env -u CC -u CXX cargo test -p saveforge-host --test aux`
Expected: 5 tests pass.

- [ ] **Step 6: Lint gate**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/saveforge-host
git commit -m "feat(host): aux file resolution with size cap"
```

---

### Task 8: Plugin registry and detection

**Files:**
- Create: `crates/saveforge-host/src/registry.rs`, `crates/saveforge-host/src/detect.rs`
- Modify: `crates/saveforge-host/src/lib.rs`
- Test: `crates/saveforge-host/tests/registry.rs`, `crates/saveforge-host/tests/detect.rs`

**Interfaces:**
- Consumes: `PluginEngine::{load, detect}`, `LoadedPlugin`, `PluginManifest::handles_extension`.
- Produces:
  - `PluginEntry { Loaded(Arc<LoadedPlugin>), Failed { id: String, dir: PathBuf, reason: String } }` with `id() -> &str`
  - `PluginRegistry` (`Default`, `Debug`) with methods:
    - `scan(&PluginEngine, plugins_dir: &Path) -> Self`
    - `entries() -> &[PluginEntry]`
    - `loaded() -> impl Iterator<Item = &Arc<LoadedPlugin>>`
    - `get(&str) -> Option<&PluginEntry>`
    - `record_fault(&mut self, id: &str) -> bool` (returns true when this fault disabled the plugin)
  - `MAX_FAULTS_PER_SESSION: u8 = 3`
  - `HEADER_LEN: usize = 512`
  - `Detection { NoMatch, Winner(Arc<LoadedPlugin>), Tie(Vec<Arc<LoadedPlugin>>) }`
  - `DetectReport { detection: Detection, faults: Vec<(String, HostError)> }`
  - `detect(&PluginEngine, &PluginRegistry, path: &Path, header: &[u8]) -> DetectReport`
  - **Scan rules:**
    - The highest SemVer version folder per id wins.
    - Folders that aren't SemVer, and dot-folders, are ignored.
    - Manifest id or version not matching the folder means `Failed`.
    - Entries are sorted by id.

- [ ] **Step 1: Write the failing tests `crates/saveforge-host/tests/registry.rs`**

```rust
mod common;

use std::fs;

use saveforge_host::{MAX_FAULTS_PER_SESSION, PluginEntry, PluginRegistry};
use serde_json::Value;

fn failed_reason(registry: &PluginRegistry, id: &str) -> String {
    match registry.get(id) {
        Some(PluginEntry::Failed { reason, .. }) => reason.clone(),
        other => panic!("expected Failed for {id}, got {other:?}"),
    }
}

#[test]
fn missing_plugins_dir_is_empty() {
    let root = tempfile::tempdir().unwrap();
    let registry = PluginRegistry::scan(&common::engine(), &root.path().join("nope"));
    assert_eq!(registry.entries().len(), 0);
}

#[test]
fn newest_version_wins() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix", "0.1.0", |_| {});
    let newest = common::install_fixture(root.path(), "fix", "0.10.0", |_| {});
    common::install_fixture(root.path(), "fix", "0.9.0", |_| {});
    let registry = PluginRegistry::scan(&common::engine(), root.path());
    let loaded: Vec<_> = registry.loaded().collect();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].dir, newest);
    assert_eq!(loaded[0].manifest.version.to_string(), "0.10.0");
}

#[test]
fn non_semver_and_dot_folders_are_ignored() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix", "0.1.0", |_| {});
    fs::create_dir_all(root.path().join("fix/latest")).unwrap();
    fs::create_dir_all(root.path().join("fix/1.0")).unwrap();
    fs::create_dir_all(root.path().join(".staging/abc")).unwrap();
    fs::write(root.path().join("stray-file.txt"), b"x").unwrap();
    let registry = PluginRegistry::scan(&common::engine(), root.path());
    let ids: Vec<_> = registry.entries().iter().map(PluginEntry::id).collect();
    assert_eq!(ids, ["fix"]);
    assert_eq!(registry.loaded().count(), 1);
}

#[test]
fn failures_are_recorded_with_reasons() {
    let root = tempfile::tempdir().unwrap();
    let engine = common::engine();
    common::install_fixture(root.path(), "ok", "0.1.0", |_| {});
    common::install_fixture(root.path(), "api", "0.1.0", |m| m["apiVersion"] = Value::from("0.2"));
    common::install_fixture(root.path(), "renamed", "0.1.0", |m| m["id"] = Value::from("other"));
    common::install_fixture(root.path(), "wrongver", "0.2.0", |m| m["version"] = Value::from("0.3.0"));
    fs::create_dir_all(root.path().join("empty")).unwrap();

    let registry = PluginRegistry::scan(&engine, root.path());
    let ids: Vec<_> = registry.entries().iter().map(PluginEntry::id).collect();
    assert_eq!(ids, ["api", "empty", "ok", "renamed", "wrongver"]);
    assert_eq!(registry.loaded().count(), 1);
    assert!(failed_reason(&registry, "api").contains("API"));
    assert!(failed_reason(&registry, "renamed").contains("does not match"));
    assert!(failed_reason(&registry, "wrongver").contains("does not match"));
    assert!(failed_reason(&registry, "empty").contains("no installed version"));
}

#[test]
fn third_fault_disables_plugin() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix", "0.1.0", |_| {});
    let mut registry = PluginRegistry::scan(&common::engine(), root.path());
    for _ in 1..MAX_FAULTS_PER_SESSION {
        assert!(!registry.record_fault("fix"));
    }
    assert!(registry.record_fault("fix"));
    assert_eq!(registry.loaded().count(), 0);
    assert!(failed_reason(&registry, "fix").contains("crash"));
    assert!(!registry.record_fault("fix"), "already disabled");
    assert!(!registry.record_fault("unknown"));
}
```

- [ ] **Step 2: Write the failing tests `crates/saveforge-host/tests/detect.rs`**

```rust
mod common;

use std::path::Path;

use saveforge_host::{Detection, HostError, PluginEngine, PluginFault, PluginRegistry, detect};
use serde_json::Value;

/// Engine and registry must come from the same engine: a `LoadedPlugin` can
/// only be instantiated by the engine that compiled it.
fn setup(root: &Path) -> (PluginEngine, PluginRegistry) {
    let engine = common::engine();
    let registry = PluginRegistry::scan(&engine, root);
    (engine, registry)
}

#[test]
fn single_match_wins() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    common::install_fixture(root.path(), "fix-c", "0.1.0", |m| {
        m["fileExtensions"] = Value::from(vec![".other"]);
    });
    let (engine, registry) = setup(root.path());
    let report = detect(&engine, &registry, Path::new("s.fixture"), b"x");
    assert_eq!(report.faults.len(), 0);
    match report.detection {
        Detection::Winner(p) => assert_eq!(p.manifest.id, "fix-a"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn equal_priority_is_a_tie() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    common::install_fixture(root.path(), "fix-b", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    let report = detect(&engine, &registry, Path::new("s.fixture"), b"x");
    match report.detection {
        Detection::Tie(ps) => {
            let ids: Vec<_> = ps.iter().map(|p| p.manifest.id.as_str()).collect();
            assert_eq!(ids, ["fix-a", "fix-b"]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn no_match_when_extension_or_plugin_declines() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    assert!(matches!(
        detect(&engine, &registry, Path::new("s.txt"), b"x").detection,
        Detection::NoMatch
    ));
    assert!(matches!(
        detect(&engine, &registry, Path::new("s.fixture"), b"no-match").detection,
        Detection::NoMatch
    ));
}

#[test]
fn long_header_is_truncated_to_512_bytes() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    let header = vec![b'x'; 4096];
    let report = detect(&engine, &registry, Path::new("s.fixture"), &header);
    assert!(matches!(report.detection, Detection::Winner(_)));
}

#[test]
fn faulting_plugin_is_reported_not_fatal() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    let report = detect(&engine, &registry, Path::new("spin-detect.fixture"), b"");
    assert!(matches!(report.detection, Detection::NoMatch));
    assert_eq!(report.faults.len(), 1);
    assert_eq!(report.faults[0].0, "fix-a");
    assert!(matches!(report.faults[0].1, HostError::Fault(PluginFault::Timeout)));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `env -u CC -u CXX cargo test -p saveforge-host --test registry --test detect`
Expected: compile errors, unresolved `PluginRegistry`, `PluginEntry`, `detect`, `Detection`, `MAX_FAULTS_PER_SESSION`.

- [ ] **Step 4: Write `crates/saveforge-host/src/registry.rs`**

```rust
//! Installed plugins: `{plugins}/{id}/{version}/` scanning and session fault
//! tracking (spec §5.2 Startup, §7).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use semver::Version;

use crate::engine::{LoadedPlugin, PluginEngine};

/// Faults in one session after which a plugin is disabled until restart.
pub const MAX_FAULTS_PER_SESSION: u8 = 3;

#[derive(Debug, Clone)]
pub enum PluginEntry {
    Loaded(Arc<LoadedPlugin>),
    Failed {
        id: String,
        dir: PathBuf,
        reason: String,
    },
}

impl PluginEntry {
    pub fn id(&self) -> &str {
        match self {
            Self::Loaded(plugin) => &plugin.manifest.id,
            Self::Failed { id, .. } => id,
        }
    }
}

#[derive(Debug, Default)]
pub struct PluginRegistry {
    entries: Vec<PluginEntry>,
    faults: HashMap<String, u8>,
}

impl PluginRegistry {
    /// Loads the newest version of every plugin under `plugins_dir`. Never
    /// fails: problems become [`PluginEntry::Failed`] with a reason.
    pub fn scan(engine: &PluginEngine, plugins_dir: &Path) -> Self {
        let Ok(id_dirs) = fs::read_dir(plugins_dir) else {
            return Self::default();
        };
        let mut entries = Vec::new();
        for id_dir in id_dirs.flatten() {
            let id = id_dir.file_name().to_string_lossy().into_owned();
            let path = id_dir.path();
            if id.starts_with('.') || !path.is_dir() {
                continue;
            }
            entries.push(load_newest(engine, id, &path));
        }
        entries.sort_by(|a, b| a.id().cmp(b.id()));
        Self {
            entries,
            faults: HashMap::new(),
        }
    }

    pub fn entries(&self) -> &[PluginEntry] {
        &self.entries
    }

    pub fn loaded(&self) -> impl Iterator<Item = &Arc<LoadedPlugin>> {
        self.entries.iter().filter_map(|entry| match entry {
            PluginEntry::Loaded(plugin) => Some(plugin),
            PluginEntry::Failed { .. } => None,
        })
    }

    pub fn get(&self, id: &str) -> Option<&PluginEntry> {
        self.entries.iter().find(|entry| entry.id() == id)
    }

    /// Counts a fault. Returns `true` when this fault disabled the plugin.
    pub fn record_fault(&mut self, id: &str) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id() == id) else {
            return false;
        };
        let PluginEntry::Loaded(plugin) = entry else {
            return false;
        };
        let count = self.faults.entry(id.to_owned()).or_insert(0);
        *count = count.saturating_add(1);
        if *count < MAX_FAULTS_PER_SESSION {
            return false;
        }
        let dir = plugin.dir.clone();
        *entry = PluginEntry::Failed {
            id: id.to_owned(),
            dir,
            reason: format!("disabled after {MAX_FAULTS_PER_SESSION} crashes this session"),
        };
        true
    }
}

fn load_newest(engine: &PluginEngine, id: String, id_dir: &Path) -> PluginEntry {
    let Some((version, dir)) = newest_version(id_dir) else {
        return PluginEntry::Failed {
            id,
            dir: id_dir.to_path_buf(),
            reason: "no installed version".to_owned(),
        };
    };
    let reason = match engine.load(&dir) {
        Ok(plugin) if plugin.manifest.id != id => format!(
            "manifest id '{}' does not match folder '{id}'",
            plugin.manifest.id
        ),
        Ok(plugin) if plugin.manifest.version != version => format!(
            "manifest version {} does not match folder {version}",
            plugin.manifest.version
        ),
        Ok(plugin) => return PluginEntry::Loaded(Arc::new(plugin)),
        Err(e) => e.to_string(),
    };
    tracing::warn!(plugin = %id, %reason, "plugin failed to load");
    PluginEntry::Failed { id, dir, reason }
}

fn newest_version(id_dir: &Path) -> Option<(Version, PathBuf)> {
    fs::read_dir(id_dir)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let version = Version::parse(&entry.file_name().to_string_lossy()).ok()?;
            Some((version, entry.path()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
}
```

Note on `record_fault`: the borrow of `plugin` (from `entry`) ends at `plugin.dir.clone()`, before `*entry` is reassigned. If the borrow checker rejects it because `self.faults` is borrowed while `entry` is live, move the `self.faults` update above the `find` and keep the logic the same. Ledger it as a ruling.

- [ ] **Step 5: Write `crates/saveforge-host/src/detect.rs`**

```rust
//! Which plugin opens a file (spec §5.2 Open).

use std::path::Path;
use std::sync::Arc;

use crate::engine::{LoadedPlugin, PluginEngine};
use crate::error::HostError;
use crate::registry::PluginRegistry;

/// Plugins see at most this many leading bytes of the file.
pub const HEADER_LEN: usize = 512;

#[derive(Debug, Clone)]
pub enum Detection {
    NoMatch,
    Winner(Arc<LoadedPlugin>),
    /// Several plugins share the highest priority; the user must pick.
    Tie(Vec<Arc<LoadedPlugin>>),
}

#[derive(Debug)]
pub struct DetectReport {
    pub detection: Detection,
    /// Plugins whose `detect` failed (id, error). Feed these to
    /// `PluginRegistry::record_fault`.
    pub faults: Vec<(String, HostError)>,
}

/// Asks every loaded plugin that handles the file's extension. Highest
/// priority wins; equal top priorities are a tie (in registry order).
pub fn detect(
    engine: &PluginEngine,
    registry: &PluginRegistry,
    path: &Path,
    header: &[u8],
) -> DetectReport {
    let header = &header[..header.len().min(HEADER_LEN)];
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut matches = Vec::new();
    let mut faults = Vec::new();
    for plugin in registry
        .loaded()
        .filter(|p| p.manifest.handles_extension(path))
    {
        match engine.detect(plugin, &file_name, header) {
            Ok(Some(priority)) => matches.push((priority, Arc::clone(plugin))),
            Ok(None) => {}
            Err(e) => faults.push((plugin.manifest.id.clone(), e)),
        }
    }

    let Some(top) = matches.iter().map(|(priority, _)| *priority).max() else {
        return DetectReport {
            detection: Detection::NoMatch,
            faults,
        };
    };
    let mut winners: Vec<_> = matches
        .into_iter()
        .filter(|(priority, _)| *priority == top)
        .map(|(_, plugin)| plugin)
        .collect();
    let detection = if winners.len() == 1 {
        Detection::Winner(winners.remove(0))
    } else {
        Detection::Tie(winners)
    };
    DetectReport { detection, faults }
}
```

- [ ] **Step 6: Final `crates/saveforge-host/src/lib.rs`**

```rust
//! Runs `SaveForge` plugin components in a sandbox (spec §3.4, §4).

mod aux;
mod detect;
mod engine;
mod error;
mod registry;
mod session;
mod stdio;

pub use aux::resolve_aux_files;
pub use detect::{DetectReport, Detection, HEADER_LEN, detect};
pub use engine::{HOST_VERSION, HostLimits, LoadedPlugin, PluginEngine, TICK};
pub use error::{HostError, PluginFault};
pub use registry::{MAX_FAULTS_PER_SESSION, PluginEntry, PluginRegistry};
pub use saveforge_plugin_api::AuxFile;
pub use saveforge_plugin_api::ui::{Event, UiTree};
pub use session::DocumentSession;
```

- [ ] **Step 7: Run tests**

Run: `env -u CC -u CXX cargo test -p saveforge-host --test registry --test detect`
Expected: registry 5 pass, detect 5 pass.

- [ ] **Step 8: Lint gate**

Run: `cargo fmt --all && env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/saveforge-host
git commit -m "feat(host): plugin registry and detection voting"
```

---

### Task 9: Full gate, roadmap

**Files:**
- Modify: `deny.toml` (only if new licenses appear), `docs/superpowers/plans/2026-10-10-roadmap.md`

**Interfaces:**
- Consumes: everything above.
- Produces: a green branch ready to merge.

- [ ] **Step 1: Run the full gate exactly as CI does**

Run:
```bash
cargo fmt --all --check
env -u CC -u CXX cargo clippy --workspace --all-targets -- -D warnings
env -u CC -u CXX cargo clippy -p saveforge-sdk -p test-plugin --target wasm32-wasip2 -- -D warnings
env -u CC -u CXX cargo xtask plugins
env -u CC -u CXX cargo test --workspace
env -u CC -u CXX cargo deny check
cargo machete
```

Expected:
- fmt prints nothing.
- Both clippy runs report 0 warnings.
- `xtask` prints `test-plugin -> …/target/fixtures/test-plugin.wasm`.
- All test suites pass: core, plugin-api tree, sdk export/ui, xtask, host engine/session/aux/registry/detect.
- `cargo deny check` prints `advisories ok, bans ok, licenses ok, sources ok`.
- machete reports no unused dependencies.

**If `cargo deny check licenses` rejects a license:** look it up. If it is OSI-approved and permissive (e.g. `CC0-1.0`, `MIT-0`, `Unicode-DFS-2016`, `BSL-1.0`), add it to `allow` in `deny.toml` and name it in the commit message. If it is not, stop and ask the user. wasmtime's own crates are `Apache-2.0 WITH LLVM-exception`, which is already allowed.

**If `cargo deny check advisories` reports an advisory in the wasmtime/WASI tree:** stop and report it to the user, with the advisory id and the crate path.

- [ ] **Step 2: Mark M2 done in the roadmap**

In `docs/superpowers/plans/2026-10-10-roadmap.md`, change the M2 row to:

```markdown
| 2 | WIT, `saveforge-plugin-api`, `saveforge-sdk` builders, `saveforge-host` + fixture plugins, `xtask plugins` | `2026-10-10-m2-plugin-runtime.md` | done |
```

- [ ] **Step 3: Commit**

```bash
git add deny.toml docs/superpowers/plans/2026-10-10-roadmap.md Cargo.lock
git commit -m "chore: pass full M2 quality gate"
```

- [ ] **Step 4: Hand off**

Use superpowers:finishing-a-development-branch.
