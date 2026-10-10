use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::platform::Os;
use saveforge_core::steam::{
    SteamLibraries, Vdf, find_steam_root, library_paths_from_vdf, parse_vdf,
};

fn vdf_escape(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "\\\\")
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Creates `<lib>/steamapps/appmanifest_<id>.acf` and `<lib>/steamapps/common/<dir>/`.
fn install_app(lib: &Path, app_id: u32, install_dir: &str) -> PathBuf {
    write(
        &lib.join("steamapps")
            .join(format!("appmanifest_{app_id}.acf")),
        &format!(
            "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"installdir\"\t\t\"{install_dir}\"\n}}\n"
        ),
    );
    let dir = lib.join("steamapps").join("common").join(install_dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn parses_nested_vdf_with_escapes_comments_and_conditionals() {
    let text = r#"
        // comment
        "Root"
        {
            "Name"   "a \"quoted\" C:\\path"
            "Child" { "Inner" "1" }
            "Flag"   "1" [$WIN32]
            Unquoted value
        }
    "#;
    let root = parse_vdf(text).unwrap();
    let r = root.get("root").unwrap();
    assert_eq!(
        r.get("name").and_then(Vdf::as_str),
        Some("a \"quoted\" C:\\path")
    );
    assert_eq!(
        r.get("Child")
            .and_then(|c| c.get("Inner"))
            .and_then(Vdf::as_str),
        Some("1")
    );
    assert_eq!(r.get("Flag").and_then(Vdf::as_str), Some("1"));
    assert_eq!(r.get("Unquoted").and_then(Vdf::as_str), Some("value"));
}

#[test]
fn rejects_malformed_vdf() {
    assert_eq!(parse_vdf("\"a\" { \"b\" \"c\""), None);
    assert_eq!(parse_vdf("\"a\" }"), None);
    assert_eq!(parse_vdf("\"unterminated"), None);
}

#[test]
fn library_paths_new_and_old_format() {
    let new = r#""libraryfolders" {
        "0" { "path" "C:\\Program Files (x86)\\Steam" "apps" { "228980" "1" } }
        "1" { "path" "D:\\SteamLibrary" }
        "contentstatsid" "123"
    }"#;
    assert_eq!(
        library_paths_from_vdf(new),
        vec![
            PathBuf::from("C:\\Program Files (x86)\\Steam"),
            PathBuf::from("D:\\SteamLibrary")
        ]
    );
    let old = r#""LibraryFolders" { "TimeNextStatsReport" "1" "1" "/mnt/games/Steam" }"#;
    assert_eq!(
        library_paths_from_vdf(old),
        vec![PathBuf::from("/mnt/games/Steam")]
    );
    assert_eq!(library_paths_from_vdf("garbage {"), Vec::<PathBuf>::new());
}

#[test]
fn load_includes_root_and_listed_libraries_without_duplicates() {
    let root = tempfile::tempdir().unwrap();
    let extra = tempfile::tempdir().unwrap();
    write(
        &root.path().join("steamapps/libraryfolders.vdf"),
        &format!(
            "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
            vdf_escape(root.path()),
            vdf_escape(extra.path())
        ),
    );
    let libs = SteamLibraries::load(root.path()).unwrap();
    assert_eq!(
        libs.libraries(),
        &[root.path().to_path_buf(), extra.path().to_path_buf()]
    );
}

#[test]
fn load_without_vdf_uses_root_only_and_missing_root_is_none() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        SteamLibraries::load(root.path()).unwrap().libraries(),
        &[root.path().to_path_buf()]
    );
    assert_eq!(SteamLibraries::load(&root.path().join("nope")), None);
}

#[test]
fn app_dir_searches_all_libraries() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let installed = install_app(b.path(), 1_284_190, "Planet Crafter");
    let libs = SteamLibraries::new(vec![a.path().to_path_buf(), b.path().to_path_buf()]);
    assert_eq!(libs.app_dir(1_284_190), Some(installed));
    assert_eq!(libs.app_dir(42), None);
}

#[test]
fn app_dir_ignores_manifest_without_install_dir_on_disk() {
    let lib = tempfile::tempdir().unwrap();
    let dir = install_app(lib.path(), 7, "Gone");
    fs::remove_dir(&dir).unwrap();
    assert_eq!(
        SteamLibraries::new(vec![lib.path().to_path_buf()]).app_dir(7),
        None
    );
}

#[test]
fn compat_user_dir_finds_proton_prefix() {
    let lib = tempfile::tempdir().unwrap();
    let user = lib
        .path()
        .join("steamapps/compatdata/1284190/pfx/drive_c/users/steamuser");
    fs::create_dir_all(&user).unwrap();
    let libs = SteamLibraries::new(vec![lib.path().to_path_buf()]);
    assert_eq!(libs.compat_user_dir(1_284_190), Some(user));
    assert_eq!(libs.compat_user_dir(1), None);
}

#[test]
fn find_steam_root_linux_prefers_dot_steam() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(find_steam_root(Os::Linux, home.path()), None);
    let share = home.path().join(".local/share/Steam");
    fs::create_dir_all(&share).unwrap();
    assert_eq!(find_steam_root(Os::Linux, home.path()), Some(share));
    let dot = home.path().join(".steam/steam");
    fs::create_dir_all(&dot).unwrap();
    assert_eq!(find_steam_root(Os::Linux, home.path()), Some(dot));
}

#[test]
fn find_steam_root_macos() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("Library/Application Support/Steam");
    fs::create_dir_all(&root).unwrap();
    assert_eq!(find_steam_root(Os::MacOs, home.path()), Some(root));
}
