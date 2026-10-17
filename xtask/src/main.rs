use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use saveforge_core::settings::file_url;
use xtask::package::{build_package, catalog_json};
use xtask::scaffold::{NewPlugin, scaffold};
use xtask::{
    PluginCrate, WASM_TARGET, find_plugin_crates, install, select_plugins, target_dir,
    wasm_artifact, workspace_root,
};

const USAGE: &str = "usage:
  cargo xtask plugins [-p <package-or-plugin-id>]
  cargo xtask package [-p <package-or-plugin-id>] [--base-url <url>]
  cargo xtask new-plugin <id> [--name <display name>] [--ext <.ext>] [--author <name>]";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("plugins") => plugins(&args[1..]),
        Some("package") => package(&args[1..]),
        Some("new-plugin") => new_plugin(&args[1..]),
        _ => bail!(USAGE),
    }
}

/// `-p <name>` and `--base-url <url>`, each optional; anything else is a
/// usage error. `allow_base_url` is false for `plugins`.
fn parse_flags(args: &[String], allow_base_url: bool) -> Result<(Option<&str>, Option<&str>)> {
    let (mut filter, mut base_url) = (None, None);
    let mut rest = args;
    while let [flag, value, tail @ ..] = rest {
        match flag.as_str() {
            "-p" if filter.is_none() => filter = Some(value.as_str()),
            "--base-url" if allow_base_url && base_url.is_none() => base_url = Some(value.as_str()),
            _ => bail!(USAGE),
        }
        rest = tail;
    }
    if !rest.is_empty() {
        bail!(USAGE);
    }
    Ok((filter, base_url))
}

fn roots() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = workspace_root(
        std::env::var_os("CARGO_MANIFEST_DIR"),
        env!("CARGO_MANIFEST_DIR"),
    );
    let target_dir = target_dir(&root, std::env::var_os("CARGO_TARGET_DIR"));
    (root, target_dir)
}

/// Builds `crates` for the wasm target in release mode.
fn build_wasm(root: &Path, crates: &[PluginCrate]) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(root)
        .args(["build", "--release", "--target", WASM_TARGET]);
    for c in crates {
        cmd.args(["-p", &c.package]);
    }
    let status = cmd.status().context("failed to run cargo")?;
    if !status.success() {
        bail!("cargo build for {WASM_TARGET} failed");
    }
    Ok(())
}

fn plugins(args: &[String]) -> Result<()> {
    let (filter, _) = parse_flags(args, false)?;
    let (root, target_dir) = roots();
    let crates = select_plugins(find_plugin_crates(&root), filter, false)?;
    build_wasm(&root, &crates)?;
    for c in &crates {
        let dest = install(&target_dir, c)?;
        println!("{} -> {}", c.package, dest.display());
    }
    Ok(())
}

fn package(args: &[String]) -> Result<()> {
    let (filter, base_url) = parse_flags(args, true)?;
    let (root, target_dir) = roots();
    let crates = select_plugins(find_plugin_crates(&root), filter, true)?;
    build_wasm(&root, &crates)?;

    let out = target_dir.join("packages");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("cannot clear {}", out.display()))?;
    }
    let mut packages = Vec::new();
    for c in &crates {
        let package = build_package(&c.dir, &wasm_artifact(&target_dir, &c.package), &out)?;
        println!("{}  sha256:{}", package.path.display(), package.sha256);
        packages.push(package);
    }
    let default_base = format!("{}/", file_url(&out));
    let catalog = catalog_json(&packages, base_url.unwrap_or(&default_base))?;
    let catalog_path = out.join("catalog.json");
    fs::write(&catalog_path, &catalog)
        .with_context(|| format!("cannot write {}", catalog_path.display()))?;
    println!("catalog: {}", catalog_path.display());
    print!("{catalog}");
    Ok(())
}

fn new_plugin(args: &[String]) -> Result<()> {
    let [id, flags @ ..] = args else {
        bail!(USAGE);
    };
    let mut plugin = NewPlugin::new(id)?;
    let mut rest = flags;
    while let [flag, value, tail @ ..] = rest {
        match flag.as_str() {
            "--name" => plugin.name.clone_from(value),
            "--ext" if value.starts_with('.') => plugin.extension.clone_from(value),
            "--ext" => plugin.extension = format!(".{value}"),
            "--author" => plugin.author.clone_from(value),
            _ => bail!(USAGE),
        }
        rest = tail;
    }
    if !rest.is_empty() {
        bail!(USAGE);
    }
    let (root, _) = roots();
    scaffold(&root, &plugin)?;
    println!("created plugins/{id}");
    // Generated lines vary in length with the names, so let rustfmt lay
    // them out.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let formatted = Command::new(cargo)
        .current_dir(&root)
        .args(["fmt", "-p", id])
        .status()
        .is_ok_and(|s| s.success());
    if !formatted {
        eprintln!("warning: cargo fmt -p {id} failed; run it before committing");
    }
    println!("next: cargo test -p {id} && cargo xtask plugins -p {id}");
    Ok(())
}
