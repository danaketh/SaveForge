use std::fs;
use std::path::Path;

use saveforge_core::manifest::PluginManifest;
use semver::Version;
use xtask::scaffold::{
    API_VERSION, NewPlugin, add_wasm_clippy_package, add_workspace_member, scaffold,
};

const WORKSPACE: &str = "[workspace]\nresolver = \"3\"\nmembers = [\n    \"crates/a\",\n    \"plugins/b\",\n    \"xtask\",\n]\n\n[workspace.package]\nversion = \"0.1.0\"\n";

const CI: &str = "jobs:\n  lint:\n    steps:\n      - run: cargo fmt --all --check\n      - run: cargo clippy -p saveforge-sdk -p planet-crafter --target wasm32-wasip2 -- -D warnings\n";

fn workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), WORKSPACE).unwrap();
    let workflows = root.path().join(".github/workflows");
    fs::create_dir_all(&workflows).unwrap();
    fs::write(workflows.join("ci.yml"), CI).unwrap();
    root
}

fn files_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found
}

#[test]
fn plugin_defaults_from_id() {
    let plugin = NewPlugin::new("my-game").unwrap();
    assert_eq!(plugin.id, "my-game");
    assert_eq!(plugin.name, "My Game Save Editor");
    assert_eq!(plugin.extension, ".json");
    assert_eq!(plugin.author, "Unknown");
    assert_eq!(plugin.crate_name(), "my_game");
    assert_eq!(plugin.type_name(), "MyGame");
}

#[test]
fn invalid_ids_are_rejected() {
    for id in ["My Game", "../x", "a--b", "", "7days"] {
        assert!(NewPlugin::new(id).is_err(), "{id:?}");
    }
    assert_eq!(
        NewPlugin::new("My Game").unwrap_err().to_string(),
        "invalid plugin id 'My Game': use lowercase letters, digits and single hyphens, e.g. my-game"
    );
}

#[test]
fn members_are_inserted_in_order() {
    let updated = add_workspace_member(WORKSPACE, "plugins/a2").unwrap();
    assert!(
        updated.contains(
            "members = [\n    \"crates/a\",\n    \"plugins/a2\",\n    \"plugins/b\",\n    \"xtask\",\n]"
        ),
        "{updated}"
    );

    let crlf = WORKSPACE.replace('\n', "\r\n");
    let updated = add_workspace_member(&crlf, "plugins/a2").unwrap();
    assert!(updated.contains("    \"plugins/a2\",\r\n"), "{updated:?}");
    assert_eq!(updated.replace("\r\n", "").matches('\n').count(), 0);

    let err = add_workspace_member(WORKSPACE, "plugins/b").unwrap_err();
    assert_eq!(err.to_string(), "plugins/b is already a workspace member");
    assert!(add_workspace_member("[workspace]\nmembers = [\"a\"]\n", "plugins/x").is_err());
}

#[test]
fn ci_clippy_line_gets_the_package() {
    let updated = add_wasm_clippy_package(CI, "my-game").unwrap();
    assert!(
        updated.contains("-p planet-crafter -p my-game --target wasm32-wasip2"),
        "{updated}"
    );
    assert_eq!(add_wasm_clippy_package("jobs: {}\n", "my-game"), None);
    assert_eq!(add_wasm_clippy_package(&updated, "my-game"), None);
}

#[test]
fn scaffold_writes_a_valid_plugin() {
    let root = workspace();
    let dir = scaffold(root.path(), &NewPlugin::new("my-game").unwrap()).unwrap();
    assert_eq!(dir, root.path().join("plugins/my-game"));
    for file in [
        "Cargo.toml",
        "manifest.json",
        "src/lib.rs",
        "tests/my_game.rs",
    ] {
        assert!(dir.join(file).is_file(), "{file}");
    }

    let manifest =
        PluginManifest::parse(&fs::read_to_string(dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.id, "my-game");
    assert_eq!(manifest.version, Version::new(0, 1, 0));
    assert_eq!(manifest.file_extensions, [".json"]);
    manifest
        .check_compat(
            &Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            &Version::new(0, 1, 0),
        )
        .unwrap();
    assert!(
        fs::read_to_string(dir.join("Cargo.toml"))
            .unwrap()
            .contains("name = \"my-game\"")
    );

    let workspace_toml = fs::read_to_string(root.path().join("Cargo.toml")).unwrap();
    assert!(
        workspace_toml.contains("\"plugins/my-game\","),
        "{workspace_toml}"
    );
    let ci = fs::read_to_string(root.path().join(".github/workflows/ci.yml")).unwrap();
    assert!(ci.contains("-p my-game"), "{ci}");

    for file in files_under(&dir) {
        let text = fs::read_to_string(&file).unwrap();
        assert!(!text.contains("TODO"), "{}", file.display());
        assert!(!text.contains("{{"), "{}", file.display());
    }
}

#[test]
fn existing_plugin_dir_is_left_alone() {
    let root = workspace();
    let marker = root.path().join("plugins/my-game/marker.txt");
    fs::create_dir_all(marker.parent().unwrap()).unwrap();
    fs::write(&marker, b"keep").unwrap();

    let err = scaffold(root.path(), &NewPlugin::new("my-game").unwrap()).unwrap_err();
    assert_eq!(err.to_string(), "plugins/my-game already exists");
    assert_eq!(fs::read(&marker).unwrap(), b"keep");
    assert_eq!(
        fs::read_to_string(root.path().join("Cargo.toml")).unwrap(),
        WORKSPACE
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".github/workflows/ci.yml")).unwrap(),
        CI
    );
}

#[test]
fn api_version_matches_the_host() {
    let host = saveforge_plugin_api::API_VERSION;
    assert_eq!(API_VERSION, format!("{}.{}", host.major, host.minor));
}

#[test]
fn display_name_stays_out_of_doc_comments() {
    // clippy::doc_markdown rejects CamelCase words like "RimWorld" in docs.
    let mut plugin = NewPlugin::new("rimworld").unwrap();
    plugin.name = "RimWorld Save Editor".to_owned();
    let root = workspace();
    let dir = scaffold(root.path(), &plugin).unwrap();
    for file in files_under(&dir) {
        for line in fs::read_to_string(&file).unwrap().lines() {
            if line.trim_start().starts_with("//") {
                assert!(!line.contains("RimWorld"), "{}: {line}", file.display());
            }
        }
    }
}

fn assert_untouched(root: &Path) {
    assert_eq!(
        fs::read_to_string(root.join("Cargo.toml")).unwrap(),
        WORKSPACE
    );
    assert_eq!(
        fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap(),
        CI
    );
}

#[test]
fn ids_that_clash_are_rejected() {
    for id in ["async", "gen", "std", "self"] {
        assert_eq!(
            NewPlugin::new(id).unwrap_err().to_string(),
            format!("'{id}' is a reserved word in Rust")
        );
    }
    let root = workspace();
    fs::create_dir_all(root.path().join("fixtures/test-plugin")).unwrap();
    for (id, used_by) in [
        ("xtask", "xtask"),
        ("b", "plugins/b"),
        ("test-plugin", "fixtures/test-plugin"),
    ] {
        let err = scaffold(root.path(), &NewPlugin::new(id).unwrap()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("'{id}' is already used by {used_by}")
        );
    }
    assert_untouched(root.path());
}

#[test]
fn bad_names_and_extensions_are_rejected() {
    let root = workspace();
    let cases: [(&str, &str, &str); 7] = [
        ("", ".json", "Me"),
        ("A\nB", ".json", "Me"),
        ("Name", ".json", ""),
        ("Name", "", "Me"),
        ("Name", ".", "Me"),
        ("Name", "a b", "Me"),
        ("Name", ".s v", "Me"),
    ];
    for (name, extension, author) in cases {
        let mut plugin = NewPlugin::new("my-game").unwrap();
        plugin.name = name.to_owned();
        plugin.extension = extension.to_owned();
        plugin.author = author.to_owned();
        assert!(
            scaffold(root.path(), &plugin).is_err(),
            "{name:?} {extension:?} {author:?}"
        );
        assert!(!root.path().join("plugins/my-game").exists());
    }
    assert_untouched(root.path());
}

#[test]
fn scaffold_is_all_or_nothing() {
    let root = workspace();
    fs::write(root.path().join("plugins"), b"not a directory").unwrap();
    scaffold(root.path(), &NewPlugin::new("my-game").unwrap()).unwrap_err();
    assert_untouched(root.path());
    let leftovers: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".new-"))
        .collect();
    assert_eq!(leftovers.len(), 0);
}
