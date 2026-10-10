use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::paths::AppPaths;
use saveforge_core::settings::{AppSettings, RELEASE_CATALOG_URL, file_url};

fn defaults(dir: &Path) -> AppSettings {
    AppSettings::defaults(&AppPaths::under(dir))
}

#[test]
fn app_paths_layout() {
    let p = AppPaths::under("/data");
    assert_eq!(p.settings_file(), PathBuf::from("/data/settings.json"));
    assert_eq!(
        p.catalog_cache_file(),
        PathBuf::from("/data/catalog-cache.json")
    );
    assert_eq!(p.logs_dir(), PathBuf::from("/data/logs"));
    assert_eq!(p.crash_log(), PathBuf::from("/data/crash.log"));
    assert!(AppPaths::detect().is_some());
}

#[test]
fn defaults_match_spec() {
    let d = defaults(Path::new("/data"));
    assert_eq!(d.backup_directory, PathBuf::from("/data/backups"));
    assert!(d.check_updates_on_startup);
    assert_eq!(d.plugin_save_directories.len(), 0);
    if cfg!(debug_assertions) {
        assert!(d.plugin_directory.ends_with("target/dev-plugins"));
        assert_eq!(d.catalog_url, file_url(Path::new("/data/catalog.json")));
    } else {
        assert_eq!(d.plugin_directory, PathBuf::from("/data/plugins"));
        assert_eq!(d.catalog_url, RELEASE_CATALOG_URL);
    }
}

#[test]
fn missing_file_loads_defaults_without_warning() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = AppSettings::load(&dir.path().join("settings.json"), defaults(dir.path()));
    assert_eq!(loaded.settings, defaults(dir.path()));
    assert_eq!(loaded.warning, None);
}

#[test]
fn corrupt_file_loads_defaults_with_warning() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    let loaded = AppSettings::load(&path, defaults(dir.path()));
    assert_eq!(loaded.settings, defaults(dir.path()));
    assert!(loaded.warning.is_some());
}

#[test]
fn partial_file_merges_over_defaults_and_ignores_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{ "checkUpdatesOnStartup": false,
             "pluginSaveDirectories": { "planet-crafter": "%APPDATA%/X" },
             "someFutureKey": 1 }"#,
    )
    .unwrap();
    let loaded = AppSettings::load(&path, defaults(dir.path()));
    assert_eq!(loaded.warning, None);
    assert!(!loaded.settings.check_updates_on_startup);
    assert_eq!(
        loaded.settings.plugin_save_directories["planet-crafter"],
        "%APPDATA%/X"
    );
    assert_eq!(
        loaded.settings.backup_directory,
        defaults(dir.path()).backup_directory
    );
}

#[test]
fn wrong_type_for_one_key_keeps_other_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{ "checkUpdatesOnStartup": "yes",
             "pluginSaveDirectories": { "planet-crafter": "%APPDATA%/X" } }"#,
    )
    .unwrap();
    let loaded = AppSettings::load(&path, defaults(dir.path()));
    assert_eq!(
        loaded.settings.plugin_save_directories["planet-crafter"],
        "%APPDATA%/X"
    );
    assert!(loaded.settings.check_updates_on_startup);
    assert!(loaded.warning.unwrap().contains("checkUpdatesOnStartup"));
}

#[test]
fn unusable_file_is_preserved_as_bad_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    for content in ["{ not json", "[]", r#"{ "catalogUrl": 5 }"#] {
        fs::write(&path, content).unwrap();
        let loaded = AppSettings::load(&path, defaults(dir.path()));
        assert!(loaded.warning.is_some(), "{content}");
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json.bad")).unwrap(),
            content
        );
    }
}

#[test]
fn save_then_load_round_trips_with_camel_case_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut s = defaults(dir.path());
    s.catalog_url = "https://example.com/catalog.json".to_owned();
    s.plugin_save_directories
        .insert("planet-crafter".to_owned(), "~/saves".to_owned());
    s.save(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    for key in [
        "catalogUrl",
        "pluginDirectory",
        "backupDirectory",
        "checkUpdatesOnStartup",
        "pluginSaveDirectories",
    ] {
        assert!(text.contains(key), "missing {key} in {text}");
    }
    assert_eq!(
        AppSettings::load(&path, defaults(Path::new("/other"))).settings,
        s
    );
}

#[test]
fn save_creates_missing_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a/b/settings.json");
    defaults(dir.path()).save(&path).unwrap();
    assert!(path.is_file());
}

#[test]
fn file_url_formats() {
    assert_eq!(
        file_url(Path::new("/home/u/catalog.json")),
        "file:///home/u/catalog.json"
    );
    assert_eq!(
        file_url(Path::new(r"C:\Users\u\catalog.json")),
        "file:///C:/Users/u/catalog.json"
    );
}
