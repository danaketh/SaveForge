use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use saveforge_core::settings::file_url;
use saveforge_store::Catalog;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use xtask::package::{Package, build_package, catalog_json, write_packages};
use xtask::{find_plugin_crates, select_plugins};

const MANIFEST: &str = r#"{
  "id": "demo", "name": "Demo", "version": "1.2.3", "author": "a",
  "description": "A demo plugin.",
  "apiVersion": "0.1", "minHostVersion": "0.1.0",
  "component": "plugin.wasm", "icon": "icon.png", "fileExtensions": [".sav"]
}"#;

const WASM: &[u8] = b"\0asm fake";

struct Fixture {
    tmp: TempDir,
}

impl Fixture {
    fn new(manifest: &str) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let krate = tmp.path().join("crate");
        fs::create_dir_all(&krate).unwrap();
        fs::write(krate.join("manifest.json"), manifest).unwrap();
        fs::write(krate.join("icon.png"), b"png").unwrap();
        fs::write(tmp.path().join("plugin.wasm"), WASM).unwrap();
        Self { tmp }
    }

    fn krate(&self) -> PathBuf {
        self.tmp.path().join("crate")
    }

    fn build(&self, out: &str) -> anyhow::Result<Package> {
        build_package(
            &self.krate(),
            &self.tmp.path().join("plugin.wasm"),
            &self.tmp.path().join(out),
        )
    }
}

fn entries(path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            (file.name().unwrap().into_owned(), bytes)
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, b| {
            write!(out, "{b:02x}").unwrap();
            out
        })
}

#[test]
fn package_contains_manifest_component_and_icon() {
    let fixture = Fixture::new(MANIFEST);
    let package = fixture.build("out").unwrap();
    assert_eq!(
        package.path.file_name().unwrap().to_str(),
        Some("demo-1.2.3.zip")
    );
    let entries = entries(&package.path);
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["manifest.json", "plugin.wasm", "icon.png"]);
    assert_eq!(entries[1].1, WASM);
}

#[test]
fn component_paths_keep_forward_slashes() {
    let fixture = Fixture::new(&MANIFEST.replace("\"plugin.wasm\"", "\"bin/plugin.wasm\""));
    let package = fixture.build("out").unwrap();
    let names: Vec<String> = entries(&package.path).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["manifest.json", "bin/plugin.wasm", "icon.png"]);
}

#[test]
fn packages_are_reproducible() {
    let fixture = Fixture::new(MANIFEST);
    let first = fixture.build("a").unwrap();
    let second = fixture.build("b").unwrap();
    assert_eq!(
        fs::read(&first.path).unwrap(),
        fs::read(&second.path).unwrap()
    );
    assert_eq!(first.sha256, second.sha256);
}

#[test]
fn sha256_matches_the_file() {
    let fixture = Fixture::new(MANIFEST);
    let package = fixture.build("out").unwrap();
    assert_eq!(package.sha256, hex(&fs::read(&package.path).unwrap()));
}

#[test]
fn missing_icon_is_an_error() {
    let fixture = Fixture::new(MANIFEST);
    fs::remove_file(fixture.krate().join("icon.png")).unwrap();
    let err = fixture.build("out").unwrap_err();
    assert!(format!("{err:#}").contains("icon.png"), "{err:#}");
    let out = fixture.tmp.path().join("out");
    let zips = fs::read_dir(&out).map_or(0, |dir| dir.flatten().count());
    assert_eq!(zips, 0);
}

#[test]
fn base_url_must_be_https_or_file() {
    let fixture = Fixture::new(MANIFEST);
    let package = fixture.build("out").unwrap();
    let err = catalog_json(std::slice::from_ref(&package), "http://x/").unwrap_err();
    assert_eq!(
        err.to_string(),
        "base URL must start with https:// or file://: http://x/"
    );
    let json = catalog_json(&[package], "https://h/r").unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        value["plugins"][0]["packageUrl"],
        "https://h/r/demo-1.2.3.zip"
    );
    assert!(json.ends_with('\n'));
}

#[test]
fn catalog_round_trips_through_the_store() {
    let fixture = Fixture::new(MANIFEST);
    let package = fixture.build("out").unwrap();
    let base = file_url(&fixture.tmp.path().join("out"));
    let json = catalog_json(std::slice::from_ref(&package), &base).unwrap();
    let catalog = Catalog::parse(&json).unwrap();
    let entry = catalog.entry("demo").unwrap();
    assert_eq!(entry.latest_version.to_string(), "1.2.3");
    assert_eq!(entry.sha256, package.sha256);
    assert_eq!(entry.max_host_version, None);
    assert_eq!(entry.description, "A demo plugin.");

    let plugins = fixture.tmp.path().join("plugins");
    let dir = saveforge_store::install::install(entry, &plugins, &mut |_, _| {}).unwrap();
    for file in ["manifest.json", "plugin.wasm", "icon.png"] {
        assert!(dir.join(file).is_file(), "{file}");
    }
}

#[test]
fn package_refuses_fixtures_and_unknown_ids() {
    let root = tempfile::tempdir().unwrap();
    for (sub, name) in [("plugins", "demo"), ("fixtures", "test-plugin")] {
        let dir = root.path().join(sub).join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
        fs::write(
            dir.join("manifest.json"),
            MANIFEST.replace("\"demo\"", &format!("\"{name}\"")),
        )
        .unwrap();
    }
    let crates = || find_plugin_crates(root.path());
    let names = |found: Vec<xtask::PluginCrate>| -> Vec<String> {
        found.into_iter().map(|c| c.package).collect()
    };

    let err = select_plugins(crates(), Some("test-plugin"), true).unwrap_err();
    assert_eq!(
        err.to_string(),
        "test-plugin is a test fixture, not a plugin"
    );
    let err = select_plugins(crates(), Some("nope"), true).unwrap_err();
    assert_eq!(err.to_string(), "no plugin crate matched nope");
    assert_eq!(
        names(select_plugins(crates(), None, true).unwrap()),
        ["demo"]
    );
    assert_eq!(
        names(select_plugins(crates(), None, false).unwrap()),
        ["demo", "test-plugin"]
    );
}

#[test]
fn failed_packaging_keeps_the_previous_output() {
    let fixture = Fixture::new(MANIFEST);
    let out = fixture.tmp.path().join("packages");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("marker.txt"), b"previous").unwrap();
    let plugins = [
        (fixture.krate(), fixture.tmp.path().join("plugin.wasm")),
        (fixture.krate(), fixture.tmp.path().join("missing.wasm")),
    ];
    write_packages(&out, &plugins, None).unwrap_err();
    assert_eq!(fs::read(out.join("marker.txt")).unwrap(), b"previous");
    assert!(!fixture.tmp.path().join("packages.tmp").exists());
}

#[test]
fn write_packages_replaces_the_output() {
    let fixture = Fixture::new(MANIFEST);
    let out = fixture.tmp.path().join("packages");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("marker.txt"), b"previous").unwrap();
    let plugins = [(fixture.krate(), fixture.tmp.path().join("plugin.wasm"))];
    let (packages, catalog) = write_packages(&out, &plugins, None).unwrap();
    assert_eq!(packages.len(), 1);
    assert!(out.join("demo-1.2.3.zip").is_file());
    assert_eq!(
        fs::read_to_string(out.join("catalog.json")).unwrap(),
        catalog
    );
    assert!(!out.join("marker.txt").exists());
    let value: serde_json::Value = serde_json::from_str(&catalog).unwrap();
    assert_eq!(
        value["plugins"][0]["packageUrl"],
        format!("{}/demo-1.2.3.zip", file_url(&out))
    );
}
