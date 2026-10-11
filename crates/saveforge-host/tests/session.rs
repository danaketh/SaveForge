mod common;

use std::time::{Duration, Instant};

use saveforge_host::{
    AuxFile, DocumentSession, Event, HostError, LoadedPlugin, PluginEngine, PluginFault,
};
use saveforge_plugin_api::ui::{TextStyle, Widget};

fn setup() -> (tempfile::TempDir, PluginEngine, LoadedPlugin) {
    let root = tempfile::tempdir().unwrap();
    let dir = common::install_fixture(root.path(), "test-plugin", "0.1.0", |_| {});
    let engine = common::engine();
    let plugin = engine.load(&dir).unwrap();
    (root, engine, plugin)
}

fn open(engine: &PluginEngine, plugin: &LoadedPlugin, bytes: &[u8]) -> DocumentSession {
    engine.open(plugin, "a.fixture", bytes, &[]).unwrap()
}

fn shown_text(session: &mut DocumentSession) -> String {
    let tree = session.view().unwrap();
    match &tree.nodes[1].widget {
        Widget::Text(span) if span.style == TextStyle::Body => span.text.clone(),
        other => panic!("unexpected widget {other:?}"),
    }
}

fn set_text(session: &mut DocumentSession, value: &str) {
    session
        .update(&Event::TextChanged(("text".to_owned(), value.to_owned())))
        .unwrap();
}

#[test]
fn round_trip_view_update_serialize_mark_saved() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"hello");
    assert_eq!(s.plugin_id(), "test-plugin");
    assert_eq!(shown_text(&mut s), "hello");
    assert!(!s.is_dirty().unwrap());

    set_text(&mut s, "bye");
    assert!(s.is_dirty().unwrap());
    assert_eq!(shown_text(&mut s), "bye");
    assert_eq!(s.serialize().unwrap(), b"bye");
    assert!(s.is_dirty().unwrap(), "serialize must not clear dirty");
    s.mark_saved().unwrap();
    assert!(!s.is_dirty().unwrap());
}

#[test]
fn aux_files_reach_the_plugin() {
    let (_root, engine, plugin) = setup();
    let aux = [AuxFile {
        id: "greeting".to_owned(),
        bytes: b"hi".to_vec(),
    }];
    let mut s = engine.open(&plugin, "a.fixture", b"x", &aux).unwrap();
    assert_eq!(shown_text(&mut s), "x +greeting=hi");
}

#[test]
fn plugin_rejection_is_reported() {
    let (_root, engine, plugin) = setup();
    let err = engine
        .open(&plugin, "a.fixture", b"reject", &[])
        .unwrap_err();
    assert!(
        matches!(err, HostError::Rejected(ref m) if m == "rejected by fixture"),
        "{err:?}"
    );
}

#[test]
fn panic_in_open_is_a_trap() {
    let (_root, engine, plugin) = setup();
    let err = engine
        .open(&plugin, "a.fixture", b"panic", &[])
        .unwrap_err();
    assert!(
        matches!(err, HostError::Fault(PluginFault::Trap(_))),
        "{err:?}"
    );
}

#[test]
fn memory_hog_is_contained_and_engine_stays_usable() {
    let (_root, engine, plugin) = setup();
    let err = engine.open(&plugin, "a.fixture", b"hog", &[]).unwrap_err();
    assert!(
        matches!(err, HostError::Fault(PluginFault::Trap(_))),
        "{err:?}"
    );
    let mut s = open(&engine, &plugin, b"still fine");
    assert_eq!(shown_text(&mut s), "still fine");
}

#[test]
fn timeout_in_update_poisons_session() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    let started = Instant::now();
    let err = s.update(&Event::Pressed("spin".to_owned())).unwrap_err();
    assert!(
        matches!(err, HostError::Fault(PluginFault::Timeout)),
        "{err:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(s.is_poisoned());
    assert!(matches!(s.view(), Err(HostError::Poisoned)));
}

#[test]
fn timeout_in_view() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    set_text(&mut s, "spin-view");
    assert!(matches!(
        s.view(),
        Err(HostError::Fault(PluginFault::Timeout))
    ));
}

#[test]
fn invalid_tree_poisons_session() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    set_text(&mut s, "bad-tree");
    assert!(matches!(
        s.view(),
        Err(HostError::Fault(PluginFault::InvalidUi(_)))
    ));
    assert!(matches!(s.is_dirty(), Err(HostError::Poisoned)));
}

#[test]
fn serialize_error_keeps_session_alive() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    set_text(&mut s, "unserializable");
    let err = s.serialize().unwrap_err();
    assert!(
        matches!(err, HostError::SerializeFailed(ref m) if m == "cannot serialize"),
        "{err:?}"
    );
    assert!(!s.is_poisoned());
    assert_eq!(shown_text(&mut s), "unserializable");
}

#[test]
fn dropping_poisoned_session_is_immediate() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    let _ = s.update(&Event::Pressed("spin".to_owned()));
    assert!(s.is_poisoned());
    let started = Instant::now();
    drop(s);
    assert!(started.elapsed() < Duration::from_millis(100));
}

#[test]
fn detect_priority_none_and_timeout() {
    let (_root, engine, plugin) = setup();
    assert_eq!(engine.detect(&plugin, "a.fixture", b"x").unwrap(), Some(5));
    assert_eq!(
        engine.detect(&plugin, "a.fixture", b"no-match!").unwrap(),
        None
    );
    let err = engine
        .detect(&plugin, "spin-detect.fixture", b"")
        .unwrap_err();
    assert!(
        matches!(err, HostError::Fault(PluginFault::Timeout)),
        "{err:?}"
    );
}

#[test]
fn sleeping_guest_cannot_block_past_deadline() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    let started = Instant::now();
    let _ = s.update(&Event::Pressed("sleep".to_owned()));
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "guest sleep blocked the host for {:?}",
        started.elapsed()
    );
}

#[test]
fn session_outliving_engine_still_times_out() {
    let (_root, engine, plugin) = setup();
    let mut s = open(&engine, &plugin, b"x");
    drop(plugin);
    drop(engine);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = s.update(&Event::Pressed("spin".to_owned()));
        let _ = tx.send(matches!(
            result,
            Err(HostError::Fault(PluginFault::Timeout))
        ));
    });
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(true));
}
