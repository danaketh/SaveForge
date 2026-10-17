//! Store screens, jobs and the update check. As in `app.rs`, background tasks
//! are not run: each test calls the store function itself and feeds the
//! result back as the message the task would have produced.

mod common;

use std::fmt::Write as _;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use common::World;
use iced_test::simulator;
use saveforge_app::store::{JobOutcome, progress_fraction};
use saveforge_app::{App, Message, Screen};
use saveforge_core::paths::AppPaths;
use saveforge_core::settings::{AppSettings, file_url};
use saveforge_host::{PluginEntry, PluginRegistry};
use saveforge_store::cache::{CatalogLoad, CatalogSource, load_catalog};
use saveforge_store::install::install;
use saveforge_store::{Catalog, Update};
use semver::Version;
use sha2::{Digest, Sha256};

fn settings(world: &World) -> AppSettings {
    let mut s = AppSettings::defaults(&AppPaths::under(world.dir.path().join("data")));
    s.plugin_directory = world.plugins_dir();
    s.backup_directory = world.dir.path().join("backups");
    s.catalog_url = file_url(&catalog_path(world));
    s.check_updates_on_startup = false;
    s.plugin_save_directories.insert(
        "test-plugin".to_owned(),
        world.dir.path().join("saves").display().to_string(),
    );
    s
}

fn app_with(world: &World, s: AppSettings) -> App {
    fs::create_dir_all(world.dir.path().join("saves")).unwrap();
    let (mut app, _task) = App::new(world.services.clone(), s, None);
    let _ = app.update(Message::PluginsLoaded(world.registry()));
    app
}

fn ready_app(world: &World) -> App {
    app_with(world, settings(world))
}

fn catalog_path(world: &World) -> PathBuf {
    world.dir.path().join("catalog").join("catalog.json")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, b| {
            write!(out, "{b:02x}").unwrap();
            out
        })
}

/// A zip of the fixture plugin whose manifest says `id`.
fn fixture_package(id: &str) -> Vec<u8> {
    let manifest_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/test-plugin/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    manifest["id"] = serde_json::Value::from(id);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.to_string().as_bytes()).unwrap();
    zip.start_file("plugin.wasm", options).unwrap();
    zip.write_all(&fs::read(common::fixture_wasm()).unwrap())
        .unwrap();
    zip.finish().unwrap().into_inner()
}

/// Writes the catalog (`test-plugin` and `other`, both 0.1.0) and the
/// package for `other`.
fn write_catalog(world: &World) {
    let dir = catalog_path(world).parent().unwrap().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    let package = fixture_package("other");
    let package_path = dir.join("other-0.1.0.zip");
    fs::write(&package_path, &package).unwrap();
    let entry = |id: &str, name: &str, url: String, sha: String| {
        serde_json::json!({
            "id": id, "name": name, "author": "SaveForge", "description": format!("{name} for tests."),
            "latestVersion": "0.1.0", "minHostVersion": "0.1.0",
            "packageUrl": url, "sha256": sha, "tags": ["fixture"]
        })
    };
    let catalog = serde_json::json!({
        "schemaVersion": "1",
        "plugins": [
            entry("test-plugin", "Test Plugin", "https://example.com/t.zip".into(), "0".repeat(64)),
            entry("other", "Other Plugin", file_url(&package_path), sha256_hex(&package)),
        ]
    });
    fs::write(catalog_path(world), catalog.to_string()).unwrap();
}

fn catalog_load(world: &World) -> CatalogLoad {
    write_catalog(world);
    load_catalog(
        &file_url(&catalog_path(world)),
        &world.services.paths.catalog_cache_file(),
    )
    .unwrap()
}

/// An app on the Store screen with the catalog loaded.
fn store_app(world: &World) -> App {
    let mut app = ready_app(world);
    let _ = app.update(Message::Navigate(Screen::Store));
    let request = app.catalog_request().expect("catalog load started");
    let _ = app.update(Message::CatalogLoaded {
        request,
        result: Ok(catalog_load(world)),
    });
    app
}

fn job_request(app: &App) -> u64 {
    app.store().job.as_ref().expect("a job is running").request
}

fn open_test_plugin_save(world: &World, app: &mut App) {
    let path = world.save_file("a.fixture", "hello");
    let _ = app.update(Message::OpenSave(path.clone()));
    let request = app.open_request().unwrap();
    let detected =
        saveforge_app::documents::detect_file(&world.services.engine, &world.registry(), &path);
    let plugin = match &detected.as_ref().unwrap().outcome {
        saveforge_app::documents::DetectOutcome::Open(plugin) => plugin.clone(),
        other => panic!("{other:?}"),
    };
    let _ = app.update(Message::Detected {
        request,
        path: path.clone(),
        result: detected,
    });
    let result = saveforge_app::documents::open_document(
        &world.services.engine,
        &plugin,
        &path,
        &world.services.tokens,
    );
    let _ = app.update(Message::Opened {
        request,
        plugin_id: "test-plugin".to_owned(),
        result,
    });
    assert!(app.editor().is_some());
}

#[test]
fn store_lists_entries_with_status() {
    let world = World::new();
    let app = store_app(&world);
    assert_eq!(app.screen(), &Screen::Store);
    let mut ui = simulator(app.view());
    assert!(ui.find("Test Plugin").is_ok());
    assert!(ui.find("Other Plugin").is_ok());
    assert!(ui.find("by SaveForge").is_ok());
    assert!(ui.find("Installed").is_ok());
    assert!(ui.find("Update available").is_err());
    assert!(ui.find("Incompatible").is_err());
    let _ = ui.click("Other Plugin");
    let messages: Vec<Message> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::Navigate(Screen::PluginDetail(id)) if id == "other")),
        "{messages:?}"
    );
}

#[test]
fn stale_catalog_results_are_ignored() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::RefreshCatalog);
    let first = app.catalog_request().unwrap();
    let _ = app.update(Message::RefreshCatalog);
    let second = app.catalog_request().unwrap();
    assert_ne!(first, second);
    let _ = app.update(Message::CatalogLoaded {
        request: first,
        result: Ok(catalog_load(&world)),
    });
    assert!(matches!(
        app.store().catalog,
        saveforge_app::store::CatalogState::Loading
    ));
    let _ = app.update(Message::CatalogLoaded {
        request: second,
        result: Err("could not download".to_owned()),
    });
    assert!(matches!(
        &app.store().catalog,
        saveforge_app::store::CatalogState::Failed(e) if e == "could not download"
    ));
}

#[test]
fn offline_catalog_says_so() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::Navigate(Screen::Store));
    let request = app.catalog_request().unwrap();
    let mut load = catalog_load(&world);
    load.source = CatalogSource::Cached;
    let _ = app.update(Message::CatalogLoaded {
        request,
        result: Ok(load),
    });
    assert_eq!(app.status(), Some("Offline: showing the cached catalog."));
}

#[test]
fn failed_catalog_shows_the_error() {
    let world = World::new();
    let mut app = ready_app(&world);
    let _ = app.update(Message::Navigate(Screen::Store));
    let request = app.catalog_request().unwrap();
    let _ = app.update(Message::CatalogLoaded {
        request,
        result: Err("could not download x: offline".to_owned()),
    });
    let mut ui = simulator(app.view());
    assert!(
        ui.find("Could not load the catalog: could not download x: offline")
            .is_ok()
    );
}

#[test]
fn search_filters_the_list() {
    let world = World::new();
    let mut app = store_app(&world);
    let _ = app.update(Message::StoreSearch("other".to_owned()));
    {
        let mut ui = simulator(app.view());
        assert!(ui.find("Other Plugin").is_ok());
        assert!(ui.find("Test Plugin").is_err());
    }
    let _ = app.update(Message::StoreSearch("zzz".to_owned()));
    let mut ui = simulator(app.view());
    assert!(ui.find("No plugins match your search.").is_ok());
}

#[test]
fn detail_page_offers_the_right_action() {
    let world = World::new();
    let mut app = store_app(&world);
    let _ = app.update(Message::Navigate(Screen::PluginDetail("other".to_owned())));
    {
        let mut ui = simulator(app.view());
        assert!(ui.find("Version 0.1.0").is_ok());
        assert!(ui.find("Tags: fixture").is_ok());
        let _ = ui.click("Install");
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::InstallPlugin(id) if id == "other"))
        );
    }
    let _ = app.update(Message::Navigate(Screen::PluginDetail(
        "test-plugin".to_owned(),
    )));
    let mut ui = simulator(app.view());
    assert!(ui.find("Remove").is_ok());
    assert!(ui.find("Install").is_err());
}

#[test]
fn install_outcome_adds_the_plugin_to_the_rail() {
    let world = World::new();
    let mut app = store_app(&world);
    let _ = app.update(Message::InstallPlugin("other".to_owned()));
    let request = job_request(&app);

    let catalog = Catalog::parse(&fs::read_to_string(catalog_path(&world)).unwrap()).unwrap();
    let entry = catalog.entry("other").unwrap();
    install(entry, &world.plugins_dir(), &mut |_, _| {}).unwrap();
    let loaded = PluginRegistry::load_entry(&world.services.engine, &world.plugins_dir(), "other");
    assert!(matches!(loaded, PluginEntry::Loaded(_)));
    let _ = app.update(Message::JobFinished {
        request,
        result: Ok(JobOutcome::Installed {
            entry: loaded,
            name: "Other Plugin".to_owned(),
            version: Version::new(0, 1, 0),
            replaced: None,
        }),
    });
    assert!(matches!(
        app.registry().unwrap().get("other"),
        Some(PluginEntry::Loaded(_))
    ));
    assert_eq!(app.status(), Some("Installed Other Plugin 0.1.0."));
    assert!(app.store().job.is_none());
}

#[test]
fn update_outcome_says_updated() {
    let world = World::new();
    let mut app = store_app(&world);
    let _ = app.update(Message::InstallPlugin("other".to_owned()));
    let request = job_request(&app);
    let entry =
        PluginRegistry::load_entry(&world.services.engine, &world.plugins_dir(), "test-plugin");
    let _ = app.update(Message::JobFinished {
        request,
        result: Ok(JobOutcome::Installed {
            entry,
            name: "Test Plugin".to_owned(),
            version: Version::new(0, 2, 0),
            replaced: Some(Version::new(0, 1, 0)),
        }),
    });
    assert_eq!(app.status(), Some("Updated Test Plugin to 0.2.0."));
}

#[test]
fn remove_is_blocked_while_its_save_is_open() {
    let world = World::new();
    let mut app = store_app(&world);
    open_test_plugin_save(&world, &mut app);
    let _ = app.update(Message::RemovePlugin("test-plugin".to_owned()));
    assert_eq!(
        app.status(),
        Some("Close the open save before removing Test Plugin.")
    );
    assert!(app.store().job.is_none());
}

#[test]
fn remove_outcome_drops_plugin_and_selection() {
    let world = World::new();
    let mut app = store_app(&world);
    assert_eq!(app.home().selected.as_deref(), Some("test-plugin"));
    let _ = app.update(Message::RemovePlugin("test-plugin".to_owned()));
    let request = job_request(&app);
    let _ = app.update(Message::JobFinished {
        request,
        result: Ok(JobOutcome::Removed {
            id: "test-plugin".to_owned(),
            name: "Test Plugin".to_owned(),
        }),
    });
    assert!(app.registry().unwrap().get("test-plugin").is_none());
    assert_eq!(app.home().selected, None);
    assert_eq!(app.status(), Some("Removed Test Plugin."));
}

#[test]
fn second_job_is_ignored_while_one_runs() {
    let world = World::new();
    let mut app = store_app(&world);
    let _ = app.update(Message::InstallPlugin("other".to_owned()));
    let request = job_request(&app);
    let _ = app.update(Message::InstallPlugin("other".to_owned()));
    let _ = app.update(Message::RemovePlugin("test-plugin".to_owned()));
    assert_eq!(job_request(&app), request);
    assert_eq!(app.store().job.as_ref().unwrap().plugin_id, "other");
}

#[test]
fn job_failure_reports_and_clears_job() {
    let world = World::new();
    let mut app = store_app(&world);
    let _ = app.update(Message::InstallPlugin("other".to_owned()));
    let request = job_request(&app);
    let _ = app.update(Message::JobProgress {
        request,
        done: 50,
        total: Some(200),
    });
    assert_eq!(
        app.store().job.as_ref().unwrap().progress,
        Some((50, Some(200)))
    );
    let message = "package checksum mismatch: expected a, got b".to_owned();
    let _ = app.update(Message::JobFinished {
        request: request + 100,
        result: Err("stale".to_owned()),
    });
    assert!(app.store().job.is_some());
    let _ = app.update(Message::JobFinished {
        request,
        result: Err(message.clone()),
    });
    assert_eq!(app.status(), Some(message.as_str()));
    assert!(app.store().job.is_none());
}

#[test]
fn progress_fraction_handles_unknown_totals() {
    assert_eq!(progress_fraction(Some((50, Some(200)))), Some(0.25));
    assert_eq!(progress_fraction(Some((5, None))), None);
    assert_eq!(progress_fraction(Some((0, Some(0)))), None);
    assert_eq!(progress_fraction(None), None);
}

#[test]
fn startup_update_check_reports_updates() {
    let world = World::new();
    let mut s = settings(&world);
    s.check_updates_on_startup = true;
    let mut app = app_with(&world, s);
    assert_eq!(app.status(), Some("Checking for plugin updates…"));
    let update = Update {
        id: "test-plugin".to_owned(),
        name: "Test Plugin".to_owned(),
        installed: Version::new(0, 1, 0),
        latest: Version::new(0, 2, 0),
    };
    let _ = app.update(Message::UpdatesChecked {
        result: Ok((catalog_load(&world), vec![update])),
    });
    assert_eq!(
        app.info(),
        Some("Updates available for 1 plugin(s). Go to Store to update.")
    );
    assert_eq!(app.status(), None);
    assert!(matches!(
        app.store().catalog,
        saveforge_app::store::CatalogState::Ready(_)
    ));
}

#[test]
fn failed_update_check_points_to_settings() {
    let world = World::new();
    let mut s = settings(&world);
    s.check_updates_on_startup = true;
    let mut app = app_with(&world, s);
    let _ = app.update(Message::UpdatesChecked {
        result: Err("offline".to_owned()),
    });
    assert_eq!(
        app.status(),
        Some("Update check failed. Retry in Settings.")
    );
    assert_eq!(app.info(), None);
}
