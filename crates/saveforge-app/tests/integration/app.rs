//! App state-machine tests. Background tasks are not run: each test calls the
//! blocking function itself and feeds the result back as the message the task
//! would have produced.

use crate::common;

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::World;
use iced_test::simulator;
use saveforge_app::documents::{DetectOutcome, Failure, detect_file, open_document};
use saveforge_app::home::Saves;
use saveforge_app::render::UiMessage;
use saveforge_app::{App, Message, Modal, Pending};
use saveforge_core::discovery::discover;
use saveforge_core::paths::AppPaths;
use saveforge_core::settings::AppSettings;
use saveforge_host::{Event, MAX_FAULTS_PER_SESSION, PluginEntry};

fn settings(world: &World) -> AppSettings {
    let mut s = AppSettings::defaults(&AppPaths::under(world.dir.path().join("data")));
    s.plugin_directory = world.plugins_dir();
    s.backup_directory = world.dir.path().join("backups");
    s
}

/// An app whose plugins have loaded and whose `test-plugin` save folder is
/// `<world>/saves` (set via the settings override).
fn ready_app(world: &World) -> App {
    let mut s = settings(world);
    fs::create_dir_all(world.dir.path().join("saves")).unwrap();
    s.plugin_save_directories.insert(
        "test-plugin".to_owned(),
        world.dir.path().join("saves").display().to_string(),
    );
    let (mut app, _task) = App::new(world.services.clone(), s, None);
    let _ = app.update(Message::PluginsLoaded(world.registry()));
    app
}

/// Opens `path` the way the UI does: `OpenSave` starts a request, then the
/// detection and open results are fed back with that request number.
fn open_in_app(world: &World, app: &mut App, path: &Path) {
    let _ = app.update(Message::OpenSave(path.to_path_buf()));
    let request = app.open_request().expect("open started");
    let detected = detect_file(&world.services.engine, &world.registry(), path);
    let plugin = match &detected.as_ref().unwrap().outcome {
        DetectOutcome::Open(plugin) => plugin.clone(),
        other => panic!("fixture did not match: {other:?}"),
    };
    let _ = app.update(Message::Detected {
        request,
        path: path.to_path_buf(),
        result: detected,
    });
    let result = open_document(
        &world.services.engine,
        &plugin,
        path,
        &world.services.tokens,
    );
    let _ = app.update(Message::Opened {
        request,
        plugin_id: "test-plugin".to_owned(),
        result,
    });
}

fn saved(app: &mut App, result: Result<PathBuf, Failure>) {
    let request = app
        .editor()
        .and_then(|e| e.save_request)
        .expect("a save is in flight");
    let _ = app.update(Message::Saved { request, result });
}

fn type_text(value: &str) -> Message {
    Message::Editor(UiMessage::Plugin(Event::TextChanged((
        "text".to_owned(),
        value.to_owned(),
    ))))
}

fn dirty_app(world: &World) -> App {
    let mut app = ready_app(world);
    let path = world.save_file("a.fixture", "hello");
    open_in_app(world, &mut app, &path);
    let _ = app.update(type_text("changed"));
    app
}

#[test]
fn startup_shows_loading_then_selects_first_plugin() {
    let world = World::new();
    let (app, _task) = App::new(
        world.services.clone(),
        settings(&world),
        Some("warn".to_owned()),
    );
    assert_eq!(app.busy(), Some("Loading plugins…"));
    assert_eq!(app.status(), Some("warn"));

    let app = ready_app(&world);
    assert_eq!(app.home().selected.as_deref(), Some("test-plugin"));
    assert_eq!(app.home().saves, Saves::Scanning);
    assert_eq!(app.busy(), None);
}

#[test]
fn plugin_without_folder_shows_not_configured() {
    let world = World::new();
    let (mut app, _task) = App::new(world.services.clone(), settings(&world), None);
    let _ = app.update(Message::PluginsLoaded(world.registry()));
    assert!(matches!(app.home().saves, Saves::NotConfigured(_)));
}

#[test]
fn saves_found_for_selected_plugin_only() {
    let world = World::new();
    let mut app = ready_app(&world);
    world.save_file("a.fixture", "x");
    let saves = discover(&world.dir.path().join("saves"), &[".fixture".to_owned()]);
    let _ = app.update(Message::SavesFound {
        plugin_id: "other".to_owned(),
        saves: saves.clone(),
    });
    assert_eq!(app.home().saves, Saves::Scanning);
    let _ = app.update(Message::SavesFound {
        plugin_id: "test-plugin".to_owned(),
        saves,
    });
    assert!(matches!(&app.home().saves, Saves::Found(s) if s.len() == 1));
}

#[test]
fn confirming_a_path_persists_it_and_rescans() {
    let world = World::new();
    let (mut app, _task) = App::new(world.services.clone(), settings(&world), None);
    let _ = app.update(Message::PluginsLoaded(world.registry()));
    let saves = world.dir.path().join("my saves");
    fs::create_dir_all(&saves).unwrap();

    let _ = app.update(Message::EditPath);
    assert_eq!(app.home().editing.as_deref(), Some(""));
    let _ = app.update(Message::PathChanged(saves.display().to_string()));
    let _ = app.update(Message::ConfirmPath);

    assert_eq!(app.home().editing, None);
    assert_eq!(app.home().saves, Saves::Scanning);
    let stored = fs::read_to_string(world.services.paths.settings_file()).unwrap();
    assert!(stored.contains("my saves"), "{stored}");
    assert_eq!(
        app.settings().plugin_save_directories["test-plugin"],
        saves.display().to_string()
    );
}

#[test]
fn cancel_path_edit_keeps_old_value() {
    let world = World::new();
    let mut app = ready_app(&world);
    let before = app.home().raw_dir.clone();
    let _ = app.update(Message::EditPath);
    let _ = app.update(Message::PathChanged("elsewhere".to_owned()));
    let _ = app.update(Message::CancelPath);
    assert_eq!(app.home().editing, None);
    assert_eq!(app.home().raw_dir, before);
}

#[test]
fn opened_document_shows_editor_and_dirty_title() {
    let world = World::new();
    let mut app = ready_app(&world);
    open_in_app(&world, &mut app, &world.save_file("a.fixture", "hello"));
    assert!(app.editor().is_some());
    assert_eq!(app.title(), "SaveForge — a.fixture");
    let _ = app.update(type_text("changed"));
    assert_eq!(app.title(), "SaveForge — a.fixture •");
}

#[test]
fn closing_with_unsaved_changes_asks_first() {
    let world = World::new();
    let mut app = dirty_app(&world);
    let _ = app.update(Message::CloseEditor);
    assert!(matches!(
        app.modal(),
        Some(Modal::Unsaved {
            then: Pending::CloseEditor
        })
    ));
    let _ = app.update(Message::CancelModal);
    assert!(app.modal().is_none());
    assert!(app.editor().is_some());

    let _ = app.update(Message::CloseEditor);
    let _ = app.update(Message::GuardDiscard);
    assert!(app.editor().is_none());
}

#[test]
fn window_close_with_unsaved_changes_asks_first() {
    let world = World::new();
    let mut app = dirty_app(&world);
    let _ = app.update(Message::CloseRequested(iced::window::Id::unique()));
    assert!(matches!(
        app.modal(),
        Some(Modal::Unsaved {
            then: Pending::Exit(_)
        })
    ));
}

#[test]
fn opening_another_save_while_dirty_asks_first() {
    let world = World::new();
    let mut app = dirty_app(&world);
    let other = world.save_file("b.fixture", "other");
    let _ = app.update(Message::OpenSave(other));
    assert!(matches!(
        app.modal(),
        Some(Modal::Unsaved {
            then: Pending::OpenFile(_)
        })
    ));
    let _ = app.update(Message::GuardDiscard);
    assert!(app.editor().is_none());
    assert_eq!(app.busy(), Some("Detecting…"));
}

#[test]
fn guard_save_then_runs_pending_action_after_save() {
    let world = World::new();
    let mut app = dirty_app(&world);
    let _ = app.update(Message::CloseEditor);
    let _ = app.update(Message::GuardSave);
    assert!(app.editor().is_some_and(|e| e.saving));
    saved(&mut app, Ok(PathBuf::from("backup.bak")));
    assert!(
        app.editor().is_none(),
        "pending close should run after save"
    );
    assert!(app.status().is_some_and(|s| s.contains("Saved")));
}

#[test]
fn failed_save_keeps_editor_and_drops_pending_action() {
    let world = World::new();
    let mut app = dirty_app(&world);
    let _ = app.update(Message::CloseEditor);
    let _ = app.update(Message::GuardSave);
    saved(&mut app, Err(Failure::new("disk full")));
    let editor = app.editor().expect("editor stays open");
    assert!(!editor.saving);
    assert!(editor.dirty);
    assert_eq!(app.status(), Some("disk full"));
}

#[test]
fn tie_shows_plugin_chooser() {
    let world = World::new();
    common::install_fixture(&world.plugins_dir(), "second");
    let mut app = ready_app(&world);
    let path = world.save_file("a.fixture", "hello");
    let _ = app.update(Message::OpenSave(path.clone()));
    let request = app.open_request().unwrap();
    let result = detect_file(&world.services.engine, &world.registry(), &path);
    let _ = app.update(Message::Detected {
        request,
        path,
        result,
    });
    assert!(
        matches!(app.modal(), Some(Modal::ChoosePlugin { candidates, .. }) if candidates.len() == 2)
    );
}

#[test]
fn no_match_reports_in_status() {
    let world = World::new();
    let mut app = ready_app(&world);
    let path = world.save_file("a.txt", "hello");
    let _ = app.update(Message::OpenSave(path.clone()));
    let request = app.open_request().unwrap();
    let result = detect_file(&world.services.engine, &world.registry(), &path);
    let _ = app.update(Message::Detected {
        request,
        path,
        result,
    });
    assert_eq!(app.open_request(), None);
    assert_eq!(app.status(), Some("No plugin can open a.txt"));
}

#[test]
fn repeated_plugin_crashes_disable_the_plugin() {
    let world = World::new();
    let mut app = ready_app(&world);
    for _ in 0..MAX_FAULTS_PER_SESSION {
        open_in_app(&world, &mut app, &world.save_file("a.fixture", "hello"));
        let _ = app.update(Message::Editor(UiMessage::Plugin(Event::Pressed(
            "spin".to_owned(),
        ))));
        assert!(app.editor().is_some_and(|e| e.crashed.is_some()));
        let _ = app.update(Message::CloseEditor);
    }
    assert!(matches!(
        app.registry().and_then(|r| r.get("test-plugin")),
        Some(PluginEntry::Failed { .. })
    ));
    assert!(app.status().is_some_and(|s| s.contains("disabled")));
}

#[test]
fn view_shows_saves_and_editor_controls() {
    let world = World::new();
    let mut app = ready_app(&world);
    world.save_file("a.fixture", "hello");
    let saves = discover(&world.dir.path().join("saves"), &[".fixture".to_owned()]);
    let _ = app.update(Message::SavesFound {
        plugin_id: "test-plugin".to_owned(),
        saves,
    });
    {
        let mut ui = simulator(app.view());
        assert!(ui.find("a.fixture").is_ok());
        assert!(ui.find("Test Plugin").is_ok());
    }

    open_in_app(&world, &mut app, &world.save_file("a.fixture", "hello"));
    let mut ui = simulator(app.view());
    assert!(ui.find("hello").is_ok());
    let _ = ui.click("Save");
    let _ = ui.click("Close");
    let messages: Vec<Message> = ui.into_messages().collect();
    assert!(messages.iter().any(|m| matches!(m, Message::Save)));
    assert!(messages.iter().any(|m| matches!(m, Message::CloseEditor)));
}

#[test]
fn save_list_refreshes_after_a_save() {
    let world = World::new();
    let mut app = dirty_app(&world);
    let _ = app.update(Message::SavesFound {
        plugin_id: "test-plugin".to_owned(),
        saves: vec![],
    });
    let _ = app.update(Message::Save);
    saved(&mut app, Ok(PathBuf::from("backup.bak")));
    // Size and modified time changed on disk: the list is rescanned.
    assert_eq!(app.home().saves, Saves::Scanning);
}

#[test]
fn save_is_ignored_without_editor_or_while_saving() {
    let world = World::new();
    let mut app = ready_app(&world);
    assert_eq!(app.update(Message::Save).units(), 0);
    assert_eq!(app.busy(), None);

    let mut app = dirty_app(&world);
    assert_eq!(app.update(Message::Save).units(), 1);
    assert_eq!(app.busy(), Some("Saving…"));
    // A second Save while the first is in flight starts no new work.
    assert_eq!(app.update(Message::Save).units(), 0);
}

#[test]
fn crashed_editor_closes_without_asking() {
    let world = World::new();
    let mut app = dirty_app(&world);
    let _ = app.update(Message::Editor(UiMessage::Plugin(Event::Pressed(
        "spin".to_owned(),
    ))));
    let _ = app.update(Message::CloseEditor);
    assert!(app.modal().is_none());
    assert!(app.editor().is_none());
}

#[test]
fn unwritable_settings_keep_the_path_in_memory() {
    let mut world = World::new();
    // A file where the data directory should be: settings cannot be written.
    let blocker = world.dir.path().join("blocked");
    fs::write(&blocker, b"not a dir").unwrap();
    world.services.paths = AppPaths::under(blocker);
    let (mut app, _task) = App::new(world.services.clone(), settings(&world), None);
    let _ = app.update(Message::PluginsLoaded(world.registry()));
    let saves = world.dir.path().join("saves");
    fs::create_dir_all(&saves).unwrap();

    let _ = app.update(Message::EditPath);
    let _ = app.update(Message::PathChanged(saves.display().to_string()));
    let _ = app.update(Message::ConfirmPath);

    assert!(
        app.status()
            .is_some_and(|s| s.contains("Could not save settings"))
    );
    assert_eq!(app.home().saves, Saves::Scanning);
    assert_eq!(app.home().raw_dir, Some(saves.display().to_string()));
}

#[test]
fn cancelled_browse_keeps_typed_path() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::EditPath);
    let _ = app.update(Message::PathChanged("typed".to_owned()));
    let _ = app.update(Message::PathBrowsed(None));
    assert_eq!(app.home().editing.as_deref(), Some("typed"));
}

fn fixture_plugin(world: &World) -> std::sync::Arc<saveforge_host::LoadedPlugin> {
    match world.registry().get("test-plugin") {
        Some(PluginEntry::Loaded(plugin)) => plugin.clone(),
        other => panic!("fixture not loaded: {other:?}"),
    }
}

#[test]
fn late_open_result_is_ignored() {
    let world = World::new();
    let mut app = ready_app(&world);
    let a = world.save_file("a.fixture", "aaa");
    let b = world.save_file("b.fixture", "bbb");
    let plugin = fixture_plugin(&world);

    // Click a, then b before a has opened: only b's result may apply.
    let _ = app.update(Message::OpenSave(a.clone()));
    let first = app.open_request().unwrap();
    let _ = app.update(Message::OpenSave(b.clone()));
    let second = app.open_request().unwrap();
    assert_ne!(first, second);

    let late_a = open_document(&world.services.engine, &plugin, &a, &world.services.tokens);
    let _ = app.update(Message::Opened {
        request: first,
        plugin_id: "test-plugin".to_owned(),
        result: late_a,
    });
    assert!(app.editor().is_none(), "superseded open must not apply");

    let b_doc = open_document(&world.services.engine, &plugin, &b, &world.services.tokens);
    let _ = app.update(Message::Opened {
        request: second,
        plugin_id: "test-plugin".to_owned(),
        result: b_doc,
    });
    assert_eq!(app.editor().unwrap().file_name(), "b.fixture");
    assert_eq!(app.busy(), None);
}

#[test]
fn late_open_result_does_not_replace_edited_document() {
    let world = World::new();
    let mut app = ready_app(&world);
    let plugin = fixture_plugin(&world);
    let other = world.save_file("b.fixture", "bbb");
    let stale = open_document(
        &world.services.engine,
        &plugin,
        &other,
        &world.services.tokens,
    );
    open_in_app(&world, &mut app, &world.save_file("a.fixture", "aaa"));
    let _ = app.update(type_text("edited"));
    let _ = app.update(Message::Opened {
        request: 0,
        plugin_id: "test-plugin".to_owned(),
        result: stale,
    });
    let editor = app.editor().unwrap();
    assert_eq!(editor.file_name(), "a.fixture");
    assert!(editor.dirty);
}

#[test]
fn switching_plugin_cancels_pending_open() {
    let world = World::new();
    common::install_fixture(&world.plugins_dir(), "second");
    let mut app = ready_app(&world);
    let a = world.save_file("a.fixture", "aaa");
    let _ = app.update(Message::OpenSave(a.clone()));
    let request = app.open_request().unwrap();
    // "second" sorts first, so it is the selected plugin; switch away from it.
    assert_eq!(app.home().selected.as_deref(), Some("second"));
    let _ = app.update(Message::SelectPlugin("test-plugin".to_owned()));
    assert_eq!(app.open_request(), None);
    assert_eq!(app.busy(), None);
    let late = open_document(
        &world.services.engine,
        &fixture_plugin(&world),
        &a,
        &world.services.tokens,
    );
    let _ = app.update(Message::Opened {
        request,
        plugin_id: "test-plugin".to_owned(),
        result: late,
    });
    assert!(app.editor().is_none());
}

#[test]
fn stale_save_result_does_not_touch_replacement_document() {
    let world = World::new();
    let mut app = ready_app(&world);
    open_in_app(&world, &mut app, &world.save_file("a.fixture", "aaa"));
    let _ = app.update(Message::Save);
    let a_save = app.editor().and_then(|e| e.save_request).unwrap();
    let _ = app.update(Message::CloseEditor);
    open_in_app(&world, &mut app, &world.save_file("b.fixture", "bbb"));
    let _ = app.update(type_text("edited b"));

    let _ = app.update(Message::Saved {
        request: a_save,
        result: Ok(PathBuf::from("a.bak")),
    });
    let b = app.editor().unwrap();
    assert!(b.dirty, "b must keep its unsaved marker");
    assert_eq!(app.title(), "SaveForge — b.fixture •");
    assert!(!app.status().is_some_and(|s| s.contains("a.bak")));

    let _ = app.update(Message::Saved {
        request: a_save,
        result: Err(Failure {
            message: "plugin crashed: timed out".to_owned(),
            plugin_fault: true,
        }),
    });
    assert!(app.editor().unwrap().crashed.is_none());
}

#[test]
fn stale_guard_action_does_not_run_on_replacement_document() {
    let world = World::new();
    let mut app = dirty_app(&world);
    // Save-then-close is requested, but the user discards via another guard.
    let _ = app.update(Message::CloseEditor);
    let _ = app.update(Message::GuardSave);
    let a_save = app.editor().and_then(|e| e.save_request).unwrap();
    let _ = app.update(Message::OpenSave(world.save_file("b.fixture", "bbb")));
    let _ = app.update(Message::GuardDiscard);
    open_in_app(&world, &mut app, &world.save_file("c.fixture", "ccc"));

    let _ = app.update(Message::Saved {
        request: a_save,
        result: Ok(PathBuf::from("a.bak")),
    });
    assert_eq!(
        app.editor()
            .map(saveforge_app::editor::Editor::file_name)
            .as_deref(),
        Some("c.fixture"),
        "the old pending close must not close the new document"
    );
}
