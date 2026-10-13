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
