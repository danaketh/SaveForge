use std::fs;
use std::io;
use std::path::Path;

use saveforge_core::CoreError;
use saveforge_core::backup::{TMP_SUFFIX, save_with_backup, save_with_backup_at, write_atomic};
use time::macros::datetime;

fn real_rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[test]
fn backs_up_original_then_replaces_content() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("Standard-1.json");
    fs::write(&target, b"old").unwrap();
    let backups = dir.path().join("backups");

    let backup = save_with_backup_at(
        &target,
        b"new",
        &backups,
        "planet-crafter",
        datetime!(2026-10-10 12:30:45 UTC),
        real_rename,
    )
    .unwrap();

    assert_eq!(
        backup,
        backups
            .join("planet-crafter")
            .join("Standard-1.json.2026-10-10T12-30-45Z.bak")
    );
    assert_eq!(fs::read(&backup).unwrap(), b"old");
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert!(
        !dir.path()
            .join(format!("Standard-1.json{TMP_SUFFIX}"))
            .exists()
    );
}

#[test]
fn second_backup_in_same_second_gets_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("s.json");
    fs::write(&target, b"v1").unwrap();
    let now = datetime!(2026-10-10 12:30:45 UTC);
    let first = save_with_backup_at(&target, b"v2", dir.path(), "p", now, real_rename).unwrap();
    let second = save_with_backup_at(&target, b"v3", dir.path(), "p", now, real_rename).unwrap();
    let third = save_with_backup_at(&target, b"v4", dir.path(), "p", now, real_rename).unwrap();

    assert_eq!(
        second.file_name().unwrap(),
        "s.json.2026-10-10T12-30-45Z-1.bak"
    );
    assert_eq!(
        third.file_name().unwrap(),
        "s.json.2026-10-10T12-30-45Z-2.bak"
    );
    assert_eq!(fs::read(first).unwrap(), b"v1");
    assert_eq!(fs::read(second).unwrap(), b"v2");
    assert_eq!(fs::read(&target).unwrap(), b"v4");
}

#[test]
fn failed_rename_leaves_original_untouched_and_removes_tmp() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("s.json");
    fs::write(&target, b"original").unwrap();

    let result = save_with_backup_at(
        &target,
        b"new",
        &dir.path().join("b"),
        "p",
        datetime!(2026-01-01 00:00:00 UTC),
        |_, _| Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked")),
    );

    assert!(matches!(result, Err(CoreError::Io { .. })));
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert!(!dir.path().join(format!("s.json{TMP_SUFFIX}")).exists());
}

#[test]
fn missing_target_fails_before_writing_anything() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("missing.json");
    let result = save_with_backup(&target, b"x", &dir.path().join("b"), "p");
    assert!(matches!(result, Err(CoreError::Io { .. })));
    assert!(!target.exists());
    assert!(
        !dir.path()
            .join(format!("missing.json{TMP_SUFFIX}"))
            .exists()
    );
}

#[test]
fn write_atomic_creates_and_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("settings.json");
    write_atomic(&target, b"a").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"a");
    write_atomic(&target, b"bb").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"bb");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
