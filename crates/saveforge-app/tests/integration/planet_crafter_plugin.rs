//! End to end through the real Planet Crafter wasm component built by
//! `cargo xtask plugins` into `target/dev-plugins`.

use crate::common;

use std::fs;
use std::path::{Path, PathBuf};

use iced_test::simulator;
use saveforge_app::documents::{DetectOutcome, detect_file, open_document, save_document};
use saveforge_app::editor::Editor;
use saveforge_app::render::{UiMessage, render};
use saveforge_host::{Event, PluginRegistry};

fn dev_plugins() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace.join("target"), PathBuf::from);
    let dir = target.join("dev-plugins");
    assert!(
        dir.join("planet-crafter").is_dir(),
        "Planet Crafter plugin not built: run `cargo xtask plugins` first ({})",
        dir.display()
    );
    dir
}

fn sample_bytes() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/sample-saves/PlanetCrafter-Standard-1.json"),
    )
    .unwrap()
}

fn open_sample(world: &common::World) -> (Editor, PathBuf) {
    let engine = &world.services.engine;
    let registry = PluginRegistry::scan(engine, &dev_plugins());
    let path = world.dir.path().join("Standard-1.json");
    fs::write(&path, sample_bytes()).unwrap();

    let DetectOutcome::Open(plugin) = detect_file(engine, &registry, &path).unwrap().outcome else {
        panic!("the sample save should match the Planet Crafter plugin");
    };
    assert_eq!(plugin.manifest.id, "planet-crafter");
    let opened = open_document(engine, &plugin, &path, &world.services.tokens).unwrap();
    (Editor::new(opened), path)
}

#[test]
fn planet_crafter_saves_an_unmodified_save_byte_for_byte() {
    let world = common::World::new();
    let (editor, path) = open_sample(&world);
    {
        let mut ui = simulator(render(&editor.tree, &editor.ui));
        assert!(ui.find("Characters").is_ok());
        assert!(ui.find("Health").is_ok());
    }
    assert!(!editor.dirty);

    save_document(&editor.session(), &path, &world.dir.path().join("backups")).unwrap();
    assert_eq!(fs::read(&path).unwrap(), sample_bytes());
}

#[test]
fn planet_crafter_edits_terra_tokens_and_keeps_everything_else() {
    let world = common::World::new();
    let (mut editor, path) = open_sample(&world);
    editor.handle(UiMessage::Plugin(Event::TabSelected((
        "tabs".to_owned(),
        1,
    ))));
    {
        let mut ui = simulator(render(&editor.tree, &editor.ui));
        assert!(ui.find("All-Time Terra Tokens").is_ok());
    }
    editor.handle(UiMessage::NumberEdited {
        id: "tokens.terraTokens".to_owned(),
        text: "15000".to_owned(),
    });
    assert!(editor.dirty);

    let backup =
        save_document(&editor.session(), &path, &world.dir.path().join("backups")).unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    let original = String::from_utf8(sample_bytes()).unwrap();
    assert!(
        saved.starts_with("\r{\"terraTokens\":15000,"),
        "{}",
        &saved[..60]
    );
    let after_tokens = |text: &str| text[text.find('@').unwrap()..].to_owned();
    assert_eq!(after_tokens(&saved), after_tokens(&original));
    assert_eq!(fs::read(backup).unwrap(), sample_bytes());
}
