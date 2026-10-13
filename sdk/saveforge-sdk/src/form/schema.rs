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
