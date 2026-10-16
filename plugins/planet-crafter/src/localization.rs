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
