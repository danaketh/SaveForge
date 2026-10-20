use crate::common;

use std::fs;

use saveforge_core::CoreError;
use saveforge_host::{HOST_VERSION, HostError, HostLimits, PluginEngine, WASMTIME_VERSION};
use serde_json::Value;

#[test]
fn default_limits_match_spec() {
    let l = HostLimits::default();
    assert_eq!(l.ui_call.as_millis(), 250);
    assert_eq!(l.io_call.as_secs(), 10);
    assert_eq!(l.memory_bytes, 512 * 1024 * 1024);
    assert_eq!(HOST_VERSION.to_string(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn loads_fixture() {
    let root = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "test-plugin", "0.1.0", |_| {});
    let plugin = common::engine().load(&dir).unwrap();
    assert_eq!(plugin.manifest.id, "test-plugin");
    assert_eq!(plugin.dir, dir);
}

#[test]
fn rejects_incompatible_api_and_host_versions() {
    let root = tempfile::tempdir().unwrap();
    let engine = common::engine();

    let api = common::install_fixture(root.path(), "api", "0.1.0", |m| {
        m["apiVersion"] = Value::from("0.2");
    });
    assert!(matches!(
        engine.load(&api),
        Err(HostError::Core(CoreError::Incompatible(_)))
    ));

    let host = common::install_fixture(root.path(), "host", "0.1.0", |m| {
        m["minHostVersion"] = Value::from("99.0.0");
    });
    assert!(matches!(
        engine.load(&host),
        Err(HostError::Core(CoreError::Incompatible(_)))
    ));
}

#[test]
fn missing_or_invalid_component_is_compile_error() {
    let root = tempfile::tempdir().unwrap();
    let engine = common::engine();

    let missing = common::install_fixture(root.path(), "missing", "0.1.0", |m| {
        m["component"] = Value::from("nope.wasm");
    });
    assert!(matches!(
        engine.load(&missing),
        Err(HostError::Compile { .. })
    ));

    let garbage = common::install_fixture(root.path(), "garbage", "0.1.0", |_| {});
    fs::write(garbage.join("plugin.wasm"), b"not wasm").unwrap();
    assert!(matches!(
        engine.load(&garbage),
        Err(HostError::Compile { .. })
    ));
}

#[test]
fn component_without_editor_export_is_compile_error() {
    let root = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "empty", "0.1.0", |_| {});
    // A valid but empty component: magic, component-model version/layer.
    fs::write(
        dir.join("plugin.wasm"),
        [0u8, 0x61, 0x73, 0x6d, 0x0d, 0, 1, 0],
    )
    .unwrap();
    let err = common::engine().load(&dir).unwrap_err();
    assert!(matches!(err, HostError::Compile { .. }), "{err:?}");
}

#[test]
fn compile_cache_is_written() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "test-plugin", "0.1.0", |_| {});
    let engine = PluginEngine::new(common::test_limits(), Some(cache.path())).unwrap();
    engine.load(&dir).unwrap();
    assert!(fs::read_dir(cache.path()).unwrap().count() > 0);
}

#[test]
fn wasmtime_version_matches_the_lockfile() {
    let lock =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock")).unwrap();
    // `lines()` so a CRLF checkout (Windows CI) matches too.
    let block = lock
        .split("[[package]]")
        .find(|b| b.lines().any(|l| l == "name = \"wasmtime\""))
        .unwrap();
    let version = format!("version = \"{WASMTIME_VERSION}\"");
    assert!(block.lines().any(|l| l == version), "{block}");
}
