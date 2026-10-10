//! Steam library discovery used by the `{steam_app:ID}` and `{steam_compat:ID}`
//! path tokens (spec §4.3).

mod vdf;

use std::fs;
use std::path::{Path, PathBuf};

pub use vdf::{Vdf, parse as parse_vdf};

use crate::platform::Os;

/// All Steam library folders on this machine, Steam root first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamLibraries {
    libraries: Vec<PathBuf>,
}

impl SteamLibraries {
    pub fn new(libraries: Vec<PathBuf>) -> Self {
        Self { libraries }
    }

    /// Reads `<root>/steamapps/libraryfolders.vdf`. Returns `None` if `steam_root`
    /// is not a directory. A missing or broken VDF yields just the root.
    pub fn load(steam_root: &Path) -> Option<Self> {
        if !steam_root.is_dir() {
            return None;
        }
        let mut libraries = vec![steam_root.to_path_buf()];
        let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(&vdf_path) {
            for path in library_paths_from_vdf(&text) {
                if !libraries.iter().any(|known| same_path(known, &path)) {
                    libraries.push(path);
                }
            }
        }
        Some(Self { libraries })
    }

    pub fn libraries(&self) -> &[PathBuf] {
        &self.libraries
    }

    /// `<lib>/steamapps/common/<installdir>` of the first library whose
    /// `appmanifest_<id>.acf` names an install dir that exists.
    pub fn app_dir(&self, app_id: u32) -> Option<PathBuf> {
        self.libraries.iter().find_map(|lib| {
            let steamapps = lib.join("steamapps");
            let text =
                fs::read_to_string(steamapps.join(format!("appmanifest_{app_id}.acf"))).ok()?;
            let root = vdf::parse(&text)?;
            let install_dir = root.get("AppState")?.get("installdir")?.as_str()?;
            let dir = steamapps.join("common").join(install_dir);
            dir.is_dir().then_some(dir)
        })
    }

    /// Proton prefix user dir: `<lib>/steamapps/compatdata/<id>/pfx/drive_c/users/steamuser`.
    pub fn compat_user_dir(&self, app_id: u32) -> Option<PathBuf> {
        self.libraries
            .iter()
            .map(|lib| {
                lib.join("steamapps")
                    .join("compatdata")
                    .join(app_id.to_string())
                    .join("pfx")
                    .join("drive_c")
                    .join("users")
                    .join("steamuser")
            })
            .find(|p| p.is_dir())
    }
}

/// Library paths listed in `libraryfolders.vdf`, in file order. Handles both
/// the current format (`"0" { "path" "..." }`) and the legacy one (`"1" "..."`).
pub fn library_paths_from_vdf(text: &str) -> Vec<PathBuf> {
    let Some(root) = vdf::parse(text) else {
        return Vec::new();
    };
    let Some(Vdf::Obj(entries)) = root.get("libraryfolders") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|(key, _)| !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|(_, value)| match value {
            Vdf::Str(path) => Some(PathBuf::from(path)),
            Vdf::Obj(_) => value.get("path").and_then(Vdf::as_str).map(PathBuf::from),
        })
        .collect()
}

/// First existing Steam root for `os`.
pub fn find_steam_root(os: Os, home: &Path) -> Option<PathBuf> {
    root_candidates(os, home).into_iter().find(|p| p.is_dir())
}

fn root_candidates(os: Os, home: &Path) -> Vec<PathBuf> {
    match os {
        Os::Windows => {
            let mut candidates = Vec::new();
            if let Some(path) = registry_steam_path() {
                candidates.push(path);
            }
            if let Some(pf) = std::env::var_os("ProgramFiles(x86)") {
                candidates.push(PathBuf::from(pf).join("Steam"));
            }
            candidates
        }
        Os::Linux => vec![
            home.join(".steam").join("steam"),
            home.join(".local/share/Steam"),
        ],
        Os::MacOs => vec![home.join("Library/Application Support/Steam")],
    }
}

#[cfg(windows)]
fn registry_steam_path() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(not(windows))]
fn registry_steam_path() -> Option<PathBuf> {
    None
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        p.to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase()
    };
    norm(a) == norm(b)
}
