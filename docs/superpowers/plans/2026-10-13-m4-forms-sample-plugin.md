# Milestone 4: Schema Forms + Sample Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two deliverables:
- **`saveforge_sdk::form`:** a plugin describes its editor as a `FormSchema` and gets a complete `Document`. That includes rendering, edits by dot path, lists, nested objects, conditional fields, validation that blocks invalid saves, and a lossless JSON codec.
- **The StardewLike sample plugin built on it**, the first real plugin the app can open, verified end to end through its wasm build.

**Architecture:** The form helper lives in the guest SDK, so the host keeps its single UI-tree renderer (spec §4.6).
- **`FormDocument<C: Codec>`** holds a `serde_json::Value` (`preserve_order`) and implements the SDK `Document` trait. It renders pages as tabs and fields through the existing `saveforge_sdk::ui` builders, and maps events back to dot paths.
- **`sample-stardew`** is a schema plus `export_plugin!`.
- **An app test** loads the real component from `target/dev-plugins` and runs detect → open → edit → save.

**Tech Stack:** Rust 1.99 (edition 2024), `serde_json` with `preserve_order` (SDK), `regex-lite` 0.1.9 (pattern validation, small wasm footprint). Existing: `saveforge-sdk` (wit-bindgen 0.62), `saveforge-app`, `xtask`.

**Spec:** `docs/superpowers/specs/2026-10-10-rust-iced-rewrite-design.md` (§4.6 `form` module, §6.2 sample plugin, §8 `saveforge-sdk` and plugin rows)

**Prior milestones:** M1–M3 are merged. The APIs used here:
- `saveforge_sdk::{Plugin, Document, AuxFile, export_plugin!}`
- `saveforge_sdk::ui::{El, UiTree, Event, Widget, FieldSpec, Input, column, section, tabs, text, text_input, number, checkbox, pick, option, list, button}`, with `FieldEl::{help, error, read_only, max_len, min, max, decimals}` and `ListEl::removable`
- `saveforge_app::{documents::{detect_file, open_document, save_document, DetectOutcome}, editor::Editor, render::{render, UiMessage}}`
- `saveforge_host::PluginRegistry`
- `cargo xtask plugins`, which installs `plugins/*` into `target/dev-plugins/{id}/{version}/`

**Verified draft:** every code block was compiled, linted (clippy pedantic `-D warnings`, native and `wasm32-wasip2`) and tested in a scratch copy of `main` before this plan was written: 207 workspace tests green, `cargo deny` and `cargo machete` clean.

## Global Constraints

- **Dependencies:** `regex-lite = "0.1.9"` (workspace). The SDK uses `serde_json = { workspace = true, features = ["preserve_order"] }`, so saves keep their key order.
- **Field ids** are dot paths: `player.stats.health`, with list items as `inventory.0.id`. Reserved widget ids: `__tabs` (page tabs) and `<list path>.__add` (a list's Add button). Schema keys must not start with `__`.
- **`FormDocument::open`** requires a JSON **object** root (`Err("the save is not a JSON object")`).
- **Number storage:** fields with `decimals = 0` store whole numbers as JSON integers (`12`, not `12.0`, up to ±2^53).
- **Validation texts** (exact):
  - `{label} is required.`
  - `{label} must not exceed {n} characters.`
  - `{label} has an invalid format.`
  - `{label} must be between {min} and {max}.`
  - `{label} must be at least {min}.`
  - `{label} must be at most {max}.`
- **Blocked saves:** `serialize` returns `Err("Fix {n} invalid field(s) before saving: …")` while any *visible* field is invalid. Hidden fields are neither rendered nor validated.
- **Sample plugin manifest:**
  - id `stardew-like`, version `0.1.0`, `minHostVersion` `0.1.0`, `apiVersion` `0.1`
  - extensions `[".json"]`, detection priority 5 when the header contains `"player"`
  - default saves `%APPDATA%/StardewValley/Saves` (Windows), `~/.config/StardewValley/Saves` (Linux and macOS)
- **Tests:** plugin and app tests that load wasm need `cargo xtask plugins` first, which now builds the sample plugin as well.
- **Lints:** clippy 1.99 rejects `assert!(x.is_empty())`; use `assert_eq!`. Strict float `==` in tests is rejected too; compare with `f64::EPSILON`.
- **Branch:** work on `feat/m4-forms-sample-plugin`, never on `main`.

## Review Focus

Inputs a real save can contain that the spec implies, each pinned by a test in Task 2 (`form_document.rs`):

1. **A number stored as a string** (`"level": "12"`). Expected: the field shows 12, not 0. Test: `numbers_stored_as_strings_show_their_value`.
2. **List items that aren't objects** (`"inventory": ["seeds", 7]`). Expected: renders without crashing, and edits are ignored rather than corrupting data. Test: `non_object_list_items_render_and_ignore_edits`.
3. **Multi-byte text in a `max_len` field** (`Žluťoučký`). Expected: counted in characters, not bytes. Test: `max_len_counts_characters_not_bytes`.
4. **A malformed `pattern` in a plugin's schema.** Expected: the rule is ignored instead of making every value invalid, so the save stays possible. Test: `malformed_pattern_in_schema_is_ignored`.
5. **A save whose root is not a JSON object.** Expected: a clear open error. Test: `root_must_be_a_json_object`.

---

## File Structure

```
rust/
  Cargo.toml                                   # + member plugins/sample-stardew, regex-lite
  .github/workflows/ci.yml                     # wasm clippy includes sample-stardew
  docs/sample-saves/StardewLike-Farm.json      # sample save for tests and manual runs
  sdk/saveforge-sdk/
    Cargo.toml                                 # + regex-lite, serde_json (preserve_order)
    src/lib.rs                                 # + pub mod form
    src/form/mod.rs                            # re-exports
    src/form/path.rs                           # get/get_mut/set/join on dot paths
    src/form/visibility.rs                     # VisibilityRule (port of .NET)
    src/form/schema.rs                         # FormSchema, FormPage, FormSection, FieldDef + builders
    src/form/document.rs                       # FormDocument<C>, Codec, JsonCodec, validation
    tests/form_path.rs  tests/form_visibility.rs  tests/form_document.rs
  plugins/sample-stardew/
    Cargo.toml  manifest.json  src/lib.rs  tests/stardew.rs
  crates/saveforge-app/tests/sample_plugin.rs  # end to end through the wasm build
```

---

### Task 1: Dot paths and visibility rules

**Files:**
- Modify: `Cargo.toml` (workspace dep `regex-lite`), `sdk/saveforge-sdk/Cargo.toml`, `sdk/saveforge-sdk/src/lib.rs`
- Create: `sdk/saveforge-sdk/src/form/mod.rs` (interim), `sdk/saveforge-sdk/src/form/path.rs`, `sdk/saveforge-sdk/src/form/visibility.rs`
- Test: `sdk/saveforge-sdk/tests/form_path.rs`, `sdk/saveforge-sdk/tests/form_visibility.rs`

**Interfaces:**
- Produces:
  - `saveforge_sdk::form::path::{get(&Value, &str) -> Option<&Value>, get_mut(&mut Value, &str) -> Option<&mut Value>, set(&mut Value, &str, Value) -> bool, join(&str, &str) -> String}`.
    - `set` creates missing objects along the way.
    - Array segments must index an existing element.
    - It returns whether anything changed.
  - `saveforge_sdk::form::{VisibilityOp { Equals, NotEquals, GreaterThan, LessThan }, VisibilityRule { depends_on: String, op, value: Value }}` with `VisibilityRule::new(impl Into<String>, VisibilityOp, impl Into<Value>)` and `evaluate(Option<&Value>) -> bool`.
    - Missing counts as null.
    - Numbers compare numerically, also when one side is a numeric string.
    - Strings order lexically. Values that can't be ordered are neither greater nor less.

- [ ] **Step 1: Create the branch**

```bash
git checkout -b feat/m4-forms-sample-plugin
```

- [ ] **Step 2: Dependencies**

In the workspace `Cargo.toml` `[workspace.dependencies]`, add (sorted, before `rfd`):

```toml
regex-lite = "0.1.9"
```

Replace `sdk/saveforge-sdk/Cargo.toml` with:

```toml
[package]
name = "saveforge-sdk"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
regex-lite.workspace = true
serde_json = { workspace = true, features = ["preserve_order"] }
wit-bindgen.workspace = true

[lints]
workspace = true
```

`regex-lite` is unused until Task 2. `cargo machete` only runs in Task 5.

- [ ] **Step 3: Write the failing tests**

`sdk/saveforge-sdk/tests/form_path.rs`:

```rust
use saveforge_sdk::form::path::{get, get_mut, join, set};
use serde_json::{Value, json};

#[test]
fn get_walks_objects_and_arrays() {
    let v =
        json!({ "player": { "stats": { "hp": 5 } }, "items": [ { "id": "a" }, { "id": "b" } ] });
    assert_eq!(get(&v, "player.stats.hp"), Some(&json!(5)));
    assert_eq!(get(&v, "items.1.id"), Some(&json!("b")));
    assert_eq!(get(&v, "items.9.id"), None);
    assert_eq!(get(&v, "items.x"), None);
    assert_eq!(get(&v, "player.missing"), None);
    assert_eq!(get(&v, ""), Some(&v));
}

#[test]
fn set_creates_missing_objects_and_reports_changes() {
    let mut v = json!({ "player": {} });
    assert!(set(&mut v, "player.stats.hp", json!(10)));
    assert_eq!(v, json!({ "player": { "stats": { "hp": 10 } } }));
    assert!(
        !set(&mut v, "player.stats.hp", json!(10)),
        "same value is no change"
    );
    assert!(set(&mut v, "player.stats.hp", json!(11)));
}

#[test]
fn set_into_arrays_needs_existing_index() {
    let mut v = json!({ "items": [ { "id": "a" } ] });
    assert!(set(&mut v, "items.0.id", json!("z")));
    assert!(!set(&mut v, "items.5.id", json!("z")));
    assert!(
        !set(&mut v, "items.0.id.deeper", json!(1)),
        "cannot descend into a string"
    );
    assert_eq!(v, json!({ "items": [ { "id": "z" } ] }));
}

#[test]
fn get_mut_and_join() {
    let mut v = json!({ "items": [1, 2] });
    if let Some(Value::Array(items)) = get_mut(&mut v, "items") {
        items.push(json!(3));
    }
    assert_eq!(v, json!({ "items": [1, 2, 3] }));
    assert_eq!(join("", "a"), "a");
    assert_eq!(join("a.0", "b"), "a.0.b");
}
```

`sdk/saveforge-sdk/tests/form_visibility.rs`:

```rust
//! Port of the .NET `VisibilityRuleTests`, plus missing values and loose
//! number/string comparison.

use saveforge_sdk::form::{VisibilityOp, VisibilityRule};
use serde_json::{Value, json};

fn rule(op: VisibilityOp, value: Value) -> VisibilityRule {
    VisibilityRule::new("x", op, value)
}

#[test]
fn equals_and_not_equals_strings() {
    assert!(rule(VisibilityOp::Equals, json!("admin")).evaluate(Some(&json!("admin"))));
    assert!(!rule(VisibilityOp::Equals, json!("admin")).evaluate(Some(&json!("user"))));
    assert!(rule(VisibilityOp::NotEquals, json!("admin")).evaluate(Some(&json!("user"))));
}

#[test]
fn greater_and_less_than_numbers() {
    assert!(rule(VisibilityOp::GreaterThan, json!(5)).evaluate(Some(&json!(10))));
    assert!(!rule(VisibilityOp::GreaterThan, json!(5)).evaluate(Some(&json!(5))));
    assert!(rule(VisibilityOp::LessThan, json!(10)).evaluate(Some(&json!(3))));
    assert!(rule(VisibilityOp::LessThan, json!(10.5)).evaluate(Some(&json!(10))));
}

#[test]
fn null_and_missing_values() {
    assert!(rule(VisibilityOp::Equals, Value::Null).evaluate(None));
    assert!(rule(VisibilityOp::Equals, Value::Null).evaluate(Some(&Value::Null)));
    assert!(!rule(VisibilityOp::Equals, json!(true)).evaluate(None));
    assert!(!rule(VisibilityOp::GreaterThan, json!(1)).evaluate(None));
    assert!(!rule(VisibilityOp::LessThan, json!(1)).evaluate(None));
}

#[test]
fn numeric_strings_compare_as_numbers() {
    assert!(rule(VisibilityOp::Equals, json!(5)).evaluate(Some(&json!("5"))));
    assert!(rule(VisibilityOp::GreaterThan, json!("9")).evaluate(Some(&json!(10))));
    assert!(rule(VisibilityOp::LessThan, json!("b")).evaluate(Some(&json!("a"))));
    assert!(!rule(VisibilityOp::GreaterThan, json!("b")).evaluate(Some(&json!(true))));
}

#[test]
fn booleans_match_exactly() {
    assert!(rule(VisibilityOp::Equals, json!(true)).evaluate(Some(&json!(true))));
    assert!(!rule(VisibilityOp::Equals, json!(true)).evaluate(Some(&json!(false))));
}
```

- [ ] **Step 4: Run them to verify they fail**

Run: `cargo test -p saveforge-sdk --test form_path --test form_visibility`
Expected: compile errors, unresolved `saveforge_sdk::form`.

- [ ] **Step 5: Write `sdk/saveforge-sdk/src/form/path.rs`**

```rust
//! Dot paths into a JSON value: `player.stats.health`, `inventory.0.id`.

use serde_json::{Map, Value};

fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split('.').filter(|s| !s.is_empty())
}

/// The value at `path`. Numeric segments index arrays.
pub fn get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    segments(path).try_fold(root, |value, segment| match value {
        Value::Object(map) => map.get(segment),
        Value::Array(items) => segment.parse::<usize>().ok().and_then(|i| items.get(i)),
        _ => None,
    })
}

/// Mutable access to the value at `path`.
pub fn get_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    segments(path).try_fold(root, |value, segment| match value {
        Value::Object(map) => map.get_mut(segment),
        Value::Array(items) => segment.parse::<usize>().ok().and_then(|i| items.get_mut(i)),
        _ => None,
    })
}

/// Sets the value at `path`, creating missing objects on the way. Array
/// segments must index an existing element. Returns whether anything changed.
pub fn set(root: &mut Value, path: &str, value: Value) -> bool {
    let parts: Vec<&str> = segments(path).collect();
    let Some((last, parents)) = parts.split_last() else {
        return false;
    };
    let mut current = root;
    for segment in parents {
        current = match current {
            Value::Object(map) => map
                .entry((*segment).to_owned())
                .or_insert_with(|| Value::Object(Map::new())),
            Value::Array(items) => {
                match segment.parse::<usize>().ok().and_then(|i| items.get_mut(i)) {
                    Some(item) => item,
                    None => return false,
                }
            }
            _ => return false,
        };
    }
    match current {
        Value::Object(map) => {
            if map.get(*last) == Some(&value) {
                return false;
            }
            map.insert((*last).to_owned(), value);
            true
        }
        Value::Array(items) => match last.parse::<usize>().ok().and_then(|i| items.get_mut(i)) {
            Some(slot) if *slot != value => {
                *slot = value;
                true
            }
            _ => false,
        },
        _ => false,
    }
}

/// `prefix.key`, or just `key` at the top level.
pub fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}
```

- [ ] **Step 6: Write `sdk/saveforge-sdk/src/form/visibility.rs`**

```rust
//! Conditional fields (port of the .NET `VisibilityRule`).

use std::cmp::Ordering;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityOp {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
}

/// Shows a field only while the value at `depends_on` (an absolute dot path)
/// satisfies `op value`.
#[derive(Debug, Clone, PartialEq)]
pub struct VisibilityRule {
    pub depends_on: String,
    pub op: VisibilityOp,
    pub value: Value,
}

impl VisibilityRule {
    pub fn new(depends_on: impl Into<String>, op: VisibilityOp, value: impl Into<Value>) -> Self {
        Self {
            depends_on: depends_on.into(),
            op,
            value: value.into(),
        }
    }

    /// `current` is the value at `depends_on`; `None` (missing) counts as null.
    /// Numbers compare numerically, also when one side is a numeric string;
    /// strings order lexically. Values that can't be ordered are neither
    /// greater nor less.
    pub fn evaluate(&self, current: Option<&Value>) -> bool {
        let current = current.unwrap_or(&Value::Null);
        match self.op {
            VisibilityOp::Equals => loose_eq(current, &self.value),
            VisibilityOp::NotEquals => !loose_eq(current, &self.value),
            VisibilityOp::GreaterThan => compare(current, &self.value) == Some(Ordering::Greater),
            VisibilityOp::LessThan => compare(current, &self.value) == Some(Ordering::Less),
        }
    }
}

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

fn loose_eq(a: &Value, b: &Value) -> bool {
    if a == b {
        return true;
    }
    match (as_number(a), as_number(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y) == Some(Ordering::Equal),
        _ => false,
    }
}

fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    if let (Some(x), Some(y)) = (as_number(a), as_number(b)) {
        return x.partial_cmp(&y);
    }
    match (a, b) {
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        _ => None,
    }
}
```

- [ ] **Step 7: Write the interim `sdk/saveforge-sdk/src/form/mod.rs` and register the module**

```rust
//! Schema-driven documents (spec §4.6): describe the editor declaratively and
//! let `FormDocument` render it and apply edits to a JSON value.

pub mod path;
mod visibility;

pub use visibility::{VisibilityOp, VisibilityRule};
```

(Task 2 replaces this file and turns `FormDocument` into a doc link.)

In `sdk/saveforge-sdk/src/lib.rs`, change the module list to:

```rust
mod adapter;
pub mod form;
mod log;
pub mod ui;
```

- [ ] **Step 8: Run tests and lints (native and wasm)**

Run: `cargo test -p saveforge-sdk --test form_path --test form_visibility`
Expected: form_path 4 passed, form_visibility 5 passed.

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p saveforge-sdk --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock sdk/saveforge-sdk
git commit -m "feat(sdk): dot-path access and visibility rules for forms"
```

---

### Task 2: Form schema and `FormDocument`

**Files:**
- Create: `sdk/saveforge-sdk/src/form/schema.rs`, `sdk/saveforge-sdk/src/form/document.rs`
- Modify: `sdk/saveforge-sdk/src/form/mod.rs` (final)
- Test: `sdk/saveforge-sdk/tests/form_document.rs`

**Interfaces:**
- Consumes: `path` and `VisibilityRule` (Task 1); `saveforge_sdk::{Document, ui}`.
- Produces, in `saveforge_sdk::form`:
  - **Schema:** `FormSchema { pages }` (`new`, `page`), `FormPage { title, sections }` (`new`, `section`), `FormSection { heading, fields }` (`new`, `heading`, `field`)
  - **Fields:** `FieldDef { key, label, help, read_only, required, visible_when, kind }` with builder methods `help`, `read_only`, `required`, `visible_when`, `max_len`, `pattern`, `range`, `min`, `max`, `decimals`
  - **Field kinds:** `FieldKind::{Text { max_len, pattern }, Number { min, max, decimals }, Checkbox, Choice { options }, List { item_label, fields }, Object { fields }}`
  - **Constructors:** `text`, `number`, `checkbox`, `choice(key, label, &[(&str, &str)])`, `list(key, label, item_label, Vec<FieldDef>)`, `object(key, label, Vec<FieldDef>)`
  - **Document:** `trait Codec { decode(&[u8]) -> Result<Value, String>; encode(&Value) -> Result<Vec<u8>, String> }`, `JsonCodec` (pretty JSON), and `FormDocument<C: Codec>` (`open(FormSchema, &[u8]) -> Result<Self, String>`, `value()`, `errors() -> Vec<String>`), which implements `Document`
  - **Constants:** `TABS_ID = "__tabs"`, `ADD_SUFFIX = ".__add"`
  - **Event mapping:**

    | Event | Effect |
    |---|---|
    | `TextChanged` / `Picked` | set a string |
    | `NumberChanged` | set a number per `decimals` |
    | `Toggled` | set a bool |
    | `Pressed("<list>.__add")` | append a default item |
    | `ListRemove` | remove the item |
    | `TabSelected` | switch page (not an edit) |

    Read-only and unknown ids are ignored. Setting the same value is not an edit.

- [ ] **Step 1: Write the failing test `sdk/saveforge-sdk/tests/form_document.rs`**

```rust
use saveforge_sdk::Document;
use saveforge_sdk::form::{
    ADD_SUFFIX, FormDocument, FormPage, FormSchema, FormSection, JsonCodec, TABS_ID, VisibilityOp,
    VisibilityRule, checkbox, choice, list, number, object, text,
};
use saveforge_sdk::ui::{Event, FieldSpec, Input, UiTree, Widget};
use serde_json::{Value, json};

type Doc = FormDocument<JsonCodec>;

fn schema() -> FormSchema {
    FormSchema::new()
        .page(
            FormPage::new("Player").section(
                FormSection::new()
                    .heading("Identity")
                    .field(text("player.name", "Name").required().max_len(10))
                    .field(text("player.code", "Code").pattern("[A-Z]{3}"))
                    .field(number("player.level", "Level").range(1.0, 100.0))
                    .field(number("player.speed", "Speed").decimals(2))
                    .field(checkbox("player.married", "Married"))
                    .field(text("player.spouse", "Spouse").required().visible_when(
                        VisibilityRule::new("player.married", VisibilityOp::Equals, true),
                    ))
                    .field(choice(
                        "player.farm",
                        "Farm",
                        &[("standard", "Standard"), ("forest", "Forest")],
                    ))
                    .field(text("player.id", "Id").read_only())
                    .field(object(
                        "player.stats",
                        "Stats",
                        vec![number("health", "Health").range(0.0, 100.0)],
                    )),
            ),
        )
        .page(
            FormPage::new("Inventory").section(FormSection::new().field(list(
                "inventory",
                "Items",
                "item",
                vec![
                    text("id", "Item ID").required(),
                    number("count", "Count").min(1.0),
                ],
            ))),
        )
}

fn sample() -> Value {
    json!({
        "player": {
            "name": "Robin",
            "code": "ABC",
            "level": 12,
            "speed": 1.5,
            "married": false,
            "farm": "forest",
            "id": "p-1",
            "stats": { "health": 80 }
        },
        "inventory": [ { "id": "seeds", "count": 15 } ],
        "unknown": { "kept": true }
    })
}

fn open(value: &Value) -> Doc {
    Doc::open(schema(), value.to_string().as_bytes()).unwrap()
}

fn field<'a>(tree: &'a UiTree, id: &str) -> Option<&'a FieldSpec> {
    tree.nodes.iter().find_map(|n| match &n.widget {
        Widget::Field(f) if f.id == id => Some(f),
        _ => None,
    })
}

fn has_button(tree: &UiTree, id: &str) -> bool {
    tree.nodes
        .iter()
        .any(|n| matches!(&n.widget, Widget::Button(b) if b.id == id))
}

fn edit(doc: &mut Doc, event: Event) {
    doc.update(event);
}

fn text_event(id: &str, value: &str) -> Event {
    Event::TextChanged((id.to_owned(), value.to_owned()))
}

#[test]
fn invalid_json_is_rejected() {
    let err = Doc::open(schema(), b"{ nope").unwrap_err();
    assert!(err.contains("not valid JSON"), "{err}");
}

#[test]
fn multi_page_schema_renders_tabs_with_the_selected_page() {
    let mut doc = open(&sample());
    let tree = doc.view();
    assert!(matches!(&tree.nodes[0].widget, Widget::Tabs(t) if t.id == TABS_ID && t.selected == 0));
    assert!(field(&tree, "player.name").is_some());
    assert!(field(&tree, "inventory.0.id").is_none());

    edit(&mut doc, Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    assert!(field(&tree, "inventory.0.id").is_some());
    assert!(field(&tree, "player.name").is_none());
    assert!(!doc.is_dirty(), "switching tabs is not an edit");
}

#[test]
fn fields_show_current_values() {
    let tree = open(&sample()).view();
    let name = field(&tree, "player.name").unwrap();
    assert!(matches!(&name.input, Input::Text(t) if t.value == "Robin" && t.max_len == Some(10)));
    let level = field(&tree, "player.level").unwrap();
    assert!(
        matches!(&level.input, Input::Number(n) if (n.value - 12.0).abs() < f64::EPSILON && n.min == Some(1.0))
    );
    let married = field(&tree, "player.married").unwrap();
    assert_eq!(married.input, Input::Checkbox(false));
    let farm = field(&tree, "player.farm").unwrap();
    assert!(matches!(&farm.input, Input::Pick(p) if p.selected.as_deref() == Some("forest")));
    assert!(field(&tree, "player.id").unwrap().read_only);
    assert!(field(&tree, "player.stats.health").is_some());
}

#[test]
fn edits_update_value_and_dirty() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", "Abigail"));
    edit(
        &mut doc,
        Event::NumberChanged(("player.level".to_owned(), 30.0)),
    );
    edit(
        &mut doc,
        Event::NumberChanged(("player.speed".to_owned(), 2.25)),
    );
    edit(
        &mut doc,
        Event::Toggled(("player.married".to_owned(), true)),
    );
    edit(
        &mut doc,
        Event::Picked(("player.farm".to_owned(), "standard".to_owned())),
    );
    edit(
        &mut doc,
        Event::NumberChanged(("player.stats.health".to_owned(), 90.0)),
    );
    let v = doc.value();
    assert_eq!(v["player"]["name"], json!("Abigail"));
    assert_eq!(
        v["player"]["level"],
        json!(30),
        "whole numbers stay integers"
    );
    assert_eq!(v["player"]["speed"], json!(2.25));
    assert_eq!(v["player"]["married"], json!(true));
    assert_eq!(v["player"]["farm"], json!("standard"));
    assert_eq!(v["player"]["stats"]["health"], json!(90));
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());
}

#[test]
fn read_only_and_unknown_fields_ignore_edits() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.id", "hacked"));
    edit(&mut doc, text_event("not.in.schema", "x"));
    edit(&mut doc, text_event("unknown.kept", "x"));
    assert_eq!(doc.value(), &sample());
    assert!(!doc.is_dirty());
}

#[test]
fn same_value_is_not_an_edit() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", "Robin"));
    assert!(!doc.is_dirty());
}

#[test]
fn list_items_add_and_remove() {
    let mut doc = open(&sample());
    edit(&mut doc, Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    assert!(has_button(&tree, &format!("inventory{ADD_SUFFIX}")));

    edit(&mut doc, Event::Pressed(format!("inventory{ADD_SUFFIX}")));
    assert_eq!(
        doc.value()["inventory"],
        json!([ { "id": "seeds", "count": 15 }, { "id": "", "count": 1 } ])
    );
    edit(&mut doc, text_event("inventory.1.id", "ore"));
    assert_eq!(doc.value()["inventory"][1]["id"], json!("ore"));

    edit(&mut doc, Event::ListRemove(("inventory".to_owned(), 0)));
    assert_eq!(
        doc.value()["inventory"],
        json!([ { "id": "ore", "count": 1 } ])
    );
    edit(&mut doc, Event::ListRemove(("inventory".to_owned(), 7)));
    assert_eq!(doc.value()["inventory"].as_array().unwrap().len(), 1);
    assert!(doc.is_dirty());
}

#[test]
fn adding_to_a_missing_list_creates_it() {
    let mut value = sample();
    value.as_object_mut().unwrap().remove("inventory");
    let mut doc = open(&value);
    edit(&mut doc, Event::Pressed(format!("inventory{ADD_SUFFIX}")));
    assert_eq!(
        doc.value()["inventory"],
        json!([ { "id": "", "count": 1 } ])
    );
}

#[test]
fn hidden_fields_are_not_rendered_or_validated() {
    let mut doc = open(&sample());
    assert!(field(&doc.view(), "player.spouse").is_none());
    assert_eq!(doc.errors(), Vec::<String>::new());

    edit(
        &mut doc,
        Event::Toggled(("player.married".to_owned(), true)),
    );
    let tree = doc.view();
    let spouse = field(&tree, "player.spouse").expect("shown once married");
    assert_eq!(spouse.error.as_deref(), Some("Spouse is required."));
    assert_eq!(doc.errors(), vec!["Spouse is required.".to_owned()]);
}

#[test]
fn validation_messages() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", ""));
    edit(&mut doc, text_event("player.code", "abcd"));
    edit(
        &mut doc,
        Event::NumberChanged(("player.level".to_owned(), 500.0)),
    );
    assert_eq!(
        doc.errors(),
        vec![
            "Name is required.".to_owned(),
            "Code has an invalid format.".to_owned(),
            "Level must be between 1 and 100.".to_owned(),
        ]
    );
    edit(&mut doc, text_event("player.name", "Abcdefghijkl"));
    assert_eq!(doc.errors()[0], "Name must not exceed 10 characters.");

    let tree = doc.view();
    assert_eq!(
        field(&tree, "player.code").unwrap().error.as_deref(),
        Some("Code has an invalid format.")
    );
}

#[test]
fn invalid_document_cannot_be_serialized() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", ""));
    let err = doc.serialize().unwrap_err();
    assert!(
        err.starts_with("Fix 1 invalid field(s) before saving"),
        "{err}"
    );
    assert!(err.contains("Name is required."), "{err}");
}

#[test]
fn serialize_keeps_unknown_keys_and_key_order() {
    let original = r#"{"zeta":1,"player":{"name":"Robin","code":"ABC","level":12,"speed":1.5,"married":false,"farm":"forest","id":"p-1","stats":{"health":80}},"inventory":[],"alpha":{"kept":true}}"#;
    let mut doc = Doc::open(schema(), original.as_bytes()).unwrap();
    edit(&mut doc, text_event("player.name", "Abigail"));
    let bytes = doc.serialize().unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let reparsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(reparsed["player"]["name"], json!("Abigail"));
    assert_eq!(reparsed["alpha"], json!({ "kept": true }));
    let zeta = text.find("\"zeta\"").unwrap();
    let alpha = text.find("\"alpha\"").unwrap();
    assert!(zeta < alpha, "original key order is kept:\n{text}");
    assert!(text.contains('\n'), "written pretty-printed");
}

#[test]
fn numbers_stored_as_strings_show_their_value() {
    let mut value = sample();
    value["player"]["level"] = json!("42");
    let tree = open(&value).view();
    let level = field(&tree, "player.level").unwrap();
    assert!(matches!(&level.input, Input::Number(n) if (n.value - 42.0).abs() < f64::EPSILON));
}

#[test]
fn non_object_list_items_render_and_ignore_edits() {
    let mut value = sample();
    value["inventory"] = json!(["seeds", 7]);
    let mut doc = open(&value);
    edit(&mut doc, Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    assert!(field(&tree, "inventory.0.id").is_some());
    edit(&mut doc, text_event("inventory.0.id", "x"));
    assert_eq!(doc.value()["inventory"], json!(["seeds", 7]));
}

#[test]
fn max_len_counts_characters_not_bytes() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", "Žluťoučký"));
    assert_eq!(doc.errors(), Vec::<String>::new());
}

#[test]
fn malformed_pattern_in_schema_is_ignored() {
    let schema = FormSchema::new().page(
        FormPage::new("P").section(FormSection::new().field(text("code", "Code").pattern("["))),
    );
    let doc = Doc::open(schema, br#"{"code":"anything"}"#).unwrap();
    assert_eq!(doc.errors(), Vec::<String>::new());
}

#[test]
fn root_must_be_a_json_object() {
    let err = Doc::open(schema(), b"[1, 2]").unwrap_err();
    assert_eq!(err, "the save is not a JSON object");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p saveforge-sdk --test form_document`
Expected: compile errors, unresolved `FormDocument`, `text`, `TABS_ID`, …

- [ ] **Step 3: Write `sdk/saveforge-sdk/src/form/schema.rs`**

```rust
//! Declarative form schema (port of the .NET `SaveForge.SDK.Schema`).
//!
//! Field `key`s are dot paths relative to the enclosing object: top-level
//! fields are relative to the document root, fields inside an `object` to the
//! object, fields of a `list` item to the item.

use super::visibility::VisibilityRule;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormSchema {
    pub pages: Vec<FormPage>,
}

impl FormSchema {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn page(mut self, page: FormPage) -> Self {
        self.pages.push(page);
        self
    }
}

/// One tab of the editor.
#[derive(Debug, Clone, PartialEq)]
pub struct FormPage {
    pub title: String,
    pub sections: Vec<FormSection>,
}

impl FormPage {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            sections: Vec::new(),
        }
    }

    #[must_use]
    pub fn section(mut self, section: FormSection) -> Self {
        self.sections.push(section);
        self
    }
}

/// A group of fields, optionally under a heading.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormSection {
    pub heading: Option<String>,
    pub fields: Vec<FieldDef>,
}

impl FormSection {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn heading(mut self, heading: impl Into<String>) -> Self {
        self.heading = Some(heading.into());
        self
    }

    #[must_use]
    pub fn field(mut self, field: FieldDef) -> Self {
        self.fields.push(field);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text {
        max_len: Option<u32>,
        /// Regular expression the whole value must match (`regex-lite` syntax).
        pattern: Option<String>,
    },
    Number {
        min: Option<f64>,
        max: Option<f64>,
        /// 0 stores whole numbers as JSON integers.
        decimals: u8,
    },
    Checkbox,
    /// A fixed set of string values: `(value, display)`.
    Choice {
        options: Vec<(String, String)>,
    },
    /// A JSON array of objects, each edited with `fields`.
    List {
        item_label: String,
        fields: Vec<FieldDef>,
    },
    /// A nested JSON object shown as a titled group.
    Object {
        fields: Vec<FieldDef>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    pub help: Option<String>,
    pub read_only: bool,
    pub required: bool,
    pub visible_when: Option<VisibilityRule>,
    pub kind: FieldKind,
}

fn field(key: impl Into<String>, label: impl Into<String>, kind: FieldKind) -> FieldDef {
    FieldDef {
        key: key.into(),
        label: label.into(),
        help: None,
        read_only: false,
        required: false,
        visible_when: None,
        kind,
    }
}

pub fn text(key: impl Into<String>, label: impl Into<String>) -> FieldDef {
    field(
        key,
        label,
        FieldKind::Text {
            max_len: None,
            pattern: None,
        },
    )
}

pub fn number(key: impl Into<String>, label: impl Into<String>) -> FieldDef {
    field(
        key,
        label,
        FieldKind::Number {
            min: None,
            max: None,
            decimals: 0,
        },
    )
}

pub fn checkbox(key: impl Into<String>, label: impl Into<String>) -> FieldDef {
    field(key, label, FieldKind::Checkbox)
}

pub fn choice(
    key: impl Into<String>,
    label: impl Into<String>,
    options: &[(&str, &str)],
) -> FieldDef {
    field(
        key,
        label,
        FieldKind::Choice {
            options: options
                .iter()
                .map(|(v, d)| ((*v).to_owned(), (*d).to_owned()))
                .collect(),
        },
    )
}

pub fn list(
    key: impl Into<String>,
    label: impl Into<String>,
    item_label: impl Into<String>,
    fields: Vec<FieldDef>,
) -> FieldDef {
    field(
        key,
        label,
        FieldKind::List {
            item_label: item_label.into(),
            fields,
        },
    )
}

pub fn object(key: impl Into<String>, label: impl Into<String>, fields: Vec<FieldDef>) -> FieldDef {
    field(key, label, FieldKind::Object { fields })
}

impl FieldDef {
    #[must_use]
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    #[must_use]
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    #[must_use]
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    #[must_use]
    pub fn visible_when(mut self, rule: VisibilityRule) -> Self {
        self.visible_when = Some(rule);
        self
    }

    /// Text fields only.
    #[must_use]
    pub fn max_len(mut self, max: u32) -> Self {
        if let FieldKind::Text { max_len, .. } = &mut self.kind {
            *max_len = Some(max);
        }
        self
    }

    /// Text fields only.
    #[must_use]
    pub fn pattern(mut self, regex: impl Into<String>) -> Self {
        if let FieldKind::Text { pattern, .. } = &mut self.kind {
            *pattern = Some(regex.into());
        }
        self
    }

    /// Number fields only.
    #[must_use]
    pub fn range(self, min: f64, max: f64) -> Self {
        self.min(min).max(max)
    }

    /// Number fields only.
    #[must_use]
    pub fn min(mut self, value: f64) -> Self {
        if let FieldKind::Number { min, .. } = &mut self.kind {
            *min = Some(value);
        }
        self
    }

    /// Number fields only.
    #[must_use]
    pub fn max(mut self, value: f64) -> Self {
        if let FieldKind::Number { max, .. } = &mut self.kind {
            *max = Some(value);
        }
        self
    }

    /// Number fields only.
    #[must_use]
    pub fn decimals(mut self, places: u8) -> Self {
        if let FieldKind::Number { decimals, .. } = &mut self.kind {
            *decimals = places;
        }
        self
    }
}
```

- [ ] **Step 4: Write `sdk/saveforge-sdk/src/form/document.rs`**

```rust
//! A `Document` driven by a `FormSchema` over a JSON value (spec §4.6).

use std::fmt;
use std::marker::PhantomData;

use serde_json::{Map, Value};

use super::path;
use super::schema::{FieldDef, FieldKind, FormSchema};
use crate::Document;
use crate::ui::{self, El, Event, UiTree};

/// Widget id of the page tabs. Field ids are dot paths, which never start
/// with `__`.
pub const TABS_ID: &str = "__tabs";
/// Suffix of a list's "Add" button id: `inventory.__add`.
pub const ADD_SUFFIX: &str = ".__add";

/// Converts between file bytes and the JSON value the form edits.
pub trait Codec: 'static {
    fn decode(bytes: &[u8]) -> Result<Value, String>;
    fn encode(value: &Value) -> Result<Vec<u8>, String>;
}

/// Plain JSON files, written back pretty-printed with keys in their
/// original order.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonCodec;

impl Codec for JsonCodec {
    fn decode(bytes: &[u8]) -> Result<Value, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("not valid JSON: {e}"))
    }

    fn encode(value: &Value) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())
    }
}

/// Edits a decoded save through a schema. Unknown keys in the file are kept.
pub struct FormDocument<C: Codec> {
    schema: FormSchema,
    value: Value,
    tab: u32,
    dirty: bool,
    codec: PhantomData<fn() -> C>,
}

impl<C: Codec> fmt::Debug for FormDocument<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormDocument")
            .field("tab", &self.tab)
            .field("dirty", &self.dirty)
            .finish_non_exhaustive()
    }
}

impl<C: Codec> FormDocument<C> {
    /// Decodes `bytes`; the root must be a JSON object.
    pub fn open(schema: FormSchema, bytes: &[u8]) -> Result<Self, String> {
        let value = C::decode(bytes)?;
        if !value.is_object() {
            return Err("the save is not a JSON object".to_owned());
        }
        Ok(Self {
            schema,
            value,
            tab: 0,
            dirty: false,
            codec: PhantomData,
        })
    }

    /// The document's current JSON value.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Validation messages for every visible field, in schema order.
    pub fn errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for page in &self.schema.pages {
            for section in &page.sections {
                self.collect_errors(&section.fields, "", &mut errors);
            }
        }
        errors
    }

    fn collect_errors(&self, fields: &[FieldDef], prefix: &str, errors: &mut Vec<String>) {
        for def in fields.iter().filter(|d| self.is_visible(d)) {
            let at = path::join(prefix, &def.key);
            match &def.kind {
                FieldKind::Object { fields } => self.collect_errors(fields, &at, errors),
                FieldKind::List { fields, .. } => {
                    for i in 0..self.list_len(&at) {
                        self.collect_errors(fields, &path::join(&at, &i.to_string()), errors);
                    }
                }
                _ => errors.extend(validate(def, path::get(&self.value, &at))),
            }
        }
    }

    fn is_visible(&self, def: &FieldDef) -> bool {
        def.visible_when
            .as_ref()
            .is_none_or(|rule| rule.evaluate(path::get(&self.value, &rule.depends_on)))
    }

    fn list_len(&self, at: &str) -> usize {
        path::get(&self.value, at)
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    }

    fn page_view(&self, page_index: usize) -> El {
        let Some(page) = self.schema.pages.get(page_index) else {
            return ui::column(Vec::new());
        };
        ui::column(
            page.sections
                .iter()
                .map(|section| {
                    let fields = self.fields_view(&section.fields, "");
                    match &section.heading {
                        Some(heading) => ui::section(heading.clone(), fields),
                        None => ui::column(fields),
                    }
                })
                .collect(),
        )
    }

    fn fields_view(&self, fields: &[FieldDef], prefix: &str) -> Vec<El> {
        fields
            .iter()
            .filter(|def| self.is_visible(def))
            .map(|def| self.field_view(def, &path::join(prefix, &def.key)))
            .collect()
    }

    fn field_view(&self, def: &FieldDef, at: &str) -> El {
        let current = path::get(&self.value, at);
        let error = validate(def, current);
        let field = match &def.kind {
            FieldKind::Text { max_len, .. } => {
                let mut input = ui::text_input(at, def.label.clone(), display_text(current));
                if let Some(max) = max_len {
                    input = input.max_len(*max);
                }
                input
            }
            FieldKind::Number { min, max, decimals } => {
                let mut input =
                    ui::number(at, def.label.clone(), number_of(current).unwrap_or(0.0))
                        .decimals(*decimals);
                if let Some(min) = min {
                    input = input.min(*min);
                }
                if let Some(max) = max {
                    input = input.max(*max);
                }
                input
            }
            FieldKind::Checkbox => ui::checkbox(
                at,
                def.label.clone(),
                current.and_then(Value::as_bool).unwrap_or(false),
            ),
            FieldKind::Choice { options } => ui::pick(
                at,
                def.label.clone(),
                options
                    .iter()
                    .map(|(v, d)| ui::option(v.clone(), d.clone()))
                    .collect(),
                current.and_then(Value::as_str).map(str::to_owned),
            ),
            FieldKind::Object { fields } => {
                return ui::section(def.label.clone(), self.fields_view(fields, at));
            }
            FieldKind::List { item_label, fields } => {
                return self.list_view(def, at, item_label, fields);
            }
        };
        let mut field = field.read_only(def.read_only);
        if let Some(help) = &def.help {
            field = field.help(help.clone());
        }
        if let Some(error) = error {
            field = field.error(error);
        }
        field.into()
    }

    fn list_view(&self, def: &FieldDef, at: &str, item_label: &str, fields: &[FieldDef]) -> El {
        let rows = (0..self.list_len(at))
            .map(|i| ui::column(self.fields_view(fields, &path::join(at, &i.to_string()))))
            .collect();
        let mut children = vec![
            ui::text(def.label.clone()),
            ui::list(at, rows).removable(!def.read_only).into(),
        ];
        if !def.read_only {
            children
                .push(ui::button(format!("{at}{ADD_SUFFIX}"), format!("Add {item_label}")).into());
        }
        ui::column(children)
    }

    /// Sets the field at `id` if the schema has an editable field there.
    fn set_field(&mut self, id: &str, value: Value) -> bool {
        match find_field(&self.schema, id) {
            Some(def) if !def.read_only => path::set(&mut self.value, id, value),
            _ => false,
        }
    }

    fn set_number(&mut self, id: &str, value: f64) -> bool {
        match find_field(&self.schema, id) {
            Some(FieldDef {
                kind: FieldKind::Number { decimals, .. },
                read_only: false,
                ..
            }) => {
                let json = number_value(value, *decimals);
                path::set(&mut self.value, id, json)
            }
            _ => false,
        }
    }

    fn add_item(&mut self, list_id: &str) -> bool {
        let Some(FieldDef {
            kind: FieldKind::List { fields, .. },
            read_only: false,
            ..
        }) = find_field(&self.schema, list_id)
        else {
            return false;
        };
        let item = default_item(fields);
        match path::get_mut(&mut self.value, list_id) {
            Some(Value::Array(items)) => {
                items.push(item);
                true
            }
            Some(_) => false,
            None => path::set(&mut self.value, list_id, Value::Array(vec![item])),
        }
    }

    fn remove_item(&mut self, list_id: &str, index: u32) -> bool {
        let editable = matches!(
            find_field(&self.schema, list_id),
            Some(FieldDef {
                kind: FieldKind::List { .. },
                read_only: false,
                ..
            })
        );
        match path::get_mut(&mut self.value, list_id) {
            Some(Value::Array(items)) if editable && (index as usize) < items.len() => {
                items.remove(index as usize);
                true
            }
            _ => false,
        }
    }
}

impl<C: Codec> Document for FormDocument<C> {
    fn view(&self) -> UiTree {
        let pages = &self.schema.pages;
        let last = u32::try_from(pages.len().saturating_sub(1)).unwrap_or(u32::MAX);
        let selected = self.tab.min(last);
        let page = self.page_view(selected as usize);
        if pages.len() > 1 {
            let titles = pages.iter().map(|p| p.title.clone()).collect();
            ui::tabs(TABS_ID, titles, selected, page).into_tree()
        } else {
            page.into_tree()
        }
    }

    fn update(&mut self, event: Event) {
        let changed = match event {
            Event::TabSelected((_, index)) => {
                self.tab = index;
                false
            }
            Event::TextChanged((id, value)) | Event::Picked((id, value)) => {
                self.set_field(&id, Value::String(value))
            }
            Event::NumberChanged((id, value)) => self.set_number(&id, value),
            Event::Toggled((id, value)) => self.set_field(&id, Value::Bool(value)),
            Event::Pressed(id) => id
                .strip_suffix(ADD_SUFFIX)
                .is_some_and(|list| self.add_item(list)),
            Event::ListRemove((id, index)) => self.remove_item(&id, index),
            Event::ListAdd(_) => false,
        };
        self.dirty |= changed;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        let errors = self.errors();
        if !errors.is_empty() {
            return Err(format!(
                "Fix {} invalid field(s) before saving: {}",
                errors.len(),
                errors.join(" ")
            ));
        }
        C::encode(&self.value)
    }

    fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

/// The schema field a widget id (dot path, numeric segments for list items)
/// refers to.
fn find_field<'a>(schema: &'a FormSchema, id: &str) -> Option<&'a FieldDef> {
    schema
        .pages
        .iter()
        .flat_map(|page| &page.sections)
        .find_map(|section| find_in(&section.fields, "", id))
}

fn find_in<'a>(fields: &'a [FieldDef], prefix: &str, id: &str) -> Option<&'a FieldDef> {
    fields.iter().find_map(|def| {
        let at = path::join(prefix, &def.key);
        if at == id {
            return Some(def);
        }
        let rest = id.strip_prefix(&at)?.strip_prefix('.')?;
        match &def.kind {
            FieldKind::Object { fields } => find_in(fields, &at, id),
            FieldKind::List { fields, .. } => {
                let index = rest.split('.').next()?;
                index.parse::<usize>().ok()?;
                find_in(fields, &path::join(&at, index), id)
            }
            _ => None,
        }
    })
}

/// The message to show under a field, if its current value is invalid.
fn validate(def: &FieldDef, current: Option<&Value>) -> Option<String> {
    let label = &def.label;
    match &def.kind {
        FieldKind::Text { max_len, pattern } => {
            let value = display_text(current);
            if def.required && value.trim().is_empty() {
                return Some(format!("{label} is required."));
            }
            if let Some(max) = max_len
                && value.chars().count() > *max as usize
            {
                return Some(format!("{label} must not exceed {max} characters."));
            }
            // A malformed pattern is the plugin author's bug; it must not make
            // every value invalid, so it is ignored.
            if let Some(pattern) = pattern
                && !value.is_empty()
                && regex_lite::Regex::new(&format!("^(?:{pattern})$"))
                    .is_ok_and(|re| !re.is_match(&value))
            {
                return Some(format!("{label} has an invalid format."));
            }
            None
        }
        FieldKind::Number { min, max, .. } => {
            let value = number_of(current)?;
            match (min, max) {
                (Some(min), Some(max)) if value < *min || value > *max => {
                    Some(format!("{label} must be between {min} and {max}."))
                }
                (Some(min), None) if value < *min => {
                    Some(format!("{label} must be at least {min}."))
                }
                (None, Some(max)) if value > *max => {
                    Some(format!("{label} must be at most {max}."))
                }
                _ => None,
            }
        }
        FieldKind::Choice { .. } => (def.required
            && current.and_then(Value::as_str).is_none_or(str::is_empty))
        .then(|| format!("{label} is required.")),
        FieldKind::Checkbox | FieldKind::List { .. } | FieldKind::Object { .. } => None,
    }
}

/// A number field's value: JSON numbers, or strings holding a number.
fn number_of(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// How a JSON value shows in a text field: strings as-is, null or missing
/// as empty, anything else in JSON form.
fn display_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// A JSON number for `value`: an integer when `decimals` is 0 and the value is
/// whole, so `12` stays `12` rather than becoming `12.0`.
fn number_value(value: f64, decimals: u8) -> Value {
    // Exact integers up to 2^53 round-trip through f64.
    const MAX_EXACT: f64 = 9_007_199_254_740_992.0;
    if decimals == 0 && value.fract() == 0.0 && value.abs() <= MAX_EXACT {
        #[allow(clippy::cast_possible_truncation)] // whole and within ±2^53
        return Value::from(value as i64);
    }
    serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
}

/// A new list item: empty text, the lowest allowed number (or 0), unchecked,
/// the first choice, empty lists and objects.
fn default_item(fields: &[FieldDef]) -> Value {
    let mut item = Value::Object(Map::new());
    for def in fields {
        let value = match &def.kind {
            FieldKind::Text { .. } => Value::String(String::new()),
            FieldKind::Number { min, decimals, .. } => {
                number_value(min.unwrap_or(0.0).max(0.0), *decimals)
            }
            FieldKind::Checkbox => Value::Bool(false),
            FieldKind::Choice { options } => {
                Value::String(options.first().map(|(v, _)| v.clone()).unwrap_or_default())
            }
            FieldKind::List { .. } => Value::Array(Vec::new()),
            FieldKind::Object { fields } => default_item(fields),
        };
        path::set(&mut item, &def.key, value);
    }
    item
}
```

- [ ] **Step 5: Replace `sdk/saveforge-sdk/src/form/mod.rs` with the final version**

```rust
//! Schema-driven documents (spec §4.6): describe the editor declaratively and
//! let [`FormDocument`] render it and apply edits to a JSON value.

mod document;
pub mod path;
mod schema;
mod visibility;

pub use document::{ADD_SUFFIX, Codec, FormDocument, JsonCodec, TABS_ID};
pub use schema::{
    FieldDef, FieldKind, FormPage, FormSchema, FormSection, checkbox, choice, list, number, object,
    text,
};
pub use visibility::{VisibilityOp, VisibilityRule};
```

- [ ] **Step 6: Run tests and lints (native and wasm)**

Run: `cargo test -p saveforge-sdk`
Expected: export 2, form_document 17, form_path 4, form_visibility 5, ui 6, doctest 1, all passing.

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p saveforge-sdk --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add sdk/saveforge-sdk
git commit -m "feat(sdk): schema-driven FormDocument with validation and lists"
```

---

### Task 3: StardewLike sample plugin

**Files:**
- Modify: `Cargo.toml` (member `plugins/sample-stardew`), `.github/workflows/ci.yml` (wasm clippy)
- Create: `plugins/sample-stardew/Cargo.toml`, `plugins/sample-stardew/manifest.json`, `plugins/sample-stardew/src/lib.rs`, `docs/sample-saves/StardewLike-Farm.json`
- Test: `plugins/sample-stardew/tests/stardew.rs`

**Interfaces:**
- Consumes: `saveforge_sdk::form::*` (Task 2), `saveforge_sdk::{Plugin, AuxFile, export_plugin!}`.
- Produces:
  - crate `sample_stardew` with `Stardew` (implements `Plugin`), `StardewDocument = FormDocument<JsonCodec>`, `PRIORITY = 5` and `schema() -> FormSchema`
  - the component `target/dev-plugins/stardew-like/0.1.0/plugin.wasm` after `cargo xtask plugins`
  - the sample save `docs/sample-saves/StardewLike-Farm.json`
  - **Schema:**
    - Page "Player", section Identity: Name (required, max 50, help), Level (1–100), Money (≥0), Farm type (choice), Married, Spouse (required, visible when Married).
    - Page "Player", Stats object: Health and Energy (0–100).
    - Page "Inventory": list of items with Item ID (required) and Count (≥0).

- [ ] **Step 1: Add the member and write the crate files**

In the workspace `Cargo.toml` `members`, add `"plugins/sample-stardew",` after `"fixtures/test-plugin",`.

`plugins/sample-stardew/Cargo.toml`:

```toml
[package]
name = "sample-stardew"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
saveforge-sdk.workspace = true

[dev-dependencies]
serde_json.workspace = true

[lints]
workspace = true
```

`plugins/sample-stardew/manifest.json`:

```json
{
  "id": "stardew-like",
  "name": "StardewLike Save Editor",
  "version": "0.1.0",
  "author": "SaveForge Team",
  "description": "Reference plugin for the schema-driven form helper. Edits saves of the StardewLike sample game.",
  "apiVersion": "0.1",
  "minHostVersion": "0.1.0",
  "maxHostVersion": null,
  "component": "plugin.wasm",
  "fileExtensions": [".json"],
  "defaultSaveDirectories": {
    "windows": "%APPDATA%/StardewValley/Saves",
    "linux": "~/.config/StardewValley/Saves",
    "macos": "~/.config/StardewValley/Saves"
  }
}
```

`docs/sample-saves/StardewLike-Farm.json`:

```json
{
  "player": {
    "name": "Robin",
    "level": 12,
    "money": 2500,
    "married": false,
    "spouse": "",
    "stats": {
      "health": 85,
      "energy": 100
    }
  },
  "farm": {
    "name": "Sunny Acres",
    "type": "riverland"
  },
  "inventory": [
    {
      "id": "parsnip_seeds",
      "count": 15
    },
    {
      "id": "copper_ore",
      "count": 42
    }
  ],
  "playtime_minutes": 1834
}
```

- [ ] **Step 2: Write the failing test `plugins/sample-stardew/tests/stardew.rs`**

```rust
//! Native tests of the `StardewLike` plugin against the sample save in
//! `docs/sample-saves`.

use sample_stardew::{PRIORITY, Stardew, schema};
use saveforge_sdk::form::TABS_ID;
use saveforge_sdk::ui::{Event, Widget};
use saveforge_sdk::{Document, Plugin};
use serde_json::{Value, json};

fn sample_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/sample-saves/StardewLike-Farm.json"
    ))
    .unwrap()
}

fn labels(doc: &impl Document) -> Vec<String> {
    doc.view()
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Field(f) => Some(f.label.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn detects_player_saves_only() {
    let header = sample_bytes();
    assert_eq!(
        Stardew::detect("Farm.json", &header[..header.len().min(512)]),
        Some(PRIORITY)
    );
    assert_eq!(Stardew::detect("other.json", br#"{"terraTokens":1}"#), None);
}

#[test]
fn schema_has_player_and_inventory_pages() {
    let titles: Vec<_> = schema().pages.iter().map(|p| p.title.clone()).collect();
    assert_eq!(titles, ["Player", "Inventory"]);
}

#[test]
fn opens_sample_save_and_shows_player_fields() {
    let doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    assert_eq!(
        labels(&doc),
        [
            "Name",
            "Level",
            "Money",
            "Farm type",
            "Married",
            "Health",
            "Energy"
        ]
    );
}

#[test]
fn spouse_appears_when_married() {
    let mut doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    doc.update(Event::Toggled(("player.married".to_owned(), true)));
    assert!(labels(&doc).contains(&"Spouse".to_owned()));
    // Married without a spouse name is invalid and blocks saving.
    assert!(doc.serialize().is_err());
    doc.update(Event::TextChanged((
        "player.spouse".to_owned(),
        "Abigail".to_owned(),
    )));
    assert!(doc.serialize().is_ok());
}

#[test]
fn inventory_page_lists_items() {
    let mut doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    doc.update(Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    let ids: Vec<_> = tree
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Field(f) => Some(f.id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        ids,
        [
            "inventory.0.id",
            "inventory.0.count",
            "inventory.1.id",
            "inventory.1.count"
        ]
    );
}

#[test]
fn edits_round_trip_and_keep_unknown_data() {
    let mut doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    doc.update(Event::NumberChanged(("player.money".to_owned(), 99_999.0)));
    let saved: Value = serde_json::from_slice(&doc.serialize().unwrap()).unwrap();
    assert_eq!(saved["player"]["money"], json!(99_999));
    assert_eq!(saved["farm"]["name"], json!("Sunny Acres"));
    assert_eq!(saved["playtime_minutes"], json!(1834));
    let original: Value = serde_json::from_slice(&sample_bytes()).unwrap();
    let mut expected = original;
    expected["player"]["money"] = json!(99_999);
    assert_eq!(saved, expected, "nothing else changed");
}
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test -p sample-stardew`
Expected: compile error. `src/lib.rs` is missing, or `sample_stardew` is unresolved.

- [ ] **Step 4: Write `plugins/sample-stardew/src/lib.rs`**

```rust
//! `StardewLike` sample plugin: the reference for the schema-driven pattern
//! (spec §6.2). Everything game-specific is the schema below.

use saveforge_sdk::form::{
    FormDocument, FormPage, FormSchema, FormSection, JsonCodec, VisibilityOp, VisibilityRule,
    checkbox, choice, list, number, object, text,
};
use saveforge_sdk::{AuxFile, Plugin, export_plugin};

#[derive(Debug)]
pub struct Stardew;

pub type StardewDocument = FormDocument<JsonCodec>;

/// Detection priority; Planet Crafter's more specific detector uses 20.
pub const PRIORITY: u32 = 5;

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

/// Port of the .NET `StardewSchema`, plus a conditional spouse field showing
/// a `VisibilityRule`.
pub fn schema() -> FormSchema {
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
                        .field(number("player.money", "Money").min(0.0))
                        .field(choice(
                            "farm.type",
                            "Farm type",
                            &[
                                ("standard", "Standard"),
                                ("riverland", "Riverland"),
                                ("forest", "Forest"),
                            ],
                        ))
                        .field(checkbox("player.married", "Married"))
                        .field(text("player.spouse", "Spouse").required().visible_when(
                            VisibilityRule::new("player.married", VisibilityOp::Equals, true),
                        )),
                )
                .section(FormSection::new().field(object(
                    "player.stats",
                    "Stats",
                    vec![
                        number("health", "Health").range(0.0, 100.0),
                        number("energy", "Energy").range(0.0, 100.0),
                    ],
                ))),
        )
        .page(
            FormPage::new("Inventory").section(FormSection::new().field(list(
                "inventory",
                "Items",
                "item",
                vec![
                    text("id", "Item ID").required(),
                    number("count", "Count").min(0.0),
                ],
            ))),
        )
}

export_plugin!(Stardew);
```

- [ ] **Step 5: Run tests, build the component, lint**

Run: `cargo test -p sample-stardew`
Expected: 6 passed.

Run: `cargo xtask plugins`
Expected: prints `sample-stardew -> …/target/dev-plugins/stardew-like/0.1.0` and `test-plugin -> …/target/fixtures/test-plugin.wasm`.

In `.github/workflows/ci.yml`, change the wasm clippy step to:

```yaml
      - run: cargo clippy -p saveforge-sdk -p test-plugin -p sample-stardew --target wasm32-wasip2 -- -D warnings
```

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p saveforge-sdk -p test-plugin -p sample-stardew --target wasm32-wasip2 -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock .github plugins/sample-stardew docs/sample-saves/StardewLike-Farm.json
git commit -m "feat: StardewLike sample plugin on the form helper"
```

---

### Task 4: End-to-end through the wasm build, and a smoke run

**Files:**
- Test: `crates/saveforge-app/tests/sample_plugin.rs`

**Interfaces:**
- Consumes:
  - `target/dev-plugins/stardew-like/0.1.0` (Task 3)
  - `saveforge_app::{documents, editor::Editor, render}`
  - `common::World` (the existing app test helper)

- [ ] **Step 1: Write the test `crates/saveforge-app/tests/sample_plugin.rs`**

```rust
//! End to end through the real `StardewLike` wasm component built by
//! `cargo xtask plugins` into `target/dev-plugins`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use iced_test::simulator;
use saveforge_app::documents::{DetectOutcome, detect_file, open_document, save_document};
use saveforge_app::editor::Editor;
use saveforge_app::render::{UiMessage, render};
use saveforge_host::{Event, PluginRegistry};
use serde_json::{Value, json};

fn dev_plugins() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace.join("target"), PathBuf::from);
    let dir = target.join("dev-plugins");
    assert!(
        dir.join("stardew-like").is_dir(),
        "sample plugin not built: run `cargo xtask plugins` first ({})",
        dir.display()
    );
    dir
}

fn sample_save(dir: &Path) -> PathBuf {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/sample-saves/StardewLike-Farm.json");
    let path = dir.join("Farm.json");
    fs::copy(source, &path).unwrap();
    path
}

#[test]
fn sample_plugin_detects_opens_edits_and_saves() {
    let world = common::World::new();
    let engine = &world.services.engine;
    let registry = PluginRegistry::scan(engine, &dev_plugins());
    let path = sample_save(world.dir.path());

    let DetectOutcome::Open(plugin) = detect_file(engine, &registry, &path).unwrap().outcome else {
        panic!("the sample save should match the StardewLike plugin");
    };
    assert_eq!(plugin.manifest.id, "stardew-like");

    let opened = open_document(engine, &plugin, &path, &world.services.tokens).unwrap();
    let mut editor = Editor::new(opened);
    {
        let mut ui = simulator(render(&editor.tree, &editor.ui));
        assert!(ui.find("Money").is_ok());
        assert!(ui.find("Robin").is_ok());
    }

    editor.handle(UiMessage::NumberEdited {
        id: "player.money".to_owned(),
        text: "99999".to_owned(),
    });
    assert!(editor.dirty);

    let backups = world.dir.path().join("backups");
    let backup = save_document(&editor.session(), &path, &backups).unwrap();
    let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["player"]["money"], json!(99_999));
    assert_eq!(
        saved["farm"]["name"],
        json!("Sunny Acres"),
        "unknown data kept"
    );
    let original: Value = serde_json::from_str(&fs::read_to_string(backup).unwrap()).unwrap();
    assert_eq!(original["player"]["money"], json!(2500));
}

#[test]
fn sample_plugin_blocks_saving_invalid_data() {
    let world = common::World::new();
    let engine = &world.services.engine;
    let registry = PluginRegistry::scan(engine, &dev_plugins());
    let path = sample_save(world.dir.path());
    let DetectOutcome::Open(plugin) = detect_file(engine, &registry, &path).unwrap().outcome else {
        panic!("no match");
    };
    let mut editor =
        Editor::new(open_document(engine, &plugin, &path, &world.services.tokens).unwrap());
    editor.handle(UiMessage::Plugin(Event::TextChanged((
        "player.name".to_owned(),
        String::new(),
    ))));

    let failure =
        save_document(&editor.session(), &path, &world.dir.path().join("backups")).unwrap_err();
    assert!(failure.message.contains("Name is required."), "{failure:?}");
    assert!(!failure.plugin_fault);
    let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk["player"]["name"], json!("Robin"), "file untouched");
}
```

- [ ] **Step 2: Prove the test depends on the build**

Run: `cargo clean -p sample-stardew --release --target wasm32-wasip2 && rm -rf target/dev-plugins && cargo test -p saveforge-app --test sample_plugin`
Expected: both tests FAIL with `sample plugin not built: run \`cargo xtask plugins\` first`. This is the RED step. The behaviour itself already exists from Tasks 2–3, so this test locks in the integration.

- [ ] **Step 3: Build and run**

Run: `cargo xtask plugins && cargo test -p saveforge-app --test sample_plugin`
Expected: 2 passed.

- [ ] **Step 4: Smoke-run the app with the sample plugin**

Run: `cargo build -p saveforge-app`, then launch `target/debug/saveforge.exe` (or `cargo run -p saveforge-app`), wait about 8 s, take a screenshot of the window, and close it. On Windows, the M3 PowerShell snippet works: `Start-Process` → `GetWindowRect` → `CopyFromScreen` → `Stop-Process`.

Expected:
- The rail shows one plugin button, **S** (StardewLike, no icon).
- The save panel shows "StardewLike Save Editor" and either "No save folder" (the default `%APPDATA%/StardewValley/Saves` does not exist) or the saves found there.

Record what the screenshot shows in the ledger. Don't change the user's real settings to point at `docs/sample-saves`; the end-to-end test already covers opening and saving.

- [ ] **Step 5: Lint and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

```bash
git add crates/saveforge-app/tests/sample_plugin.rs
git commit -m "test(app): open, edit and save through the StardewLike wasm plugin"
```

---

### Task 5: Full gate, roadmap

**Files:**
- Modify: `docs/superpowers/plans/2026-10-10-roadmap.md`

- [ ] **Step 1: Run the full gate exactly as CI does**

Run:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p saveforge-sdk -p test-plugin -p sample-stardew --target wasm32-wasip2 -- -D warnings
cargo xtask plugins
cargo test --workspace
cargo deny check
cargo machete
```

Expected:
- fmt prints nothing.
- Both clippy runs report 0 warnings.
- Tests: 207 passed in total: 173 from M1–M3, 26 new SDK tests (form_document 17, form_path 4, form_visibility 5), 6 sample-plugin tests and 2 app tests. Any failure stops the gate.
- `cargo deny check` prints `advisories ok, bans ok, licenses ok, sources ok`.
- machete reports no unused dependencies.

**If `cargo deny` reports a new license or advisory:** `regex-lite` is MIT OR Apache-2.0 and `preserve_order` adds `indexmap` (already in the tree), so nothing new is expected. Stop and report the id or license and the crate to the user.

- [ ] **Step 2: Mark M4 done in the roadmap**

In `docs/superpowers/plans/2026-10-10-roadmap.md`, change the M4 row to:

```markdown
| 4 | `saveforge-sdk::form` + sample Stardew plugin | `2026-10-13-m4-forms-sample-plugin.md` | done |
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-10-roadmap.md Cargo.lock
git commit -m "chore: pass full M4 quality gate"
```

- [ ] **Step 4: Hand off**

Use superpowers:finishing-a-development-branch.
