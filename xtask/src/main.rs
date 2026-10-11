use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use saveforge_core::manifest::PluginManifest;
use xtask::{WASM_TARGET, find_plugin_crates, install};

const USAGE: &str = "usage: cargo xtask plugins [-p <package-or-plugin-id>]";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("plugins") => plugins(&args[1..]),
        _ => bail!(USAGE),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives inside the workspace")
        .to_path_buf()
}

fn plugins(args: &[String]) -> Result<()> {
    let filter = match args {
        [] => None,
        [flag, name] if flag == "-p" => Some(name.as_str()),
        _ => bail!(USAGE),
    };
    let root = workspace_root();
    let target_dir =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);

    let crates: Vec<_> = find_plugin_crates(&root)
        .into_iter()
        .filter(|c| {
            filter.is_none_or(|f| {
                c.package == f || PluginManifest::load(&c.dir).is_ok_and(|m| m.id == f)
            })
        })
        .collect();
    if crates.is_empty() {
        bail!("no plugin crate matched");
    }

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&root)
        .args(["build", "--release", "--target", WASM_TARGET]);
    for c in &crates {
        cmd.args(["-p", &c.package]);
    }
    let status = cmd.status().context("failed to run cargo")?;
    if !status.success() {
        bail!("cargo build for {WASM_TARGET} failed");
    }

    for c in &crates {
        let dest = install(&target_dir, c)?;
        println!("{} -> {}", c.package, dest.display());
    }
    Ok(())
}
