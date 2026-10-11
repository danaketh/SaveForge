use std::fs;
use std::path::Path;

use saveforge_core::manifest::PluginManifest;
use xtask::{
    PluginCrate, PluginKind, dev_plugin_dir, find_plugin_crates, fixture_wasm, install,
    wasm_artifact,
};

const MANIFEST: &str = r#"{
  "id": "demo", "name": "Demo", "version": "1.2.3", "author": "a",
  "apiVersion": "0.1", "minHostVersion": "0.1.0",
  "component": "plugin.wasm", "icon": "icon.png", "fileExtensions": [".sav"]
}"#;

fn make_crate(root: &Path, sub: &str, name: &str, manifest: bool) {
    let dir = root.join(sub).join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
    if manifest {
        fs::write(dir.join("manifest.json"), MANIFEST).unwrap();
        fs::write(dir.join("icon.png"), b"png").unwrap();
    }
}

#[test]
fn finds_plugin_and_fixture_crates_with_manifests() {
    let root = tempfile::tempdir().unwrap();
    make_crate(root.path(), "plugins", "demo", true);
    make_crate(root.path(), "plugins", "no-manifest", false);
    make_crate(root.path(), "fixtures", "test-plugin", true);
    let found = find_plugin_crates(root.path());
    let summary: Vec<_> = found.iter().map(|c| (c.kind, c.package.as_str())).collect();
    assert_eq!(
        summary,
        [
            (PluginKind::Plugin, "demo"),
            (PluginKind::Fixture, "test-plugin")
        ]
    );
}

#[test]
fn output_paths() {
    let target = Path::new("/t");
    assert_eq!(
        wasm_artifact(target, "planet-crafter"),
        Path::new("/t/wasm32-wasip2/release/planet_crafter.wasm")
    );
    assert_eq!(
        fixture_wasm(target, "test-plugin"),
        Path::new("/t/fixtures/test-plugin.wasm")
    );
    let manifest = PluginManifest::parse(MANIFEST).unwrap();
    assert_eq!(
        dev_plugin_dir(target, &manifest),
        Path::new("/t/dev-plugins/demo/1.2.3")
    );
}

#[test]
fn installs_plugin_with_manifest_component_and_icon() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    make_crate(root.path(), "plugins", "demo", true);
    let wasm = wasm_artifact(&target, "demo");
    fs::create_dir_all(wasm.parent().unwrap()).unwrap();
    fs::write(&wasm, b"\0asm").unwrap();
    let krate = PluginCrate {
        kind: PluginKind::Plugin,
        package: "demo".to_owned(),
        dir: root.path().join("plugins/demo"),
    };

    let dest = install(&target, &krate).unwrap();
    assert_eq!(dest, target.join("dev-plugins/demo/1.2.3"));
    assert_eq!(fs::read(dest.join("plugin.wasm")).unwrap(), b"\0asm");
    assert_eq!(fs::read(dest.join("icon.png")).unwrap(), b"png");
    assert!(dest.join("manifest.json").is_file());

    // Re-install replaces stale files.
    fs::write(dest.join("stale.txt"), b"x").unwrap();
    install(&target, &krate).unwrap();
    assert!(!dest.join("stale.txt").exists());
}

#[test]
fn installs_fixture_wasm_only() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    make_crate(root.path(), "fixtures", "test-plugin", true);
    let wasm = wasm_artifact(&target, "test-plugin");
    fs::create_dir_all(wasm.parent().unwrap()).unwrap();
    fs::write(&wasm, b"\0asm").unwrap();
    let krate = PluginCrate {
        kind: PluginKind::Fixture,
        package: "test-plugin".to_owned(),
        dir: root.path().join("fixtures/test-plugin"),
    };
    let dest = install(&target, &krate).unwrap();
    assert_eq!(dest, target.join("fixtures/test-plugin.wasm"));
    assert_eq!(fs::read(dest).unwrap(), b"\0asm");
}

#[test]
fn install_fails_when_build_output_missing() {
    let root = tempfile::tempdir().unwrap();
    make_crate(root.path(), "plugins", "demo", true);
    let krate = PluginCrate {
        kind: PluginKind::Plugin,
        package: "demo".to_owned(),
        dir: root.path().join("plugins/demo"),
    };
    let err = install(&root.path().join("target"), &krate).unwrap_err();
    assert!(err.to_string().contains("build output missing"), "{err}");
}

#[test]
fn workspace_root_prefers_runtime_manifest_dir() {
    use std::ffi::OsString;
    use std::path::PathBuf;
    assert_eq!(
        xtask::workspace_root(
            Some(OsString::from("/checkout-b/xtask")),
            "/checkout-a/xtask"
        ),
        PathBuf::from("/checkout-b")
    );
    assert_eq!(
        xtask::workspace_root(None, "/checkout-a/xtask"),
        PathBuf::from("/checkout-a")
    );
}

#[test]
fn relative_target_dir_resolves_against_workspace_root() {
    use std::ffi::OsString;
    use std::path::PathBuf;
    let root = Path::new("/ws");
    assert_eq!(xtask::target_dir(root, None), PathBuf::from("/ws/target"));
    assert_eq!(
        xtask::target_dir(root, Some(OsString::from("out"))),
        PathBuf::from("/ws/out")
    );
    let absolute = std::env::temp_dir().join("tgt");
    assert_eq!(
        xtask::target_dir(root, Some(absolute.clone().into_os_string())),
        absolute
    );
}
