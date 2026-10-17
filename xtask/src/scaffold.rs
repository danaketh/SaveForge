//! `cargo xtask new-plugin`: a new form-based plugin crate under `plugins/`,
//! registered in the workspace and the CI wasm lint (spec §10).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use saveforge_core::manifest::{PluginManifest, is_valid_id};

/// The plugin API version new plugins declare (`major.minor` of the host's).
pub const API_VERSION: &str = "0.1";

const CARGO_TOML: &str = include_str!("../templates/Cargo.toml.tmpl");
const MANIFEST: &str = include_str!("../templates/manifest.json.tmpl");
const LIB_RS: &str = include_str!("../templates/lib.rs.tmpl");
const TEST_RS: &str = include_str!("../templates/test.rs.tmpl");

/// Words a crate name cannot be: Rust keywords (strict, reserved, and `gen`
/// from edition 2024) and the names of the standard crates.
#[rustfmt::skip]
const RESERVED: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
    "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "gen",
    "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield", "union",
    "std", "core", "alloc", "test", "proc_macro",
];

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
        if RESERVED.contains(&id.replace('-', "_").as_str()) {
            bail!("'{id}' is a reserved word in Rust");
        }
        let words: Vec<String> = id.split('-').map(capitalize).collect();
        Ok(Self {
            id: id.to_owned(),
            name: format!("{} Save Editor", words.join(" ")),
            extension: ".json".to_owned(),
            author: "Unknown".to_owned(),
        })
    }

    /// Checks the display name, author and extension: they end up in the
    /// manifest and in Rust source.
    pub fn validate(&self) -> anyhow::Result<()> {
        for (flag, value) in [("--name", &self.name), ("--author", &self.author)] {
            if value.trim().is_empty() {
                bail!("{flag} must not be empty");
            }
            if value.contains(['"', '\\', '\n', '\r']) {
                bail!("{flag} must not contain quotes, backslashes or line breaks");
            }
        }
        let valid_extension = self.extension.strip_prefix('.').is_some_and(|rest| {
            !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_alphanumeric())
        });
        if !valid_extension {
            bail!(
                "--ext must be a dot and letters or digits, e.g. .sav (got '{}')",
                self.extension
            );
        }
        Ok(())
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

/// The entries of the workspace's `members = [ … ]` list.
fn workspace_members(cargo_toml: &str) -> Vec<String> {
    cargo_toml
        .lines()
        .skip_while(|l| l.trim() != "members = [")
        .skip(1)
        .take_while(|l| !matches!(l.trim(), "]" | "],"))
        .map(|l| l.trim().trim_end_matches(',').trim_matches('"').to_owned())
        .collect()
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

/// Creates `plugins/{id}` under `workspace` and registers it. Everything is
/// checked and rendered first, and the crate is written to a temporary
/// folder that is renamed into place, so an error leaves the workspace as it
/// was.
pub fn scaffold(workspace: &Path, plugin: &NewPlugin) -> anyhow::Result<PathBuf> {
    plugin.validate()?;
    let id = &plugin.id;
    let cargo_path = workspace.join("Cargo.toml");
    let cargo = fs::read_to_string(&cargo_path)
        .with_context(|| format!("cannot read {}", cargo_path.display()))?;
    if workspace.join("plugins").join(id).exists() {
        bail!("plugins/{id} already exists");
    }
    let fixture = format!("fixtures/{id}");
    let used_by = workspace_members(&cargo)
        .into_iter()
        .find(|member| member.rsplit('/').next() == Some(id.as_str()))
        .or_else(|| workspace.join(&fixture).exists().then_some(fixture));
    if let Some(used_by) = used_by {
        bail!("'{id}' is already used by {used_by}");
    }
    let cargo = add_workspace_member(&cargo, &format!("plugins/{id}"))?;

    let files = [
        ("Cargo.toml".to_owned(), render(CARGO_TOML, plugin)),
        ("manifest.json".to_owned(), render(MANIFEST, plugin)),
        ("src/lib.rs".to_owned(), render(LIB_RS, plugin)),
        (
            format!("tests/{}.rs", plugin.crate_name()),
            render(TEST_RS, plugin),
        ),
    ];
    PluginManifest::parse(&files[1].1).context("the generated manifest is invalid")?;

    let plugins = workspace.join("plugins");
    fs::create_dir_all(&plugins).with_context(|| format!("cannot create {}", plugins.display()))?;
    let staged = tempfile::Builder::new()
        .prefix(".new-")
        .tempdir_in(&plugins)
        .with_context(|| format!("cannot write in {}", plugins.display()))?;
    for (path, text) in &files {
        let path = staged.path().join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))?;
    }
    let dir = plugins.join(id);
    fs::rename(staged.path(), &dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let _ = staged.keep();
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
