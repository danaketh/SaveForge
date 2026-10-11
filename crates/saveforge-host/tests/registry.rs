mod common;

use std::fs;

use saveforge_host::{MAX_FAULTS_PER_SESSION, PluginEntry, PluginRegistry};
use serde_json::Value;

fn failed_reason(registry: &PluginRegistry, id: &str) -> String {
    match registry.get(id) {
        Some(PluginEntry::Failed { reason, .. }) => reason.clone(),
        other => panic!("expected Failed for {id}, got {other:?}"),
    }
}

#[test]
fn missing_plugins_dir_is_empty() {
    let root = tempfile::tempdir().unwrap();
    let registry = PluginRegistry::scan(&common::engine(), &root.path().join("nope"));
    assert_eq!(registry.entries().len(), 0);
}

#[test]
fn newest_version_wins() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix", "0.1.0", |_| {});
    let newest = common::install_fixture(root.path(), "fix", "0.10.0", |_| {});
    common::install_fixture(root.path(), "fix", "0.9.0", |_| {});
    let registry = PluginRegistry::scan(&common::engine(), root.path());
    let loaded: Vec<_> = registry.loaded().collect();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].dir, newest);
    assert_eq!(loaded[0].manifest.version.to_string(), "0.10.0");
}

#[test]
fn non_semver_and_dot_folders_are_ignored() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix", "0.1.0", |_| {});
    fs::create_dir_all(root.path().join("fix/latest")).unwrap();
    fs::create_dir_all(root.path().join("fix/1.0")).unwrap();
    fs::create_dir_all(root.path().join(".staging/abc")).unwrap();
    fs::write(root.path().join("stray-file.txt"), b"x").unwrap();
    let registry = PluginRegistry::scan(&common::engine(), root.path());
    let ids: Vec<_> = registry.entries().iter().map(PluginEntry::id).collect();
    assert_eq!(ids, ["fix"]);
    assert_eq!(registry.loaded().count(), 1);
}

#[test]
fn failures_are_recorded_with_reasons() {
    let root = tempfile::tempdir().unwrap();
    let engine = common::engine();
    common::install_fixture(root.path(), "ok", "0.1.0", |_| {});
    common::install_fixture(root.path(), "api", "0.1.0", |m| {
        m["apiVersion"] = Value::from("0.2");
    });
    common::install_fixture(root.path(), "renamed", "0.1.0", |m| {
        m["id"] = Value::from("other");
    });
    common::install_fixture(root.path(), "wrongver", "0.2.0", |m| {
        m["version"] = Value::from("0.3.0");
    });
    fs::create_dir_all(root.path().join("empty")).unwrap();

    let registry = PluginRegistry::scan(&engine, root.path());
    let ids: Vec<_> = registry.entries().iter().map(PluginEntry::id).collect();
    assert_eq!(ids, ["api", "empty", "ok", "renamed", "wrongver"]);
    assert_eq!(registry.loaded().count(), 1);
    assert!(failed_reason(&registry, "api").contains("API"));
    assert!(failed_reason(&registry, "renamed").contains("does not match"));
    assert!(failed_reason(&registry, "wrongver").contains("does not match"));
    assert!(failed_reason(&registry, "empty").contains("no installed version"));
}

#[test]
fn third_fault_disables_plugin() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix", "0.1.0", |_| {});
    let mut registry = PluginRegistry::scan(&common::engine(), root.path());
    for _ in 1..MAX_FAULTS_PER_SESSION {
        assert!(!registry.record_fault("fix"));
    }
    assert!(registry.record_fault("fix"));
    assert_eq!(registry.loaded().count(), 0);
    assert!(failed_reason(&registry, "fix").contains("crash"));
    assert!(!registry.record_fault("fix"), "already disabled");
    assert!(!registry.record_fault("unknown"));
}
