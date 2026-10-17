//! Screens, navigation, the info bar, About and Settings.

mod common;

use std::fs;
use std::path::Path;

use common::World;
use iced_test::simulator;
use saveforge_app::documents::{DetectOutcome, detect_file, open_document};
use saveforge_app::settings_form::{SettingsField, SettingsForm};
use saveforge_app::{App, Message, Screen};
use saveforge_core::paths::AppPaths;
use saveforge_core::settings::AppSettings;
use saveforge_host::{HOST_VERSION, WASMTIME_VERSION};
use saveforge_store::cache::{CatalogLoad, CatalogSource};
use saveforge_store::{Catalog, Update};
use semver::Version;

fn settings(world: &World) -> AppSettings {
    let mut s = AppSettings::defaults(&AppPaths::under(world.dir.path().join("data")));
    s.plugin_directory = world.plugins_dir();
    s.backup_directory = world.dir.path().join("backups");
    s.check_updates_on_startup = false;
    s
}

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

#[test]
fn navigation_switches_screens_and_keeps_the_editor() {
    let world = World::new();
    let mut app = ready_app(&world);
    assert_eq!(app.screen(), &Screen::Home);
    open_in_app(&world, &mut app, &world.save_file("a.fixture", "hello"));
    assert!(app.editor().is_some());

    let _ = app.update(Message::Navigate(Screen::About));
    assert_eq!(app.screen(), &Screen::About);
    assert!(app.editor().is_some());

    let _ = app.update(Message::SelectPlugin("test-plugin".to_owned()));
    assert_eq!(app.screen(), &Screen::Home);
    assert!(app.editor().is_some());
}

#[test]
fn about_shows_versions_and_links() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::Navigate(Screen::About));
    let mut ui = simulator(app.view());
    assert!(ui.find(format!("SaveForge {}", *HOST_VERSION)).is_ok());
    assert!(ui.find(format!("wasmtime {WASMTIME_VERSION}")).is_ok());
    assert!(ui.find("Apache-2.0 WITH LLVM-exception").is_ok());
    let _ = ui.click("Repository");
    let messages: Vec<Message> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::OpenUrl(url) if url == saveforge_app::REPOSITORY_URL)),
        "{messages:?}"
    );
}

#[test]
fn info_bar_can_be_dismissed() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::DismissInfo);
    assert_eq!(app.info(), None);

    let catalog = Catalog::parse(r#"{"schemaVersion":"1","plugins":[]}"#).unwrap();
    let load = CatalogLoad {
        catalog,
        source: CatalogSource::Online,
    };
    let update = Update {
        id: "test-plugin".to_owned(),
        name: "Test Plugin".to_owned(),
        installed: Version::new(0, 1, 0),
        latest: Version::new(0, 2, 0),
    };
    let _ = app.update(Message::UpdatesChecked {
        result: Ok((load, vec![update])),
    });
    assert_ne!(app.info(), None);
    {
        let mut ui = simulator(app.view());
        let _ = ui.click("Dismiss");
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.iter().any(|m| matches!(m, Message::DismissInfo)));
    }
    let _ = app.update(Message::DismissInfo);
    assert_eq!(app.info(), None);
}

#[test]
fn rail_has_store_settings_and_about() {
    let world = World::new();
    let app = ready_app(&world);
    let mut ui = simulator(app.view());
    let _ = ui.click("Store");
    let _ = ui.click("Settings");
    let _ = ui.click("About");
    let messages: Vec<Message> = ui.into_messages().collect();
    for screen in [Screen::Store, Screen::Settings, Screen::About] {
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::Navigate(s) if *s == screen)),
            "{screen:?} in {messages:?}"
        );
    }
}

#[test]
fn save_panel_is_only_on_home() {
    let world = World::new();
    let mut app = ready_app(&world);
    {
        let mut ui = simulator(app.view());
        assert!(ui.find("Test Plugin").is_ok());
    }
    let _ = app.update(Message::Navigate(Screen::Settings));
    let mut ui = simulator(app.view());
    assert!(ui.find("Test Plugin").is_err());
}

fn edit(app: &mut App, field: SettingsField, value: &str) {
    let _ = app.update(Message::SettingsEdited(field, value.to_owned()));
}

fn saved_settings(world: &World) -> AppSettings {
    let defaults = settings(world);
    AppSettings::load(&world.services.paths.settings_file(), defaults).settings
}

#[test]
fn settings_form_validates_fields() {
    let world = World::new();
    let mut form = SettingsForm::from_settings(&settings(&world));
    assert_eq!(form.validate(), Vec::<String>::new());
    form.catalog_url = "ftp://x".to_owned();
    form.backup_directory = "  ".to_owned();
    assert_eq!(
        form.validate(),
        [
            "Catalog URL must start with https:// or file://",
            "Backup directory must not be empty.",
        ]
    );
    form.plugin_directory = String::new();
    assert_eq!(form.validate()[1], "Plugin directory must not be empty.");
}

#[test]
fn saving_settings_keeps_save_directories() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::Navigate(Screen::Settings));
    let backups = world.dir.path().join("elsewhere");
    edit(
        &mut app,
        SettingsField::BackupDirectory,
        &backups.display().to_string(),
    );
    let _ = app.update(Message::SettingsToggled(true));
    let _ = app.update(Message::SaveSettings);

    assert_eq!(app.status(), Some("Settings saved."));
    let on_disk = saved_settings(&world);
    assert_eq!(on_disk.backup_directory, backups);
    assert!(on_disk.check_updates_on_startup);
    assert_eq!(
        on_disk.plugin_save_directories.get("test-plugin"),
        Some(&world.dir.path().join("saves").display().to_string())
    );
    assert_eq!(app.settings().backup_directory, backups);
}

#[test]
fn changing_plugin_directory_says_restart() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::Navigate(Screen::Settings));
    let other = world.dir.path().join("other-plugins");
    edit(
        &mut app,
        SettingsField::PluginDirectory,
        &other.display().to_string(),
    );
    let _ = app.update(Message::SaveSettings);
    assert_eq!(
        app.status(),
        Some("Settings saved. Plugin directory changes take effect after restart.")
    );
    assert_eq!(app.settings().plugin_directory, other);
    assert!(app.registry().unwrap().get("test-plugin").is_some());
}

#[test]
fn invalid_settings_are_not_saved() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::Navigate(Screen::Settings));
    let _ = app.update(Message::SaveSettings);
    let before = fs::read_to_string(world.services.paths.settings_file()).unwrap();

    edit(
        &mut app,
        SettingsField::CatalogUrl,
        "http://example.com/c.json",
    );
    edit(&mut app, SettingsField::BackupDirectory, "/somewhere/else");
    let _ = app.update(Message::SaveSettings);
    assert_eq!(
        fs::read_to_string(world.services.paths.settings_file()).unwrap(),
        before
    );
    assert_ne!(app.settings_form().errors.len(), 0);
    assert_ne!(app.settings().catalog_url, "http://example.com/c.json");
    let mut ui = simulator(app.view());
    assert!(
        ui.find("Catalog URL must start with https:// or file://")
            .is_ok()
    );
}

#[test]
fn unwritable_settings_keep_changes_in_memory() {
    let mut world = World::new();
    let blocker = world.dir.path().join("blocked");
    fs::write(&blocker, b"not a dir").unwrap();
    world.services.paths = AppPaths::under(blocker);
    let mut app = ready_app(&world);
    let _ = app.update(Message::Navigate(Screen::Settings));
    let backups = world.dir.path().join("elsewhere");
    edit(
        &mut app,
        SettingsField::BackupDirectory,
        &backups.display().to_string(),
    );
    let _ = app.update(Message::SaveSettings);
    assert!(
        app.status()
            .is_some_and(|s| s.starts_with("Could not save settings")),
        "{:?}",
        app.status()
    );
    assert_eq!(app.settings().backup_directory, backups);
}
