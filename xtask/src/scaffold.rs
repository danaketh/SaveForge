//! `cargo xtask new-plugin`: a new form-based plugin crate under `plugins/`,
//! registered in the workspace and the CI wasm lint (spec §10).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use saveforge_core::manifest::is_valid_id;

/// The plugin API version new plugins declare (`major.minor` of the host's).
pub const API_VERSION: &str = "0.1";

const CARGO_TOML: &str = include_str!("../templates/Cargo.toml.tmpl");
const MANIFEST: &str = include_str!("../templates/manifest.json.tmpl");
const LIB_RS: &str = include_str!("../templates/lib.rs.tmpl");
const TEST_RS: &str = include_str!("../templates/test.rs.tmpl");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPlugin {
    pub id: String,
    /// Display name, e.g. `My Game Save Editor`.
    pub name: String,
    /// File extension with its leading `.`.
    pub extension: String,
    pub author: String,
}

impl NewPlugin {
    /// Validates `id` and fills in the defaults.
    pub fn new(id: &str) -> anyhow::Result<Self> {
        if !is_valid_id(id) {
            bail!(
                "invalid plugin id '{id}': use lowercase letters, digits and single hyphens, e.g. my-game"
            );
        }
        if !id.starts_with(|c: char| c.is_ascii_lowercase()) {
            bail!("invalid plugin id '{id}': it must start with a letter to be a crate name");
        }
        let words: Vec<String> = id.split('-').map(capitalize).collect();
        Ok(Self {
            id: id.to_owned(),
            name: format!("{} Save Editor", words.join(" ")),
            extension: ".json".to_owned(),
            author: "Unknown".to_owned(),
        })
    }

    /// The Rust crate name: `my-game` → `my_game`.
    pub fn crate_name(&self) -> String {
        self.id.replace('-', "_")
    }

    /// The plugin type's name: `my-game` → `MyGame`.
    pub fn type_name(&self) -> String {
        self.id.split('-').map(capitalize).collect()
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

/// Fills a template's `{{placeholders}}` for `plugin`.
pub fn render(template: &str, plugin: &NewPlugin) -> String {
    [
        ("{{id}}", plugin.id.clone()),
        ("{{crate_name}}", plugin.crate_name()),
        ("{{type_name}}", plugin.type_name()),
        ("{{name}}", plugin.name.clone()),
        ("{{extension}}", plugin.extension.clone()),
        ("{{author}}", plugin.author.clone()),
        ("{{api_version}}", API_VERSION.to_owned()),
        ("{{host_version}}", env!("CARGO_PKG_VERSION").to_owned()),
    ]
    .iter()
    .fold(template.to_owned(), |text, (key, value)| {
        text.replace(key, value)
    })
}

/// Adds `member` to the multi-line `members = [ … ]` list of a workspace
/// `Cargo.toml`, in sorted position, keeping the file's line endings.
pub fn add_workspace_member(cargo_toml: &str, member: &str) -> anyhow::Result<String> {
    let newline = if cargo_toml.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines: Vec<&str> = cargo_toml.split(newline).collect();
    let not_found =
        || anyhow::anyhow!("could not find a multi-line members = [ list in Cargo.toml");
    let start = lines
        .iter()
        .position(|l| l.trim() == "members = [")
        .ok_or_else(not_found)?;
    let end = start
        + lines[start..]
            .iter()
            .position(|l| matches!(l.trim(), "]" | "],"))
            .ok_or_else(not_found)?;
    let value = |line: &str| {
        line.trim()
            .trim_end_matches(',')
            .trim_matches('"')
            .to_owned()
    };
    if lines[start + 1..end].iter().any(|l| value(l) == member) {
        bail!("{member} is already a workspace member");
    }
    let position = (start + 1..end)
        .find(|&i| value(lines[i]).as_str() > member)
        .unwrap_or(end);
    let indent = lines
        .get(start + 1)
        .filter(|_| start + 1 < end)
        .map_or("    ", |l| &l[..l.len() - l.trim_start().len()]);
    let entry = format!("{indent}\"{member}\",");
    lines.insert(position, &entry);
    Ok(lines.join(newline))
}

/// Adds `-p {package}` to the CI line that lints the plugins for wasm.
/// `None` when there is no such line or it already lists the package.
pub fn add_wasm_clippy_package(ci_yml: &str, package: &str) -> Option<String> {
    const TARGET: &str = " --target wasm32-wasip2";
    let line = ci_yml
        .split('\n')
        .find(|l| l.contains("cargo clippy") && l.contains(TARGET))?;
    if line.contains(&format!("-p {package} ")) {
        return None;
    }
    let updated = line.replacen(TARGET, &format!(" -p {package}{TARGET}"), 1);
    Some(ci_yml.replacen(line, &updated, 1))
}

/// Creates `plugins/{id}` under `workspace` and registers it. Nothing is
/// written when the id's folder exists or the workspace can't be updated.
pub fn scaffold(workspace: &Path, plugin: &NewPlugin) -> anyhow::Result<PathBuf> {
    let id = &plugin.id;
    let dir = workspace.join("plugins").join(id);
    if dir.exists() {
        bail!("plugins/{id} already exists");
    }
    let cargo_path = workspace.join("Cargo.toml");
    let cargo = fs::read_to_string(&cargo_path)
        .with_context(|| format!("cannot read {}", cargo_path.display()))?;
    let cargo = add_workspace_member(&cargo, &format!("plugins/{id}"))?;

    for (path, template) in [
        ("Cargo.toml".to_owned(), CARGO_TOML),
        ("manifest.json".to_owned(), MANIFEST),
        ("src/lib.rs".to_owned(), LIB_RS),
        (format!("tests/{}.rs", plugin.crate_name()), TEST_RS),
    ] {
        let path = dir.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, render(template, plugin))
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    fs::write(&cargo_path, cargo)
        .with_context(|| format!("cannot write {}", cargo_path.display()))?;

    let ci_path = workspace.join(".github/workflows/ci.yml");
    if let Ok(ci) = fs::read_to_string(&ci_path)
        && let Some(updated) = add_wasm_clippy_package(&ci, id)
    {
        fs::write(&ci_path, updated)
            .with_context(|| format!("cannot write {}", ci_path.display()))?;
    }
    Ok(dir)
}
