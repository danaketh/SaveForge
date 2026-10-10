//! `manifest.json` shipped inside every plugin package (spec §4.2).

use std::collections::HashSet;
use std::path::Path;

use semver::Version;
use serde::Deserialize;

use crate::platform::Os;
use crate::{CoreError, Result};

pub const MANIFEST_FILE_NAME: &str = "manifest.json";
pub const DEFAULT_AUX_MAX_BYTES: u64 = 8 * 1024 * 1024;
pub const AUX_MAX_BYTES_CAP: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub api_version: String,
    pub min_host_version: Version,
    #[serde(default)]
    pub max_host_version: Option<Version>,
    pub component: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub file_extensions: Vec<String>,
    #[serde(default)]
    pub default_save_directories: DefaultSaveDirectories,
    #[serde(default)]
    pub aux_files: Vec<AuxFileSpec>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct DefaultSaveDirectories {
    #[serde(default)]
    pub windows: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
    #[serde(default)]
    pub macos: Option<String>,
}

impl DefaultSaveDirectories {
    pub fn for_os(&self, os: Os) -> Option<&str> {
        match os {
            Os::Windows => self.windows.as_deref(),
            Os::Linux => self.linux.as_deref(),
            Os::MacOs => self.macos.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuxFileSpec {
    pub id: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default = "default_aux_max_bytes")]
    pub max_bytes: u64,
    pub candidates: Vec<String>,
}

fn default_aux_max_bytes() -> u64 {
    DEFAULT_AUX_MAX_BYTES
}

impl PluginManifest {
    /// Parses and validates the manifest shape. Host compatibility is checked
    /// separately by [`PluginManifest::check_compat`].
    pub fn parse(json: &str) -> Result<Self> {
        let manifest: Self =
            serde_json::from_str(json).map_err(|e| CoreError::Manifest(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load(plugin_dir: &Path) -> Result<Self> {
        let path = plugin_dir.join(MANIFEST_FILE_NAME);
        let json = std::fs::read_to_string(&path).map_err(|e| CoreError::io(&path, e))?;
        Self::parse(&json)
    }

    pub fn check_compat(&self, host_version: &Version, host_api: &Version) -> Result<()> {
        let api = parse_api_version(&self.api_version).ok_or_else(|| {
            invalid(format!(
                "apiVersion '{}' is not 'major.minor'",
                self.api_version
            ))
        })?;
        if !api_compatible(api, host_api) {
            return Err(CoreError::Incompatible(format!(
                "plugin API {} is not supported by host API {}.{}",
                self.api_version, host_api.major, host_api.minor
            )));
        }
        if *host_version < self.min_host_version {
            return Err(CoreError::Incompatible(format!(
                "requires SaveForge {} or newer (running {host_version})",
                self.min_host_version
            )));
        }
        if let Some(max) = &self.max_host_version
            && host_version > max
        {
            return Err(CoreError::Incompatible(format!(
                "supports SaveForge up to {max} (running {host_version})"
            )));
        }
        Ok(())
    }

    pub fn handles_extension(&self, path: &Path) -> bool {
        matches_extension(path, &self.file_extensions)
    }

    fn validate(&self) -> Result<()> {
        if !is_valid_id(&self.id) {
            return Err(invalid(format!(
                "id '{}' must match ^[a-z0-9]+(-[a-z0-9]+)*$",
                self.id
            )));
        }
        if parse_api_version(&self.api_version).is_none() {
            return Err(invalid(format!(
                "apiVersion '{}' is not 'major.minor'",
                self.api_version
            )));
        }
        if !is_safe_relative_path(&self.component) {
            return Err(invalid(format!(
                "component '{}' is not a safe relative path",
                self.component
            )));
        }
        if let Some(icon) = &self.icon
            && !is_safe_relative_path(icon)
        {
            return Err(invalid(format!(
                "icon '{icon}' is not a safe relative path"
            )));
        }
        if self.file_extensions.is_empty() {
            return Err(invalid("fileExtensions must not be empty"));
        }
        if let Some(bad) = self
            .file_extensions
            .iter()
            .find(|e| e.len() < 2 || !e.starts_with('.'))
        {
            return Err(invalid(format!(
                "file extension '{bad}' must start with '.'"
            )));
        }
        let mut seen = HashSet::new();
        for aux in &self.aux_files {
            if !seen.insert(aux.id.as_str()) {
                return Err(invalid(format!("duplicate auxFiles id '{}'", aux.id)));
            }
            if aux.max_bytes == 0 || aux.max_bytes > AUX_MAX_BYTES_CAP {
                return Err(invalid(format!(
                    "auxFiles '{}' maxBytes must be 1..={AUX_MAX_BYTES_CAP}",
                    aux.id
                )));
            }
            if aux.candidates.is_empty() {
                return Err(invalid(format!("auxFiles '{}' has no candidates", aux.id)));
            }
        }
        Ok(())
    }
}

/// `^[a-z0-9]+(-[a-z0-9]+)*$`
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Relative path that cannot escape its base directory on any OS.
pub fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && path.split(['/', '\\']).all(|part| part != "..")
}

/// True when `path`'s extension equals one of `extensions` (each written as `.ext`),
/// ignoring ASCII case.
pub fn matches_extension(path: &Path, extensions: &[String]) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    extensions.iter().any(|allowed| {
        allowed
            .strip_prefix('.')
            .is_some_and(|a| a.eq_ignore_ascii_case(ext))
    })
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Manifest(message.into())
}

fn parse_api_version(s: &str) -> Option<(u64, u64)> {
    let (major, minor) = s.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn api_compatible((major, minor): (u64, u64), host: &Version) -> bool {
    if host.major == 0 {
        major == 0 && minor == host.minor
    } else {
        major == host.major && minor <= host.minor
    }
}
