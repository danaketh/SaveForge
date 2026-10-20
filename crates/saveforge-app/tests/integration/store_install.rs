//! A plugin installed from a `file://` catalog opens a save, end to end.

use crate::common;

use std::fmt::Write as _;
use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

use crate::common::World;
use saveforge_app::documents::{DetectOutcome, detect_file, open_document};
use saveforge_core::settings::file_url;
use saveforge_host::{PluginEntry, PluginRegistry};
use saveforge_store::cache::{CatalogSource, load_catalog};
use saveforge_store::install::install;
use sha2::{Digest, Sha256};

fn package(id: &str) -> Vec<u8> {
    let manifest_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/test-plugin/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    manifest["id"] = serde_json::Value::from(id);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.to_string().as_bytes()).unwrap();
    zip.start_file("plugin.wasm", options).unwrap();
    zip.write_all(&fs::read(common::fixture_wasm()).unwrap())
        .unwrap();
    zip.finish().unwrap().into_inner()
}

#[test]
fn installs_from_a_file_catalog_and_opens_a_save() {
    let world = World::new();
    let plugins = world.dir.path().join("store plugins");
    let catalog_dir = world.dir.path().join("my catalog");
    fs::create_dir_all(&catalog_dir).unwrap();

    let zip = package("store-demo");
    let zip_path = catalog_dir.join("store-demo 0.1.0.zip");
    fs::write(&zip_path, &zip).unwrap();
    let sha = Sha256::digest(&zip)
        .iter()
        .fold(String::new(), |mut out, b| {
            write!(out, "{b:02x}").unwrap();
            out
        });
    let catalog = serde_json::json!({
        "schemaVersion": "1",
        "plugins": [{
            "id": "store-demo", "name": "Store Demo", "author": "SaveForge",
            "latestVersion": "0.1.0", "minHostVersion": "0.1.0",
            "packageUrl": file_url(&zip_path), "sha256": sha.to_uppercase()
        }]
    });
    let catalog_path = catalog_dir.join("catalog.json");
    fs::write(&catalog_path, catalog.to_string()).unwrap();

    let load = load_catalog(
        &file_url(&catalog_path),
        &world.services.paths.catalog_cache_file(),
    )
    .unwrap();
    assert_eq!(load.source, CatalogSource::Online);
    let entry = load.catalog.entry("store-demo").unwrap();
    let dir = install(entry, &plugins, &mut |_, _| {}).unwrap();
    assert_eq!(dir, plugins.join("store-demo").join("0.1.0"));

    let engine = &world.services.engine;
    let loaded = PluginRegistry::load_entry(engine, &plugins, "store-demo");
    assert!(matches!(loaded, PluginEntry::Loaded(_)), "{loaded:?}");
    let mut registry = PluginRegistry::default();
    registry.upsert(loaded);

    let save = world.save_file("a.fixture", "hello");
    let detected = detect_file(engine, &registry, &save).unwrap();
    let plugin = match detected.outcome {
        DetectOutcome::Open(plugin) => plugin,
        other => panic!("{other:?}"),
    };
    assert_eq!(plugin.manifest.id, "store-demo");
    open_document(engine, &plugin, &save, &world.services.tokens).unwrap();
}
