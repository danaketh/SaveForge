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
