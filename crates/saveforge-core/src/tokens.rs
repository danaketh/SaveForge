//! Path token expansion (spec §4.3).
//!
//! Supported: `%APPDATA%`, `%LOCALAPPDATA%`, `%USERPROFILE%`, `%TEMP%` (case-insensitive),
//! a leading `~`, `{steam_app:ID}` and `{steam_compat:ID}` (Linux only). Unknown tokens
//! are left verbatim. A known token that cannot be resolved, or a result that is
//! not an absolute path, makes the whole path unresolved.

use std::path::PathBuf;

use crate::platform::Os;
use crate::steam::{SteamLibraries, find_steam_root};

/// Machine facts used for expansion. Build with [`TokenEnv::detect`] or by hand in tests.
#[derive(Debug, Clone)]
pub struct TokenEnv {
    pub os: Os,
    pub appdata: Option<PathBuf>,
    pub local_appdata: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub temp: PathBuf,
    pub steam: Option<SteamLibraries>,
}

impl TokenEnv {
    pub fn detect() -> Self {
        let os = Os::current();
        let base = directories::BaseDirs::new();
        let home = base.as_ref().map(|b| b.home_dir().to_path_buf());
        let steam = home
            .as_deref()
            .and_then(|h| find_steam_root(os, h))
            .and_then(|root| SteamLibraries::load(&root));
        Self {
            os,
            appdata: base.as_ref().map(|b| b.config_dir().to_path_buf()),
            local_appdata: base.as_ref().map(|b| b.data_local_dir().to_path_buf()),
            home,
            temp: std::env::temp_dir(),
            steam,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expanded {
    Resolved(PathBuf),
    /// Human-readable reason, shown in the "path not set" state.
    Unresolved(String),
}

enum Token {
    Value(PathBuf),
    Unknown,
    Missing(String),
}

pub fn expand(raw: &str, env: &TokenEnv) -> Expanded {
    let raw = raw.trim();
    if raw.is_empty() {
        return Expanded::Unresolved("path is empty".to_owned());
    }

    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;

    if let Some(after) = rest.strip_prefix('~')
        && (after.is_empty() || after.starts_with(['/', '\\']))
    {
        let Some(home) = &env.home else {
            return Expanded::Unresolved("home directory is unknown".to_owned());
        };
        out.push_str(&home.to_string_lossy());
        rest = after;
    }

    while let Some(i) = rest.find(['%', '{']) {
        out.push_str(&rest[..i]);
        let percent = rest[i..].starts_with('%');
        let close = if percent { '%' } else { '}' };
        let tail = &rest[i + 1..];
        let token = tail.find(close).map(|j| {
            let name = &tail[..j];
            let token = if percent {
                resolve_percent(name, env)
            } else {
                resolve_brace(name, env)
            };
            (token, &tail[j + 1..])
        });
        match token {
            Some((Token::Value(path), after)) => {
                out.push_str(&path.to_string_lossy());
                rest = after;
            }
            Some((Token::Missing(reason), _)) => return Expanded::Unresolved(reason),
            Some((Token::Unknown, _)) | None => {
                // Keep the opening char and keep scanning right after it, so
                // "%FOO%/%APPDATA%" still expands the second token.
                out.push(if percent { '%' } else { '{' });
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    let path = PathBuf::from(out);
    // A relative result would resolve against the process working directory,
    // which is arbitrary for a GUI app. On Windows this also rejects `C:foo`
    // and `\foo`.
    if !path.is_absolute() {
        return Expanded::Unresolved(format!("'{}' is not an absolute path", path.display()));
    }
    Expanded::Resolved(path)
}

fn resolve_percent(name: &str, env: &TokenEnv) -> Token {
    let dir = match name.to_ascii_uppercase().as_str() {
        "APPDATA" => env.appdata.as_ref(),
        "LOCALAPPDATA" => env.local_appdata.as_ref(),
        "USERPROFILE" => env.home.as_ref(),
        "TEMP" => Some(&env.temp),
        _ => return Token::Unknown,
    };
    dir.map_or_else(
        || Token::Missing(format!("%{name}% is not available on this system")),
        |d| Token::Value(d.clone()),
    )
}

fn resolve_brace(name: &str, env: &TokenEnv) -> Token {
    let Some((kind, id)) = name.split_once(':') else {
        return Token::Unknown;
    };
    let Ok(app_id) = id.parse::<u32>() else {
        return Token::Unknown;
    };
    match kind {
        "steam_app" => env
            .steam
            .as_ref()
            .and_then(|s| s.app_dir(app_id))
            .map_or_else(
                || Token::Missing(format!("Steam app {app_id} is not installed")),
                Token::Value,
            ),
        "steam_compat" if env.os != Os::Linux => {
            Token::Missing("Proton prefixes only exist on Linux".to_owned())
        }
        "steam_compat" => env
            .steam
            .as_ref()
            .and_then(|s| s.compat_user_dir(app_id))
            .map_or_else(
                || Token::Missing(format!("no Proton prefix for Steam app {app_id}")),
                Token::Value,
            ),
        _ => Token::Unknown,
    }
}
