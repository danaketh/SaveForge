#![allow(dead_code)] // each test file uses a subset

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use saveforge_core::platform::Os;
use saveforge_core::tokens::TokenEnv;
use saveforge_host::{HostLimits, PluginEngine, PluginRegistry};

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

/// Installs the fixture plugin under `{plugins}/{id}/0.1.0/`.
pub fn install_fixture(plugins: &Path, id: &str) {
    let source = workspace().join("fixtures/test-plugin/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(source).unwrap()).unwrap();
    manifest["id"] = serde_json::Value::from(id);
    let dir = plugins.join(id).join("0.1.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    fs::copy(fixture_wasm(), dir.join("plugin.wasm")).unwrap();
}

pub fn engine() -> Arc<PluginEngine> {
    Arc::new(
        PluginEngine::new(
            HostLimits {
                ui_call: Duration::from_millis(200),
                io_call: Duration::from_secs(1),
                memory_bytes: 64 * 1024 * 1024,
            },
            None,
        )
        .unwrap(),
    )
}

pub struct Services {
    pub engine: Arc<PluginEngine>,
    pub tokens: Arc<TokenEnv>,
}

/// A temp dir with the fixture installed as `test-plugin`.
pub struct World {
    pub dir: tempfile::TempDir,
    pub services: Services,
}

impl World {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        install_fixture(&dir.path().join("plugins"), "test-plugin");
        let services = Services {
            engine: engine(),
            tokens: Arc::new(TokenEnv {
                os: Os::current(),
                appdata: Some(dir.path().to_path_buf()),
                local_appdata: None,
                home: None,
                temp: std::env::temp_dir(),
                steam: None,
            }),
        };
        Self { dir, services }
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.dir.path().join("plugins")
    }

    pub fn registry(&self) -> PluginRegistry {
        PluginRegistry::scan(&self.services.engine, &self.plugins_dir())
    }

    /// Writes a save file into `<dir>/saves/` and returns its path.
    pub fn save_file(&self, name: &str, content: &str) -> PathBuf {
        let saves = self.dir.path().join("saves");
        fs::create_dir_all(&saves).unwrap();
        let path = saves.join(name);
        fs::write(&path, content).unwrap();
        path
    }
}
