#![allow(dead_code)] // each test file uses a subset

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use saveforge_host::{HostLimits, PluginEngine};
use serde_json::Value;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The fixture component built by `cargo xtask plugins`.
pub fn fixture_wasm() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace().join("target"), PathBuf::from);
    let wasm = target.join("fixtures").join("test-plugin.wasm");
    assert!(
        wasm.is_file(),
        "fixture not built: run `cargo xtask plugins` first ({})",
        wasm.display()
    );
    wasm
}

/// Limits short enough to keep fault tests fast.
pub fn test_limits() -> HostLimits {
    HostLimits {
        ui_call: Duration::from_millis(200),
        io_call: Duration::from_secs(1),
        memory_bytes: 64 * 1024 * 1024,
    }
}

pub fn engine() -> PluginEngine {
    PluginEngine::new(test_limits(), None).expect("engine")
}

/// Installs the fixture as `{root}/{id}/{version}/` with its manifest's `id`
/// and `version` rewritten, then applies `edit` to the manifest JSON.
pub fn install_fixture(
    root: &Path,
    id: &str,
    version: &str,
    edit: impl FnOnce(&mut Value),
) -> PathBuf {
    let source = workspace().join("fixtures/test-plugin/manifest.json");
    let mut manifest: Value = serde_json::from_str(&fs::read_to_string(source).unwrap()).unwrap();
    manifest["id"] = Value::from(id);
    manifest["version"] = Value::from(version);
    edit(&mut manifest);

    let dir = root.join(id).join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    fs::copy(fixture_wasm(), dir.join("plugin.wasm")).unwrap();
    dir
}
