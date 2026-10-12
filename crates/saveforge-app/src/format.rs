//! Display formatting for sizes, dates and the window title.

use std::time::SystemTime;

use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

/// `512 B`, `1.5 KB`, `2.0 MB`, `1.2 GB` (1024-based).
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    // Display only: precision loss above 2^53 bytes is irrelevant.
    #[allow(clippy::cast_precision_loss)]
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// `2026-10-10 14:03` in the given offset.
pub fn format_modified(modified: SystemTime, offset: UtcOffset) -> String {
    OffsetDateTime::from(modified)
        .to_offset(offset)
        .format(format_description!("[year]-[month]-[day] [hour]:[minute]"))
        .unwrap_or_default()
}

/// `SaveForge`, `SaveForge — file.json` or `SaveForge — file.json •` when dirty.
pub fn window_title(file_name: Option<&str>, dirty: bool) -> String {
    match file_name {
        None => "SaveForge".to_owned(),
        Some(name) if dirty => format!("SaveForge — {name} •"),
        Some(name) => format!("SaveForge — {name}"),
    }
}
