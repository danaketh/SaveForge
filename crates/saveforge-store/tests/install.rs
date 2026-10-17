mod common;

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use common::{entry_for, manifest_json, package, sha256_hex};
use saveforge_store::install::{extract_zip, install, remove, remove_version};
use saveforge_store::{CatalogEntry, StoreError};
use semver::Version;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

struct World {
    tmp: TempDir,
}

impl World {
    fn new() -> Self {
        Self {
            tmp: tempfile::tempdir().unwrap(),
        }
    }

    fn plugins(&self) -> PathBuf {
        self.tmp.path().join("plugins")
    }

    /// Writes `bytes` as the package and returns a matching catalog entry.
    fn publish(&self, id: &str, version: &str, bytes: &[u8]) -> CatalogEntry {
        let path = self.tmp.path().join("pkg.zip");
        fs::write(&path, bytes).unwrap();
        entry_for(id, version, &path, &sha256_hex(bytes))
    }

    fn demo(&self, version: &str, wasm: &[u8]) -> CatalogEntry {
        let manifest = manifest_json("demo", version, "0.1.0");
        self.publish(
            "demo",
            version,
            &package(&[
                ("manifest.json", manifest.as_bytes()),
                ("plugin.wasm", wasm),
            ]),
        )
    }

    fn install(&self, entry: &CatalogEntry) -> Result<PathBuf, StoreError> {
        install(entry, &self.plugins(), &mut |_, _| {})
    }

    fn assert_staging_empty(&self) {
        let staging = self.plugins().join(".staging");
        if staging.exists() {
            assert_eq!(
                fs::read_dir(&staging).unwrap().count(),
                0,
                "{}",
                staging.display()
            );
        }
    }
}

fn demo_dir(plugins: &Path, version: &str) -> PathBuf {
    plugins.join("demo").join(version)
}

#[test]
fn installs_into_id_and_version() {
    let world = World::new();
    let entry = world.demo("1.0.0", b"wasm");
    let mut calls = 0;
    let dir = install(&entry, &world.plugins(), &mut |_, _| calls += 1).unwrap();
    assert_eq!(dir, demo_dir(&world.plugins(), "1.0.0"));
    assert!(dir.join("manifest.json").is_file());
    assert_eq!(fs::read(dir.join("plugin.wasm")).unwrap(), b"wasm");
    assert_ne!(calls, 0);
    world.assert_staging_empty();
}

#[test]
fn checksum_mismatch_is_rejected() {
    let world = World::new();
    let mut entry = world.demo("1.0.0", b"wasm");
    entry.sha256 = "0".repeat(64);
    let err = world.install(&entry).unwrap_err();
    assert!(matches!(err, StoreError::HashMismatch { .. }), "{err:?}");
    assert!(!world.plugins().join("demo").exists());
    world.assert_staging_empty();
}

#[test]
fn zip_slip_entries_are_rejected() {
    let world = World::new();
    let manifest = manifest_json("demo", "1.0.0", "0.1.0");
    for name in ["../evil.txt", "a/../../evil.txt"] {
        let entry = world.publish(
            "demo",
            "1.0.0",
            &package(&[("manifest.json", manifest.as_bytes()), (name, b"evil")]),
        );
        let err = world.install(&entry).unwrap_err();
        assert!(
            matches!(err, StoreError::UnsafePackage(_)),
            "{name}: {err:?}"
        );
        assert!(!world.tmp.path().join("evil.txt").exists());
        assert!(!world.plugins().join("evil.txt").exists());
        assert!(!world.plugins().join("demo").exists());
        world.assert_staging_empty();
    }
}

#[test]
fn absolute_paths_are_rejected() {
    let world = World::new();
    let manifest = manifest_json("demo", "1.0.0", "0.1.0");
    let mut names = vec!["/abs.txt", "\\abs.txt"];
    if cfg!(windows) {
        names.push("C:/abs.txt");
    }
    for name in names {
        let entry = world.publish(
            "demo",
            "1.0.0",
            &package(&[("manifest.json", manifest.as_bytes()), (name, b"abs")]),
        );
        let err = world.install(&entry).unwrap_err();
        assert!(
            matches!(err, StoreError::UnsafePackage(_)),
            "{name}: {err:?}"
        );
        world.assert_staging_empty();
    }
}

#[test]
fn symlink_entries_are_rejected() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("manifest.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(manifest_json("demo", "1.0.0", "0.1.0").as_bytes())
        .unwrap();
    zip.add_symlink("plugin.wasm", "/etc/passwd", SimpleFileOptions::default())
        .unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let dest = tempfile::tempdir().unwrap();
    let err = extract_zip(Cursor::new(bytes), dest.path(), 1_000_000).unwrap_err();
    assert!(matches!(err, StoreError::UnsafePackage(_)), "{err:?}");
}

#[test]
fn oversized_extraction_is_rejected() {
    let bytes = package(&[("big.bin", &[1u8; 2_000])]);
    let dest = tempfile::tempdir().unwrap();
    let err = extract_zip(Cursor::new(bytes), dest.path(), 1_000).unwrap_err();
    assert!(matches!(err, StoreError::PackageTooLarge), "{err:?}");
}

#[test]
fn corrupt_zip_is_rejected() {
    let world = World::new();
    let entry = world.publish("demo", "1.0.0", b"not a zip");
    let err = world.install(&entry).unwrap_err();
    assert!(matches!(err, StoreError::BadPackage(_)), "{err:?}");
    world.assert_staging_empty();
}

#[test]
fn manifest_mismatch_is_rejected() {
    let world = World::new();
    let manifest = manifest_json("demo", "1.0.1", "0.1.0");
    let entry = world.publish(
        "demo",
        "1.0.0",
        &package(&[
            ("manifest.json", manifest.as_bytes()),
            ("plugin.wasm", b"w"),
        ]),
    );
    match world.install(&entry).unwrap_err() {
        StoreError::ManifestMismatch { expected, found } => {
            assert_eq!(expected, "demo 1.0.0");
            assert_eq!(found, "demo 1.0.1");
        }
        other => panic!("{other:?}"),
    }
    assert!(!world.plugins().join("demo").exists());
    world.assert_staging_empty();
}

#[test]
fn incompatible_package_is_rejected() {
    let world = World::new();
    let manifest = manifest_json("demo", "1.0.0", "99.0.0");
    let entry = world.publish(
        "demo",
        "1.0.0",
        &package(&[
            ("manifest.json", manifest.as_bytes()),
            ("plugin.wasm", b"w"),
        ]),
    );
    let err = world.install(&entry).unwrap_err();
    assert!(matches!(err, StoreError::Core(_)), "{err:?}");
    assert!(err.to_string().contains("incompatible plugin"), "{err}");
    assert!(!world.plugins().join("demo").exists());
    world.assert_staging_empty();
}

#[test]
fn reinstalling_the_same_version_replaces_it() {
    let world = World::new();
    world.install(&world.demo("1.0.0", b"first")).unwrap();
    let dir = world.install(&world.demo("1.0.0", b"second")).unwrap();
    assert_eq!(fs::read(dir.join("plugin.wasm")).unwrap(), b"second");
    world.assert_staging_empty();
}

#[test]
fn failed_update_keeps_the_installed_version() {
    let world = World::new();
    world.install(&world.demo("1.0.0", b"old")).unwrap();
    let mut update = world.demo("1.1.0", b"new");
    update.sha256 = "f".repeat(64);
    world.install(&update).unwrap_err();
    let old = demo_dir(&world.plugins(), "1.0.0");
    assert_eq!(fs::read(old.join("plugin.wasm")).unwrap(), b"old");
    assert!(!demo_dir(&world.plugins(), "1.1.0").exists());
    world.assert_staging_empty();
}

#[test]
fn remove_deletes_the_plugin_folder() {
    let world = World::new();
    world.install(&world.demo("1.0.0", b"w")).unwrap();
    remove(&world.plugins(), "demo").unwrap();
    assert!(!world.plugins().join("demo").exists());
    remove(&world.plugins(), "demo").unwrap();
}

#[test]
fn remove_rejects_bad_ids() {
    let world = World::new();
    let err = remove(&world.plugins(), "../x").unwrap_err();
    assert!(matches!(err, StoreError::InvalidId(_)), "{err:?}");
    let err = remove_version(&world.plugins(), "..", &Version::new(1, 0, 0)).unwrap_err();
    assert!(matches!(err, StoreError::InvalidId(_)), "{err:?}");

    world.install(&world.demo("1.0.0", b"a")).unwrap();
    world.install(&world.demo("1.1.0", b"b")).unwrap();
    remove_version(&world.plugins(), "demo", &Version::new(1, 0, 0)).unwrap();
    assert!(!demo_dir(&world.plugins(), "1.0.0").exists());
    assert!(demo_dir(&world.plugins(), "1.1.0").is_dir());
}
