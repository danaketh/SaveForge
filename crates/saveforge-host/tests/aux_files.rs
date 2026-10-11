use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::manifest::PluginManifest;
use saveforge_core::platform::Os;
use saveforge_core::tokens::TokenEnv;
use saveforge_host::{HostError, resolve_aux_files};
use serde_json::json;

fn env(appdata: &Path) -> TokenEnv {
    TokenEnv {
        os: Os::current(),
        appdata: Some(appdata.to_path_buf()),
        local_appdata: None,
        home: None,
        temp: std::env::temp_dir(),
        steam: None,
    }
}

fn manifest(aux: serde_json::Value) -> PluginManifest {
    let mut value = json!({
        "id": "p", "name": "P", "version": "0.1.0", "author": "a",
        "apiVersion": "0.1", "minHostVersion": "0.1.0",
        "component": "plugin.wasm", "fileExtensions": [".x"]
    });
    value["auxFiles"] = aux;
    PluginManifest::parse(&value.to_string()).unwrap()
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn first_existing_candidate_wins() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "second.txt", b"two");
    write(dir.path(), "third.txt", b"three");
    let m = manifest(json!([{ "id": "loc",
        "candidates": ["%APPDATA%/first.txt", "%APPDATA%/second.txt", "%APPDATA%/third.txt"] }]));
    let files = resolve_aux_files(&m, &env(dir.path())).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].id, "loc");
    assert_eq!(files[0].bytes, b"two");
}

#[test]
fn optional_missing_is_omitted_required_missing_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let optional =
        manifest(json!([{ "id": "opt", "optional": true, "candidates": ["%APPDATA%/none.txt"] }]));
    assert_eq!(
        resolve_aux_files(&optional, &env(dir.path())).unwrap(),
        vec![]
    );

    let required = manifest(json!([{ "id": "req", "candidates": ["%APPDATA%/none.txt"] }]));
    let err = resolve_aux_files(&required, &env(dir.path())).unwrap_err();
    assert!(
        matches!(err, HostError::MissingAux(ref id) if id == "req"),
        "{err:?}"
    );
}

#[test]
fn oversized_candidate_falls_through() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "big.txt", &[b'x'; 10]);
    write(dir.path(), "small.txt", b"ok");
    let m = manifest(json!([{ "id": "loc", "maxBytes": 5,
        "candidates": ["%APPDATA%/big.txt", "%APPDATA%/small.txt"] }]));
    assert_eq!(
        resolve_aux_files(&m, &env(dir.path())).unwrap()[0].bytes,
        b"ok"
    );

    let only_big =
        manifest(json!([{ "id": "loc", "maxBytes": 5, "candidates": ["%APPDATA%/big.txt"] }]));
    assert!(matches!(
        resolve_aux_files(&only_big, &env(dir.path())),
        Err(HostError::MissingAux(_))
    ));
}

#[test]
fn unresolvable_and_directory_candidates_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("adir")).unwrap();
    write(dir.path(), "real.txt", b"r");
    let m = manifest(json!([{ "id": "loc",
        "candidates": ["{steam_app:1}/x.txt", "relative.txt", "%APPDATA%/adir", "%APPDATA%/real.txt"] }]));
    assert_eq!(
        resolve_aux_files(&m, &env(dir.path())).unwrap()[0].bytes,
        b"r"
    );
}

#[test]
fn preserves_spec_order_for_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", b"A");
    write(dir.path(), "b.txt", b"B");
    let m = manifest(json!([
        { "id": "b", "candidates": ["%APPDATA%/b.txt"] },
        { "id": "a", "candidates": ["%APPDATA%/a.txt"] }
    ]));
    let ids: Vec<_> = resolve_aux_files(&m, &env(dir.path()))
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(ids, ["b", "a"]);
}
