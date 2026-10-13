//! End to end through the real `StardewLike` wasm component built by
//! `cargo xtask plugins` into `target/dev-plugins`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use iced_test::simulator;
use saveforge_app::documents::{DetectOutcome, detect_file, open_document, save_document};
use saveforge_app::editor::Editor;
use saveforge_app::render::{UiMessage, render};
use saveforge_host::{Event, PluginRegistry};
use serde_json::{Value, json};

fn dev_plugins() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace.join("target"), PathBuf::from);
    let dir = target.join("dev-plugins");
    assert!(
        dir.join("stardew-like").is_dir(),
        "sample plugin not built: run `cargo xtask plugins` first ({})",
        dir.display()
    );
    dir
}

fn sample_save(dir: &Path) -> PathBuf {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/sample-saves/StardewLike-Farm.json");
    let path = dir.join("Farm.json");
    fs::copy(source, &path).unwrap();
    path
}

#[test]
fn sample_plugin_detects_opens_edits_and_saves() {
    let world = common::World::new();
    let engine = &world.services.engine;
    let registry = PluginRegistry::scan(engine, &dev_plugins());
    let path = sample_save(world.dir.path());

    let DetectOutcome::Open(plugin) = detect_file(engine, &registry, &path).unwrap().outcome else {
        panic!("the sample save should match the StardewLike plugin");
    };
    assert_eq!(plugin.manifest.id, "stardew-like");

    let opened = open_document(engine, &plugin, &path, &world.services.tokens).unwrap();
    let mut editor = Editor::new(opened);
    {
        let mut ui = simulator(render(&editor.tree, &editor.ui));
        assert!(ui.find("Money").is_ok());
        assert!(ui.find("Robin").is_ok());
    }

    editor.handle(UiMessage::NumberEdited {
        id: "player.money".to_owned(),
        text: "99999".to_owned(),
    });
    assert!(editor.dirty);

    let backups = world.dir.path().join("backups");
    let backup = save_document(&editor.session(), &path, &backups).unwrap();
    let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["player"]["money"], json!(99_999));
    assert_eq!(
        saved["farm"]["name"],
        json!("Sunny Acres"),
        "unknown data kept"
    );
    let original: Value = serde_json::from_str(&fs::read_to_string(backup).unwrap()).unwrap();
    assert_eq!(original["player"]["money"], json!(2500));
}

#[test]
fn sample_plugin_blocks_saving_invalid_data() {
    let world = common::World::new();
    let engine = &world.services.engine;
    let registry = PluginRegistry::scan(engine, &dev_plugins());
    let path = sample_save(world.dir.path());
    let DetectOutcome::Open(plugin) = detect_file(engine, &registry, &path).unwrap().outcome else {
        panic!("no match");
    };
    let mut editor =
        Editor::new(open_document(engine, &plugin, &path, &world.services.tokens).unwrap());
    editor.handle(UiMessage::Plugin(Event::TextChanged((
        "player.name".to_owned(),
        String::new(),
    ))));

    let failure =
        save_document(&editor.session(), &path, &world.dir.path().join("backups")).unwrap_err();
    assert!(failure.message.contains("Name is required."), "{failure:?}");
    assert!(!failure.plugin_fault);
    let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk["player"]["name"], json!("Robin"), "file untouched");
}
