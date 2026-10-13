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
