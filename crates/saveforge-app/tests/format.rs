use std::time::{Duration, SystemTime};

use saveforge_app::format::{format_modified, human_size, window_title};
use time::UtcOffset;

#[test]
fn human_sizes() {
    assert_eq!(human_size(0), "0 B");
    assert_eq!(human_size(1023), "1023 B");
    assert_eq!(human_size(1024), "1.0 KB");
    assert_eq!(human_size(1536), "1.5 KB");
    assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    assert_eq!(human_size(3 * 1024 * 1024 * 1024), "3.0 GB");
}

#[test]
fn modified_time_in_offset() {
    // 2026-10-10 14:03:00 UTC
    let t = SystemTime::UNIX_EPOCH + Duration::from_mins(29_860_683);
    assert_eq!(format_modified(t, UtcOffset::UTC), "2026-10-10 14:03");
    let plus_two = UtcOffset::from_hms(2, 0, 0).unwrap();
    assert_eq!(format_modified(t, plus_two), "2026-10-10 16:03");
}

#[test]
fn titles() {
    assert_eq!(window_title(None, false), "SaveForge");
    assert_eq!(window_title(None, true), "SaveForge");
    assert_eq!(window_title(Some("a.json"), false), "SaveForge — a.json");
    assert_eq!(window_title(Some("a.json"), true), "SaveForge — a.json •");
}
