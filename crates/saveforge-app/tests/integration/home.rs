use std::path::PathBuf;

use saveforge_app::home::{raw_save_dir, resolve_save_dir};
use saveforge_core::manifest::PluginManifest;
use saveforge_core::paths::AppPaths;
use saveforge_core::platform::Os;
use saveforge_core::settings::AppSettings;
use saveforge_core::tokens::TokenEnv;

fn manifest() -> PluginManifest {
    PluginManifest::parse(
        r#"{ "id": "p", "name": "P", "version": "0.1.0", "author": "a",
             "apiVersion": "0.1", "minHostVersion": "0.1.0",
             "component": "plugin.wasm", "fileExtensions": [".x"],
             "defaultSaveDirectories": { "windows": "%APPDATA%/Win", "linux": "~/lin" } }"#,
    )
    .unwrap()
}

fn env(appdata: PathBuf) -> TokenEnv {
    TokenEnv {
        os: Os::current(),
        appdata: Some(appdata),
        local_appdata: None,
        home: None,
        temp: std::env::temp_dir(),
        steam: None,
    }
}

#[test]
fn override_beats_manifest_default() {
    let mut settings = AppSettings::defaults(&AppPaths::under("/d"));
    assert_eq!(
        raw_save_dir(&settings, &manifest(), Os::Windows).as_deref(),
        Some("%APPDATA%/Win")
    );
    assert_eq!(raw_save_dir(&settings, &manifest(), Os::MacOs), None);
    settings
        .plugin_save_directories
        .insert("p".to_owned(), "~/custom".to_owned());
    assert_eq!(
        raw_save_dir(&settings, &manifest(), Os::MacOs).as_deref(),
        Some("~/custom")
    );
}

#[test]
fn resolve_reports_why_a_folder_is_unusable() {
    let dir = tempfile::tempdir().unwrap();
    let e = env(dir.path().to_path_buf());
    assert_eq!(
        resolve_save_dir(None, &e),
        Err("No save folder set".to_owned())
    );
    assert_eq!(
        resolve_save_dir(Some("   "), &e),
        Err("No save folder set".to_owned())
    );
    assert!(resolve_save_dir(Some("relative/dir"), &e).is_err());
    let missing = resolve_save_dir(Some("%APPDATA%/missing"), &e).unwrap_err();
    assert!(missing.contains("does not exist"), "{missing}");
    std::fs::create_dir(dir.path().join("Saves")).unwrap();
    assert_eq!(
        resolve_save_dir(Some("%APPDATA%/Saves"), &e),
        Ok(dir.path().join("Saves"))
    );
}
