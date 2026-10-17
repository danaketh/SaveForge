use std::fs;
use std::path::Path;

use saveforge_core::settings::file_url;
use saveforge_store::StoreError;
use saveforge_store::cache::{CatalogSource, load_catalog};
use saveforge_store::transport::read_url;

const CATALOG: &str = r#"{"schemaVersion":"1","plugins":[{"id":"demo","name":"Demo","author":"Me",
"latestVersion":"1.0.0","minHostVersion":"0.1.0","packageUrl":"https://example.com/demo.zip",
"sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}]}"#;

const CACHED: &str = r#"{"schemaVersion":"1","plugins":[{"id":"cached","name":"Cached","author":"Me",
"latestVersion":"1.0.0","minHostVersion":"0.1.0","packageUrl":"https://example.com/cached.zip",
"sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}]}"#;

type Read = (Result<u64, StoreError>, Vec<u8>, Vec<(u64, Option<u64>)>);

fn read(url: &str, limit: u64) -> Read {
    let mut sink = Vec::new();
    let mut calls = Vec::new();
    let result = read_url(url, limit, &mut sink, &mut |done, total| {
        calls.push((done, total));
    });
    (result, sink, calls)
}

fn ids(path: &Path, cache: &Path) -> (CatalogSource, Vec<String>) {
    let load = load_catalog(&file_url(path), cache).unwrap();
    let ids = load.catalog.plugins.iter().map(|e| e.id.clone()).collect();
    (load.source, ids)
}

#[test]
fn file_urls_are_read_with_progress() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("with space").join("data.bin");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
    fs::write(&path, &bytes).unwrap();

    let (result, sink, calls) = read(&file_url(&path), 1_000_000);
    assert_eq!(result.unwrap(), 10_000);
    assert_eq!(sink, bytes);
    assert_eq!(calls.last(), Some(&(10_000, Some(10_000))));
}

#[test]
fn http_urls_are_rejected() {
    let (result, _, _) = read("http://example.com/c.json", 1_000);
    let err = result.unwrap_err();
    assert!(matches!(err, StoreError::UnsupportedUrl(_)), "{err:?}");
    assert_eq!(
        err.to_string(),
        "only https:// and file:// URLs are supported: http://example.com/c.json"
    );
}

#[test]
fn oversized_downloads_stop_at_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.bin");
    fs::write(&path, vec![7u8; 2_000]).unwrap();
    let (result, sink, _) = read(&file_url(&path), 1_000);
    assert!(
        matches!(result, Err(StoreError::TooLarge { limit: 1_000, .. })),
        "{result:?}"
    );
    assert!(sink.len() <= 1_000, "{}", sink.len());
}

#[test]
fn missing_file_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let (result, _, _) = read(&file_url(&dir.path().join("missing.json")), 1_000);
    assert!(matches!(result, Err(StoreError::Io { .. })), "{result:?}");
}

#[test]
fn online_catalog_is_cached() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let cache = dir.path().join("cache").join("catalog-cache.json");
    fs::write(&path, CATALOG).unwrap();
    assert_eq!(
        ids(&path, &cache),
        (CatalogSource::Online, vec!["demo".into()])
    );
    assert_eq!(fs::read_to_string(&cache).unwrap(), CATALOG);
}

#[test]
fn fetch_failure_falls_back_to_cache() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("catalog-cache.json");
    fs::write(&cache, CACHED).unwrap();
    assert_eq!(
        ids(&dir.path().join("missing.json"), &cache),
        (CatalogSource::Cached, vec!["cached".into()])
    );
}

#[test]
fn no_cache_returns_the_fetch_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.json");
    let result = load_catalog(&file_url(&missing), &dir.path().join("cache.json"));
    match result {
        Err(StoreError::Io { path, .. }) => assert_eq!(path, missing),
        other => panic!("{other:?}"),
    }
}

#[test]
fn invalid_online_catalog_falls_back_to_cache() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.json");
    let cache = dir.path().join("catalog-cache.json");
    fs::write(&path, "not json").unwrap();
    fs::write(&cache, CACHED).unwrap();
    assert_eq!(
        ids(&path, &cache),
        (CatalogSource::Cached, vec!["cached".into()])
    );
    assert_eq!(fs::read_to_string(&cache).unwrap(), CACHED);
}
