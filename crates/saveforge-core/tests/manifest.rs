use std::path::Path;

use saveforge_core::CoreError;
use saveforge_core::manifest::{
    AUX_MAX_BYTES_CAP, DEFAULT_AUX_MAX_BYTES, PluginManifest, is_safe_relative_path, is_valid_id,
};
use saveforge_core::platform::Os;
use semver::Version;
use serde_json::{Value, json};

const VALID: &str = r#"{
  "id": "planet-crafter",
  "name": "Planet Crafter Save Editor",
  "version": "1.0.0",
  "author": "SaveForge Team",
  "description": "Edit saves for Planet Crafter.",
  "apiVersion": "0.1",
  "minHostVersion": "1.0.0",
  "maxHostVersion": null,
  "component": "plugin.wasm",
  "icon": "icon.png",
  "fileExtensions": [".json"],
  "defaultSaveDirectories": {
    "windows": "%APPDATA%/MijuGames/The Planet Crafter/Saves",
    "linux": "{steam_compat:1284190}/AppData/Roaming/MijuGames/The Planet Crafter/Saves",
    "macos": null
  },
  "auxFiles": [
    { "id": "localization", "optional": true,
      "candidates": ["{steam_app:1284190}/Planet Crafter_Data/Resources/LocalizationSpaceCrafter.txt"] }
  ]
}"#;

fn with(field: &str, value: Value) -> String {
    let mut v: Value = serde_json::from_str(VALID).unwrap();
    v[field] = value;
    v.to_string()
}

fn without(field: &str) -> String {
    let mut v: Value = serde_json::from_str(VALID).unwrap();
    v.as_object_mut().unwrap().remove(field);
    v.to_string()
}

fn assert_manifest_err(json: &str) {
    match PluginManifest::parse(json) {
        Err(CoreError::Manifest(_)) => {}
        other => panic!("expected Manifest error, got {other:?}"),
    }
}

fn v(s: &str) -> Version {
    Version::parse(s).unwrap()
}

#[test]
fn parses_valid_manifest() {
    let m = PluginManifest::parse(VALID).unwrap();
    assert_eq!(m.id, "planet-crafter");
    assert_eq!(m.version, v("1.0.0"));
    assert_eq!(m.api_version, "0.1");
    assert_eq!(m.max_host_version, None);
    assert_eq!(m.component, "plugin.wasm");
    assert_eq!(m.icon.as_deref(), Some("icon.png"));
    assert_eq!(
        m.default_save_directories.for_os(Os::Windows),
        Some("%APPDATA%/MijuGames/The Planet Crafter/Saves")
    );
    assert_eq!(m.default_save_directories.for_os(Os::MacOs), None);
    assert_eq!(m.aux_files.len(), 1);
    assert!(m.aux_files[0].optional);
    assert_eq!(m.aux_files[0].max_bytes, DEFAULT_AUX_MAX_BYTES);
}

#[test]
fn optional_fields_default() {
    let mut v: Value = serde_json::from_str(VALID).unwrap();
    let obj = v.as_object_mut().unwrap();
    for key in [
        "description",
        "maxHostVersion",
        "icon",
        "defaultSaveDirectories",
        "auxFiles",
    ] {
        obj.remove(key);
    }
    let m = PluginManifest::parse(&v.to_string()).unwrap();
    assert_eq!(m.description, "");
    assert_eq!(m.icon, None);
    assert_eq!(m.default_save_directories.for_os(Os::Linux), None);
    assert_eq!(m.aux_files, vec![]);
}

#[test]
fn unknown_fields_are_ignored() {
    PluginManifest::parse(&with("futureField", json!(42))).unwrap();
}

#[test]
fn missing_required_field_is_manifest_error() {
    assert_manifest_err(&without("component"));
    assert_manifest_err(&with("version", json!("not-semver")));
}

#[test]
fn id_pattern() {
    for ok in ["planet-crafter", "a1", "x"] {
        assert!(is_valid_id(ok), "{ok}");
    }
    for bad in [
        "",
        "Planet",
        "planet--crafter",
        "-x",
        "x-",
        "planet_crafter",
        "pl anet",
    ] {
        assert!(!is_valid_id(bad), "{bad}");
        assert_manifest_err(&with("id", json!(bad)));
    }
}

#[test]
fn rejects_unsafe_relative_paths() {
    for ok in ["plugin.wasm", "bin/plugin.wasm", "bin\\plugin.wasm"] {
        assert!(is_safe_relative_path(ok), "{ok}");
    }
    for bad in [
        "",
        "../evil.wasm",
        "..\\evil.wasm",
        "sub/../../x.wasm",
        "/abs.wasm",
        "\\abs.wasm",
        "C:\\evil.wasm",
        "C:evil.wasm",
    ] {
        assert!(!is_safe_relative_path(bad), "{bad}");
        assert_manifest_err(&with("component", json!(bad)));
        assert_manifest_err(&with("icon", json!(bad)));
    }
}

#[test]
fn file_extensions_rules() {
    assert_manifest_err(&with("fileExtensions", json!([])));
    assert_manifest_err(&with("fileExtensions", json!(["json"])));
    assert_manifest_err(&with("fileExtensions", json!(["."])));
}

#[test]
fn aux_file_rules() {
    let aux = |items: Value| with("auxFiles", items);
    let one = |extra: Value| {
        let mut base = json!({ "id": "a", "candidates": ["x.txt"] });
        for (k, val) in extra.as_object().unwrap() {
            base[k] = val.clone();
        }
        base
    };
    assert_manifest_err(&aux(json!([one(json!({})), one(json!({}))])));
    assert_manifest_err(&aux(json!([one(json!({ "maxBytes": 0 }))])));
    assert_manifest_err(&aux(json!([one(
        json!({ "maxBytes": AUX_MAX_BYTES_CAP + 1 })
    )])));
    assert_manifest_err(&aux(json!([one(json!({ "candidates": [] }))])));
    PluginManifest::parse(&aux(json!([one(json!({ "maxBytes": AUX_MAX_BYTES_CAP }))]))).unwrap();
}

#[test]
fn api_version_must_be_major_minor() {
    for bad in ["1", "0.1.0", "x.y", ""] {
        assert_manifest_err(&with("apiVersion", json!(bad)));
    }
}

#[test]
fn compat_checks_host_range() {
    let api = v("0.1.0");
    let m = PluginManifest::parse(&with("maxHostVersion", json!("1.5.0"))).unwrap();
    m.check_compat(&v("1.0.0"), &api).unwrap();
    m.check_compat(&v("1.5.0"), &api).unwrap();
    assert!(matches!(
        m.check_compat(&v("1.6.0"), &api),
        Err(CoreError::Incompatible(_))
    ));
    assert!(matches!(
        m.check_compat(&v("0.9.0"), &api),
        Err(CoreError::Incompatible(_))
    ));
}

#[test]
fn compat_checks_api_version() {
    let host = v("1.0.0");
    let pre = PluginManifest::parse(&with("apiVersion", json!("0.2"))).unwrap();
    assert!(matches!(
        pre.check_compat(&host, &v("0.1.0")),
        Err(CoreError::Incompatible(_))
    ));
    assert!(matches!(
        pre.check_compat(&host, &v("0.3.0")),
        Err(CoreError::Incompatible(_))
    ));
    pre.check_compat(&host, &v("0.2.5")).unwrap();

    let stable = PluginManifest::parse(&with("apiVersion", json!("1.2"))).unwrap();
    stable.check_compat(&host, &v("1.2.0")).unwrap();
    stable.check_compat(&host, &v("1.3.0")).unwrap();
    assert!(matches!(
        stable.check_compat(&host, &v("1.1.0")),
        Err(CoreError::Incompatible(_))
    ));
    assert!(matches!(
        stable.check_compat(&host, &v("2.0.0")),
        Err(CoreError::Incompatible(_))
    ));
}

#[test]
fn handles_extension_is_case_insensitive() {
    let m = PluginManifest::parse(VALID).unwrap();
    assert!(m.handles_extension(Path::new("Standard-1.json")));
    assert!(m.handles_extension(Path::new("SAVE.JSON")));
    assert!(!m.handles_extension(Path::new("save.json.bak")));
    assert!(!m.handles_extension(Path::new("noext")));
}

#[test]
fn load_reads_manifest_from_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("manifest.json"), VALID).unwrap();
    assert_eq!(
        PluginManifest::load(dir.path()).unwrap().id,
        "planet-crafter"
    );

    let empty = tempfile::tempdir().unwrap();
    assert!(matches!(
        PluginManifest::load(empty.path()),
        Err(CoreError::Io { .. })
    ));
}
