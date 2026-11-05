//! Shared view pieces.

use saveforge_app::components::monogram;

#[test]
fn monogram_uses_initials_of_the_first_two_words() {
    assert_eq!(monogram("Planet Crafter"), "PC");
    assert_eq!(monogram("Baldur's Gate 3"), "BG");
    assert_eq!(monogram("sample-stardew"), "SS");
}

#[test]
fn monogram_of_one_word_is_its_first_two_letters() {
    assert_eq!(monogram("stardew"), "ST");
    assert_eq!(monogram("x"), "X");
    assert_eq!(monogram("  "), "");
}
