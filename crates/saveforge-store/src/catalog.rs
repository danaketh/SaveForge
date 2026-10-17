//! The plugin catalog (`../dotnet/docs/CATALOG_SCHEMA.md`): parsing, status,
//! search and update detection.

use saveforge_core::manifest::is_valid_id;
use semver::Version;
use serde::Deserialize;
use serde_json::Value;

use crate::StoreError;

/// A parsed catalog. Malformed entries are dropped by [`Catalog::parse`].
#[derive(Debug, Clone, PartialEq)]
pub struct Catalog {
    pub schema_version: String,
    pub generated_at: Option<String>,
    pub plugins: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub author: String,
    pub latest_version: Version,
    pub min_host_version: Version,
    #[serde(default)]
    pub max_host_version: Option<Version>,
    pub package_url: String,
    /// Lowercase hex.
    pub sha256: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub screenshot_urls: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub repository_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryStatus {
    NotInstalled,
    Installed { version: Version },
    UpdateAvailable { installed: Version },
    Incompatible,
}

/// An installed plugin with a newer compatible catalog version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub id: String,
    pub name: String,
    pub installed: Version,
    pub latest: Version,
}

impl Catalog {
    /// Fails only when the top level is not a catalog object; individual bad
    /// entries are skipped with a warning.
    pub fn parse(text: &str) -> Result<Self, StoreError> {
        let value: Value =
            serde_json::from_str(text).map_err(|e| StoreError::InvalidCatalog(e.to_string()))?;
        let schema_version = value
            .get("schemaVersion")
            .and_then(Value::as_str)
            .ok_or_else(|| StoreError::InvalidCatalog("missing schemaVersion".into()))?
            .to_owned();
        let generated_at = value
            .get("generatedAt")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let entries = value
            .get("plugins")
            .and_then(Value::as_array)
            .ok_or_else(|| StoreError::InvalidCatalog("missing plugins array".into()))?;
        let plugins = entries
            .iter()
            .enumerate()
            .filter_map(|(index, raw)| match parse_entry(raw) {
                Ok(entry) => Some(entry),
                Err(reason) => {
                    tracing::warn!(index, reason, "skipping catalog entry");
                    None
                }
            })
            .collect();
        Ok(Self {
            schema_version,
            generated_at,
            plugins,
        })
    }

    pub fn entry(&self, id: &str) -> Option<&CatalogEntry> {
        self.plugins.iter().find(|e| e.id == id)
    }

    /// Entries matching `query`, in catalog order.
    pub fn search(&self, query: &str) -> Vec<&CatalogEntry> {
        self.plugins.iter().filter(|e| e.matches(query)).collect()
    }

    /// The installed plugins whose status is [`EntryStatus::UpdateAvailable`].
    pub fn updates(&self, installed: &[(String, Version)], host: &Version) -> Vec<Update> {
        installed
            .iter()
            .filter_map(|(id, version)| {
                let entry = self.entry(id)?;
                matches!(
                    entry.status(Some(version), host),
                    EntryStatus::UpdateAvailable { .. }
                )
                .then(|| Update {
                    id: id.clone(),
                    name: entry.name.clone(),
                    installed: version.clone(),
                    latest: entry.latest_version.clone(),
                })
            })
            .collect()
    }
}

fn parse_entry(raw: &Value) -> Result<CatalogEntry, String> {
    let mut entry = CatalogEntry::deserialize(raw).map_err(|e| e.to_string())?;
    if !is_valid_id(&entry.id) {
        return Err(format!("invalid id {:?}", entry.id));
    }
    if entry.sha256.len() != 64 || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("{}: sha256 is not 64 hex digits", entry.id));
    }
    entry.sha256.make_ascii_lowercase();
    Ok(entry)
}

impl CatalogEntry {
    /// The host lies within `[minHostVersion, maxHostVersion]`.
    pub fn is_compatible(&self, host: &Version) -> bool {
        *host >= self.min_host_version
            && self.max_host_version.as_ref().is_none_or(|max| host <= max)
    }

    /// Case-insensitive substring match over name, description, author and
    /// tags; a blank query matches everything.
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        [&self.name, &self.description, &self.author]
            .into_iter()
            .chain(&self.tags)
            .any(|field| field.to_lowercase().contains(&query))
    }

    pub fn status(&self, installed: Option<&Version>, host: &Version) -> EntryStatus {
        match installed {
            None if self.is_compatible(host) => EntryStatus::NotInstalled,
            None => EntryStatus::Incompatible,
            Some(version) if self.latest_version > *version && self.is_compatible(host) => {
                EntryStatus::UpdateAvailable {
                    installed: version.clone(),
                }
            }
            Some(version) => EntryStatus::Installed {
                version: version.clone(),
            },
        }
    }
}
