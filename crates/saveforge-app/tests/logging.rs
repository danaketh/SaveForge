use saveforge_app::logging::write_crash_report;

#[test]
fn crash_report_creates_dirs_and_replaces_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deep/crash.log");
    write_crash_report(&path, "first").unwrap();
    write_crash_report(&path, "boom at x.rs:1").unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("SaveForge "), "{text}");
    assert!(text.contains("crashed."));
    assert!(text.contains("boom at x.rs:1"));
    assert!(!text.contains("first"));
}
