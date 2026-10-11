mod common;

use std::path::Path;

use saveforge_host::{Detection, HostError, PluginEngine, PluginFault, PluginRegistry, detect};
use serde_json::Value;

/// Engine and registry must come from the same engine: a `LoadedPlugin` can
/// only be instantiated by the engine that compiled it.
fn setup(root: &Path) -> (PluginEngine, PluginRegistry) {
    let engine = common::engine();
    let registry = PluginRegistry::scan(&engine, root);
    (engine, registry)
}

#[test]
fn single_match_wins() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    common::install_fixture(root.path(), "fix-c", "0.1.0", |m| {
        m["fileExtensions"] = Value::from(vec![".other"]);
    });
    let (engine, registry) = setup(root.path());
    let report = detect(&engine, &registry, Path::new("s.fixture"), b"x");
    assert_eq!(report.faults.len(), 0);
    match report.detection {
        Detection::Winner(p) => assert_eq!(p.manifest.id, "fix-a"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn equal_priority_is_a_tie() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    common::install_fixture(root.path(), "fix-b", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    let report = detect(&engine, &registry, Path::new("s.fixture"), b"x");
    match report.detection {
        Detection::Tie(ps) => {
            let ids: Vec<_> = ps.iter().map(|p| p.manifest.id.as_str()).collect();
            assert_eq!(ids, ["fix-a", "fix-b"]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn no_match_when_extension_or_plugin_declines() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    assert!(matches!(
        detect(&engine, &registry, Path::new("s.txt"), b"x").detection,
        Detection::NoMatch
    ));
    assert!(matches!(
        detect(&engine, &registry, Path::new("s.fixture"), b"no-match").detection,
        Detection::NoMatch
    ));
}

#[test]
fn long_header_is_truncated_to_512_bytes() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    let header = vec![b'x'; 4096];
    let report = detect(&engine, &registry, Path::new("s.fixture"), &header);
    assert!(matches!(report.detection, Detection::Winner(_)));
}

#[test]
fn faulting_plugin_is_reported_not_fatal() {
    let root = tempfile::tempdir().unwrap();
    common::install_fixture(root.path(), "fix-a", "0.1.0", |_| {});
    let (engine, registry) = setup(root.path());
    let report = detect(&engine, &registry, Path::new("spin-detect.fixture"), b"");
    assert!(matches!(report.detection, Detection::NoMatch));
    assert_eq!(report.faults.len(), 1);
    assert_eq!(report.faults[0].0, "fix-a");
    assert!(matches!(
        report.faults[0].1,
        HostError::Fault(PluginFault::Timeout)
    ));
}
