use saveforge_core::platform::Os;

#[test]
fn current_os_matches_compile_target() {
    let expected = if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else {
        Os::Linux
    };
    assert_eq!(Os::current(), expected);
}
