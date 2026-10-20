use std::fs;

use crate::common::World;
use saveforge_app::documents::{DetectOutcome, detect_file, lock, open_document, save_document};
use saveforge_host::{Event, LoadedPlugin};
use saveforge_plugin_api::ui::Widget;

fn shown_text(tree: &saveforge_host::UiTree) -> String {
    match &tree.nodes[1].widget {
        Widget::Text(span) => span.text.clone(),
        other => panic!("unexpected {other:?}"),
    }
}

fn detect_one(world: &World, path: &std::path::Path) -> std::sync::Arc<LoadedPlugin> {
    match detect_file(&world.services.engine, &world.registry(), path)
        .unwrap()
        .outcome
    {
        DetectOutcome::Open(plugin) => plugin,
        other => panic!("expected a single match, got {other:?}"),
    }
}

#[test]
fn detect_open_edit_save_round_trip() {
    let world = World::new();
    let path = world.save_file("a.fixture", "hello");
    let plugin = detect_one(&world, &path);
    assert_eq!(plugin.manifest.id, "test-plugin");

    let opened = open_document(
        &world.services.engine,
        &plugin,
        &path,
        &world.services.tokens,
    )
    .unwrap();
    assert_eq!(opened.plugin_id, "test-plugin");
    assert_eq!(shown_text(&opened.tree), "hello");

    lock(&opened.session)
        .update(&Event::TextChanged(("text".to_owned(), "bye".to_owned())))
        .unwrap();
    let backups = world.dir.path().join("backups");
    let backup = save_document(&opened.session, &path, &backups).unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "bye");
    assert_eq!(fs::read_to_string(&backup).unwrap(), "hello");
    assert!(backup.starts_with(backups.join("test-plugin")));
    assert!(!lock(&opened.session).is_dirty().unwrap());
}

#[test]
fn unknown_extension_matches_nothing() {
    let world = World::new();
    let path = world.save_file("a.txt", "hello");
    let detected = detect_file(&world.services.engine, &world.registry(), &path).unwrap();
    assert!(matches!(detected.outcome, DetectOutcome::NoMatch));
    assert_eq!(detected.faulted, Vec::<String>::new());
}

#[test]
fn missing_file_is_reported() {
    let world = World::new();
    let path = world.dir.path().join("gone.fixture");
    let failure = detect_file(&world.services.engine, &world.registry(), &path).unwrap_err();
    assert!(failure.message.contains("gone.fixture"), "{failure:?}");
    assert!(!failure.plugin_fault);
}

#[test]
fn rejection_is_not_a_fault_but_a_panic_is() {
    let world = World::new();
    let rejected = world.save_file("r.fixture", "reject");
    let plugin = detect_one(&world, &rejected);
    let failure = open_document(
        &world.services.engine,
        &plugin,
        &rejected,
        &world.services.tokens,
    )
    .unwrap_err();
    assert!(
        failure.message.contains("rejected by fixture"),
        "{failure:?}"
    );
    assert!(!failure.plugin_fault);

    let panics = world.save_file("p.fixture", "panic");
    let failure = open_document(
        &world.services.engine,
        &plugin,
        &panics,
        &world.services.tokens,
    )
    .unwrap_err();
    assert!(failure.plugin_fault, "{failure:?}");
}

#[test]
fn serialize_error_leaves_file_untouched() {
    let world = World::new();
    let path = world.save_file("a.fixture", "hello");
    let plugin = detect_one(&world, &path);
    let opened = open_document(
        &world.services.engine,
        &plugin,
        &path,
        &world.services.tokens,
    )
    .unwrap();
    lock(&opened.session)
        .update(&Event::TextChanged((
            "text".to_owned(),
            "unserializable".to_owned(),
        )))
        .unwrap();
    let failure =
        save_document(&opened.session, &path, &world.dir.path().join("backups")).unwrap_err();
    assert!(failure.message.contains("cannot serialize"), "{failure:?}");
    assert!(!failure.plugin_fault);
    assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
}
