use std::fs;
use std::path::{Path, PathBuf};

use saveforge_core::platform::Os;
use saveforge_core::steam::SteamLibraries;
use saveforge_core::tokens::{Expanded, TokenEnv, expand};

fn env(os: Os, steam: Option<SteamLibraries>) -> TokenEnv {
    TokenEnv {
        os,
        appdata: Some(PathBuf::from(abs("/A"))),
        local_appdata: Some(PathBuf::from(abs("/L"))),
        home: Some(PathBuf::from(abs("/H"))),
        temp: PathBuf::from(abs("/T")),
        steam,
    }
}

/// Absolute on the host OS: `/A` stays `/A` on Unix, becomes `C:/A` on Windows.
fn abs(s: &str) -> String {
    if cfg!(windows) {
        format!("C:{s}")
    } else {
        s.to_owned()
    }
}

/// Expected resolved path; `s` starts with '/' and is made absolute via [`abs`].
fn resolved(s: &str) -> Expanded {
    Expanded::Resolved(PathBuf::from(abs(s)))
}

fn is_unresolved(e: &Expanded) -> bool {
    matches!(e, Expanded::Unresolved(_))
}

fn steam_with_app(lib: &Path, app_id: u32, install_dir: &str) -> (SteamLibraries, PathBuf) {
    let steamapps = lib.join("steamapps");
    fs::create_dir_all(steamapps.join("common").join(install_dir)).unwrap();
    fs::write(
        steamapps.join(format!("appmanifest_{app_id}.acf")),
        format!("\"AppState\" {{ \"installdir\" \"{install_dir}\" }}"),
    )
    .unwrap();
    (
        SteamLibraries::new(vec![lib.to_path_buf()]),
        steamapps.join("common").join(install_dir),
    )
}

#[test]
fn expands_known_percent_tokens() {
    let e = env(Os::Windows, None);
    assert_eq!(
        expand("%APPDATA%/MijuGames/Saves", &e),
        resolved("/A/MijuGames/Saves")
    );
    assert_eq!(expand("%LOCALAPPDATA%/x", &e), resolved("/L/x"));
    assert_eq!(expand("%USERPROFILE%/x", &e), resolved("/H/x"));
    assert_eq!(expand("%TEMP%/x", &e), resolved("/T/x"));
}

#[test]
fn percent_tokens_are_case_insensitive() {
    let e = env(Os::Windows, None);
    assert_eq!(expand("%appdata%/x", &e), resolved("/A/x"));
    assert_eq!(expand("%AppData%/x", &e), resolved("/A/x"));
}

#[test]
fn leading_tilde_expands_to_home_only_as_whole_segment() {
    let e = env(Os::Linux, None);
    assert_eq!(expand("~", &e), resolved("/H"));
    assert_eq!(expand("~/saves", &e), resolved("/H/saves"));
    assert_eq!(expand("~\\saves", &e), resolved("/H\\saves"));
}

#[test]
fn unknown_tokens_are_left_verbatim() {
    let e = env(Os::Windows, None);
    assert_eq!(expand("%APPDATA%/%FOO%/x", &e), resolved("/A/%FOO%/x"));
    assert_eq!(expand("%APPDATA%/{foo}/x", &e), resolved("/A/{foo}/x"));
    assert_eq!(
        expand("%APPDATA%/{steam_app:abc}", &e),
        resolved("/A/{steam_app:abc}")
    );
    assert_eq!(expand("%APPDATA%/100% sure", &e), resolved("/A/100% sure"));
    assert_eq!(expand("%APPDATA%/%APPDATA", &e), resolved("/A/%APPDATA"));
}

#[test]
fn relative_results_are_unresolved() {
    let e = env(Os::Windows, None);
    for raw in [
        "saves/x",
        "%FOO%/x",
        "{foo}/x",
        "~user/saves",
        "a/~/b",
        "100% sure",
    ] {
        assert!(is_unresolved(&expand(raw, &e)), "{raw}");
    }
}

#[cfg(windows)]
#[test]
fn windows_drive_relative_and_rooted_paths_are_unresolved() {
    let e = env(Os::Windows, None);
    for raw in ["C:foo", r"\foo", "/foo"] {
        assert!(is_unresolved(&expand(raw, &e)), "{raw}");
    }
}

#[test]
fn trims_and_rejects_empty() {
    let e = env(Os::Windows, None);
    assert_eq!(expand("  %APPDATA%/x  ", &e), resolved("/A/x"));
    assert!(is_unresolved(&expand("", &e)));
    assert!(is_unresolved(&expand("   ", &e)));
}

#[test]
fn missing_base_dir_makes_path_unresolved() {
    let mut e = env(Os::Windows, None);
    e.appdata = None;
    e.home = None;
    assert!(is_unresolved(&expand("%APPDATA%/x", &e)));
    assert!(is_unresolved(&expand("~/x", &e)));
    assert_eq!(expand("%TEMP%/x", &e), resolved("/T/x"));
}

#[test]
fn steam_app_token() {
    let lib = tempfile::tempdir().unwrap();
    let (steam, dir) = steam_with_app(lib.path(), 1_284_190, "Planet Crafter");
    let e = env(Os::Windows, Some(steam));
    assert_eq!(
        expand("{steam_app:1284190}/Data/loc.txt", &e),
        Expanded::Resolved(PathBuf::from(format!("{}/Data/loc.txt", dir.display())))
    );
    assert!(is_unresolved(&expand("{steam_app:42}/x", &e)));
    assert!(is_unresolved(&expand(
        "{steam_app:1284190}/x",
        &env(Os::Windows, None)
    )));
}

#[test]
fn steam_compat_token_only_on_linux() {
    let lib = tempfile::tempdir().unwrap();
    let user = lib
        .path()
        .join("steamapps/compatdata/1284190/pfx/drive_c/users/steamuser");
    fs::create_dir_all(&user).unwrap();
    let steam = SteamLibraries::new(vec![lib.path().to_path_buf()]);

    let linux = env(Os::Linux, Some(steam.clone()));
    assert_eq!(
        expand("{steam_compat:1284190}/AppData", &linux),
        Expanded::Resolved(PathBuf::from(format!("{}/AppData", user.display())))
    );
    assert!(is_unresolved(&expand(
        "{steam_compat:1284190}/AppData",
        &env(Os::Windows, Some(steam))
    )));
}

#[test]
fn detect_produces_usable_env() {
    let e = TokenEnv::detect();
    assert_eq!(e.os, Os::current());
    assert!(e.home.is_some());
    assert!(matches!(expand("%TEMP%", &e), Expanded::Resolved(_)));
}
