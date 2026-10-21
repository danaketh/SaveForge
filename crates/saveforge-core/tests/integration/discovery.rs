use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use filetime::{FileTime, set_file_mtime};
use saveforge_core::discovery::discover;

fn json() -> Vec<String> {
    vec![".json".to_owned()]
}

fn touch(path: &Path, bytes: &[u8], secs_after_epoch: u64) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs_after_epoch);
    set_file_mtime(path, FileTime::from_system_time(t)).unwrap();
}

#[test]
fn finds_matching_files_recursively_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("old.json"), b"1", 1_000);
    touch(&dir.path().join("nested/deeper/new.json"), b"22", 3_000);
    touch(&dir.path().join("mid.json"), b"333", 2_000);
    touch(&dir.path().join("ignored.txt"), b"x", 4_000);

    let found = discover(dir.path(), &json());
    let names: Vec<_> = found.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, ["new.json", "mid.json", "old.json"]);
    assert_eq!(found[0].path, dir.path().join("nested/deeper/new.json"));
    assert_eq!(found[0].size_bytes, 2);
    assert_eq!(
        found[0].modified,
        SystemTime::UNIX_EPOCH + Duration::from_mins(50)
    );
}

#[test]
fn extension_filter_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("SAVE.JSON"), b"{}", 1);
    touch(&dir.path().join("save.json.bak"), b"{}", 2);
    let found = discover(dir.path(), &json());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].file_name, "SAVE.JSON");
}

#[test]
fn equal_mtimes_sort_by_path() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("b.json"), b"", 5);
    touch(&dir.path().join("a.json"), b"", 5);
    let names: Vec<_> = discover(dir.path(), &json())
        .into_iter()
        .map(|f| f.file_name)
        .collect();
    assert_eq!(names, ["a.json", "b.json"]);
}

#[test]
fn missing_dir_or_file_path_yields_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(discover(&dir.path().join("nope"), &json()), vec![]);
    touch(&dir.path().join("f.json"), b"", 1);
    assert_eq!(discover(&dir.path().join("f.json"), &json()), vec![]);
}

#[test]
fn multiple_extensions() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("a.sav"), b"", 1);
    touch(&dir.path().join("b.json"), b"", 2);
    let found = discover(dir.path(), &[".sav".to_owned(), ".json".to_owned()]);
    assert_eq!(found.len(), 2);
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlinked_dirs() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("sub/a.json"), b"", 1);
    std::os::unix::fs::symlink(dir.path(), dir.path().join("sub/loop")).unwrap();
    assert_eq!(discover(dir.path(), &json()).len(), 1);
}
