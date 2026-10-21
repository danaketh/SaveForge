use planet_crafter::localization::Names;

const FILE: &str = "\u{feff}LANGUAGES**English*French\r\n\
    GROUP_NAME_Iron**Iron Ore*Minerai de fer\r\n\
    GROUP_NAME_Cobalt** Cobalt *Cobalt\r\n\
    GROUP_DESC_Iron**Used for crafting*Utilisé\r\n\
    GROUP_NAME_Empty***Vide\r\n\
    GROUP_NAME_aluminium**aluminium\r\n\
    not a key line\r\n";

#[test]
fn names_come_from_the_english_column() {
    let names = Names::parse(FILE.as_bytes());
    assert_eq!(names.name("Iron"), "Iron Ore");
    assert_eq!(names.name("Cobalt"), "Cobalt");
    assert_eq!(names.name("aluminium"), "aluminium");
}

#[test]
fn unknown_and_empty_names_fall_back_to_the_gid() {
    let names = Names::parse(FILE.as_bytes());
    assert_eq!(names.name("Empty"), "Empty");
    assert_eq!(names.name("Titanium"), "Titanium");
    assert_eq!(Names::default().name("Iron"), "Iron");
}

#[test]
fn known_items_are_sorted_by_name_ignoring_case() {
    let names = Names::parse(FILE.as_bytes());
    let items = names.known_items(vec!["Titanium".to_owned()]);
    let pairs: Vec<_> = items
        .iter()
        .map(|(gid, name)| (gid.as_str(), name.as_str()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("aluminium", "aluminium"),
            ("Cobalt", "Cobalt"),
            ("Iron", "Iron Ore")
        ]
    );
}

#[test]
fn without_a_file_the_save_gids_are_offered() {
    let items = Names::default().known_items(vec!["iron".to_owned(), "Cobalt".to_owned()]);
    assert_eq!(
        items,
        [
            ("Cobalt".to_owned(), "Cobalt".to_owned()),
            ("iron".to_owned(), "iron".to_owned())
        ]
    );
}

#[test]
fn invalid_utf8_does_not_fail() {
    let mut bytes = b"GROUP_NAME_Iron**Iron\xff*x\n".to_vec();
    bytes.extend_from_slice(b"GROUP_NAME_Gold**Gold\n");
    let names = Names::parse(&bytes);
    assert_eq!(names.name("Gold"), "Gold");
    assert_eq!(names.name("Iron"), "Iron\u{fffd}");
}
