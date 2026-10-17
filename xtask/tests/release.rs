//! The release workflow publishes with a write token, so it must not run
//! third-party code from movable tags or publish from manual runs.

const RELEASE: &str = include_str!("../../.github/workflows/release.yml");

#[test]
fn release_token_is_read_only_by_default() {
    assert!(
        RELEASE.contains("\npermissions:\n  contents: read\n"),
        "missing workflow-level read-only permissions"
    );
}

#[test]
fn third_party_actions_are_pinned_to_commits() {
    for line in RELEASE.lines().filter(|l| l.contains("uses:")) {
        let action = line.split("uses:").nth(1).unwrap().trim();
        if action.starts_with("actions/") {
            continue;
        }
        let reference = action
            .split('@')
            .nth(1)
            .and_then(|r| r.split_whitespace().next())
            .unwrap_or_default();
        assert!(
            reference.len() == 40 && reference.bytes().all(|b| b.is_ascii_hexdigit()),
            "{line}"
        );
    }
}

#[test]
fn only_tag_pushes_publish() {
    assert!(
        RELEASE.contains("if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')"),
        "publish must be limited to tag pushes"
    );
}

#[test]
fn unix_binaries_ship_as_tarballs() {
    for needle in [
        "saveforge-linux-x64.tar.gz",
        "saveforge-macos-arm64.tar.gz",
        "tar -czf",
    ] {
        assert!(RELEASE.contains(needle), "missing {needle}");
    }
}

#[test]
fn releases_refuse_placeholder_urls() {
    assert!(RELEASE.contains(
        "::error::Replace YOUR_ORG/YOUR_REPO in RELEASE_CATALOG_URL and REPOSITORY_URL before releasing"
    ));
    assert!(RELEASE.contains("crates/saveforge-core/src/settings.rs"));
    assert!(RELEASE.contains("crates/saveforge-app/src/app/mod.rs"));
    assert!(RELEASE.contains("exit 1"));
}
