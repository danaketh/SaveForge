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
