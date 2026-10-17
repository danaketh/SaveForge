//! The Planet Crafter save format (spec §6.1), parsed losslessly.
//!
//! A save is a sequence of groups separated by `@`, and a group holds JSON
//! objects separated by `|`. Everything between objects (whitespace, the
//! separators, a leading byte order mark) is kept verbatim, and so is each
//! object's own text: an edit replaces only the edited value's text. An
//! unmodified save therefore serializes byte for byte as it was read, and an
//! edited one differs only where it was edited.

use std::mem;
use std::ops::Range;

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

/// One JSON object of a group: its parsed fields and its exact text.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    raw: String,
    fields: Map<String, Value>,
}

impl Object {
    /// A new object, written out compactly.
    pub fn new(fields: Map<String, Value>) -> Self {
        Self {
            raw: Value::Object(fields.clone()).to_string(),
            fields,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    pub fn fields(&self) -> &Map<String, Value> {
        &self.fields
    }

    /// Sets `key`. Only the text of its value changes: of its last
    /// occurrence when the key is repeated (the one parsing keeps), or a new
    /// member after the last one. Returns whether anything changed.
    pub fn set(&mut self, key: &str, value: Value) -> bool {
        if self.fields.get(key) == Some(&value) {
            return false;
        }
        let text = value.to_string();
        let members = members(&self.raw);
        if let Some((_, range)) = members.iter().rev().find(|(k, _)| k == key) {
            self.raw.replace_range(range.clone(), &text);
        } else {
            let name = Value::from(key).to_string();
            if let Some((_, last)) = members.last() {
                self.raw.insert_str(last.end, &format!(",{name}:{text}"));
            } else {
                let close = self.raw.rfind('}').unwrap_or(self.raw.len());
                self.raw.insert_str(close, &format!("{name}:{text}"));
            }
        }
        self.fields.insert(key.to_owned(), value);
        true
    }

    fn write(&self, out: &mut String) {
        out.push_str(&self.raw);
    }
}

/// The top-level members of a JSON object's text: each decoded key and the
/// byte range of its value. `raw` is text the JSON parser accepted.
fn members(raw: &str) -> Vec<(String, Range<usize>)> {
    let bytes = raw.as_bytes();
    let skip_space = |mut i: usize| {
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        i
    };
    let mut found = Vec::new();
    let mut i = skip_space(1);
    while bytes.get(i) == Some(&b'"') {
        let key_end = string_end(bytes, i);
        let key = serde_json::from_str(&raw[i..key_end]).unwrap_or_default();
        i = skip_space(key_end) + 1; // past ':'
        let start = skip_space(i);
        let end = value_end(bytes, start);
        found.push((key, start..end));
        i = skip_space(end);
        if bytes.get(i) != Some(&b',') {
            break;
        }
        i = skip_space(i + 1);
    }
    found
}

/// The index after the string that starts at `start` (a `"`).
fn string_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while let Some(&b) = bytes.get(i) {
        match b {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// The index after the JSON value that starts at `start`.
fn value_end(bytes: &[u8], start: usize) -> usize {
    match bytes.get(start) {
        Some(b'"') => string_end(bytes, start),
        Some(b'{' | b'[') => {
            let mut depth = 0usize;
            let mut i = start;
            while let Some(&b) = bytes.get(i) {
                match b {
                    b'"' => {
                        i = string_end(bytes, i);
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return i + 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            bytes.len()
        }
        _ => {
            let mut i = start;
            while bytes
                .get(i)
                .is_some_and(|b| !matches!(b, b',' | b'}' | b']') && !b.is_ascii_whitespace())
            {
                i += 1;
            }
            i
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
                            let (line, column) = file_position(text, i, &e);
                            let message = e.to_string();
                            let reason = message
                                .rsplit_once(" at line ")
                                .map_or(message.as_str(), |(reason, _)| reason);
                            return Err(format!(
                                "{} object {} is not valid JSON at line {line} column {column}: {reason}",
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

/// Where in the whole file a JSON error inside the object starting at byte
/// `start` is: 1-based line and column, as serde counts them.
fn file_position(text: &str, start: usize, error: &serde_json::Error) -> (usize, usize) {
    let before = &text[..start];
    let line = before.matches('\n').count() + error.line();
    let column = if error.line() == 1 {
        let line_start = before.rfind('\n').map_or(0, |n| n + 1);
        start - line_start + error.column()
    } else {
        error.column()
    };
    (line, column)
}

fn group_name(index: usize) -> String {
    GROUP_NAMES
        .get(index)
        .map_or_else(|| format!("group {index}"), |name| (*name).to_owned())
}
