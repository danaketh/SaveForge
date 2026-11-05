//! Editing ability scores and experience.

use bg3::document::{ABILITY_PREFIX, CHARACTER, EXPERIENCE};
use bg3::globals::{GLOBALS, Globals};
use bg3::{Bg3, Bg3Document};
use lslib::newage::NewAge;
use lslib::package::Package;
use saveforge_sdk::ui::{Event, Widget};
use saveforge_sdk::{Document as _, Plugin as _};

use crate::common::{editable_save, globals_lsf, newage, party, save};

fn open(bytes: Vec<u8>) -> Bg3Document {
    Bg3::open("save.lsv", bytes, vec![]).unwrap()
}

fn number(id: &str, value: f64) -> Event {
    Event::NumberChanged((id.to_owned(), value))
}

fn ability(index: usize) -> String {
    format!("{ABILITY_PREFIX}{index}")
}

/// The blob in a serialized save, and the points spent at character
/// creation (STR..CHA) per character.
fn points(bytes: &[u8]) -> (NewAge, Vec<[i32; 6]>) {
    let package = Package::read(bytes).unwrap();
    let blob = Globals::read(&package).unwrap().blob().clone();
    let creation = blob
        .component("game.character_creation.v1.CharacterCreationStatsComponent")
        .unwrap();
    let points = (0..creation.count)
        .map(|i| {
            let element = blob.element(creation, i).unwrap();
            let start = u64::from_le_bytes(element[72..80].try_into().unwrap());
            let raw = blob.bytes_at(start + 4, 24).unwrap();
            std::array::from_fn(|a| i32::from_le_bytes(raw[a * 4..a * 4 + 4].try_into().unwrap()))
        })
        .collect();
    (blob, points)
}

#[test]
fn lists_characters_with_scores_and_experience() {
    let doc = open(editable_save());
    let characters = doc.characters().unwrap();
    let summary: Vec<(&str, [i32; 6], Option<i32>)> = characters
        .iter()
        .map(|c| (c.name.as_str(), c.abilities, c.experience))
        .collect();
    assert_eq!(
        summary,
        [
            ("Gale", [8, 12, 14, 17, 14, 10], Some(673)),
            ("Lae'zel", [17, 14, 14, 8, 10, 12], None),
        ]
    );
}

#[test]
fn view_shows_the_selected_character() {
    let mut doc = open(editable_save());
    let fields = |doc: &Bg3Document| -> Vec<(String, String)> {
        doc.view()
            .nodes
            .into_iter()
            .filter_map(|node| match node.widget {
                Widget::Field(field) => Some((field.id, field.label)),
                _ => None,
            })
            .collect()
    };
    let ids: Vec<String> = fields(&doc).into_iter().map(|(id, _)| id).collect();
    assert_eq!(
        ids,
        [
            CHARACTER,
            "ability.0",
            "ability.1",
            "ability.2",
            "ability.3",
            "ability.4",
            "ability.5",
            EXPERIENCE
        ]
    );

    doc.update(Event::Picked((CHARACTER.to_owned(), "1".to_owned())));
    let ids: Vec<String> = fields(&doc).into_iter().map(|(id, _)| id).collect();
    assert_eq!(ids.len(), 7, "Lae'zel has no experience field: {ids:?}");
    assert!(doc.view().nodes.iter().any(|n| matches!(
        &n.widget,
        Widget::Text(span) if span.text == "Only party members have experience."
    )));

    // Out-of-range picks are ignored.
    doc.update(Event::Picked((CHARACTER.to_owned(), "9".to_owned())));
    assert_eq!(fields(&doc).len(), 7);
}

#[test]
fn ability_edits_move_creation_points_and_scores_together() {
    let mut doc = open(editable_save());
    doc.update(number(&ability(0), 18.0));
    doc.update(number(&ability(4), 5.0));
    assert!(doc.is_dirty());
    assert_eq!(
        doc.characters().unwrap()[0].abilities,
        [18, 12, 14, 17, 5, 10]
    );

    let bytes = doc.serialize().unwrap();
    let (blob, points) = points(&bytes);
    assert_eq!(blob.stored_checksum(), blob.checksum());
    // Gale: STR 0 + 10, WIS 6 - 9 (bonuses are not part of the points).
    assert_eq!(points[0], [10, 4, 5, 7, -3, 2]);
    assert_eq!(points[1], party()[1].points);

    let reopened = open(bytes);
    assert_eq!(
        reopened.characters().unwrap()[0].abilities,
        [18, 12, 14, 17, 5, 10]
    );
}

#[test]
fn experience_edits_are_saved() {
    let mut doc = open(editable_save());
    doc.update(number(EXPERIENCE, 2_700.4));
    let reopened = open(doc.serialize().unwrap());
    assert_eq!(reopened.characters().unwrap()[0].experience, Some(2_700));
}

#[test]
fn out_of_range_edits_are_rejected_with_a_message() {
    let mut doc = open(editable_save());
    doc.update(number(&ability(0), 31.0));
    doc.update(number(EXPERIENCE, -1.0));
    doc.update(number(&ability(0), f64::NAN));
    assert!(!doc.is_dirty());
    assert_eq!(doc.characters().unwrap()[0].abilities[0], 8);
    assert!(doc.view().nodes.iter().any(|n| matches!(
        &n.widget,
        Widget::Text(span) if span.text.contains("experience ranges")
    )));
    // A valid edit clears the message.
    doc.update(number(&ability(0), 9.0));
    assert!(!doc.view().nodes.iter().any(|n| matches!(
        &n.widget,
        Widget::Text(span) if span.text.contains("ranges")
    )));
}

#[test]
fn saving_replaces_only_globals_and_renews_the_hash() {
    let original = editable_save();
    let mut doc = open(original.clone());
    assert_eq!(
        doc.serialize().unwrap(),
        original,
        "unchanged saves are kept"
    );

    doc.update(number(&ability(1), 13.0));
    let bytes = doc.serialize().unwrap();
    let before = Package::read(&original).unwrap();
    let after = Package::read(&bytes).unwrap();
    for file in before.files() {
        let new = after.find(&file.name).unwrap();
        if file.name == GLOBALS {
            assert_ne!(after.contents(new).unwrap(), before.contents(file).unwrap());
        } else {
            assert_eq!(after.contents(new).unwrap(), before.contents(file).unwrap());
        }
    }
    // The hash matches a fresh build of the same contents.
    let mut fresh = after.to_builder().unwrap();
    for file in &mut fresh.files {
        file.stored = None;
    }
    assert_eq!(
        after.md5(),
        Package::read(&fresh.build().unwrap()).unwrap().md5()
    );

    doc.mark_saved();
    assert!(!doc.is_dirty());
}

#[test]
fn saves_without_character_data_open_read_only() {
    let mut doc = open(save(true, None));
    assert!(doc.characters().unwrap_err().contains("Globals.lsf"));
    assert!(doc.view().nodes.iter().any(|n| matches!(
        &n.widget,
        Widget::Text(span) if span.text.starts_with("Characters cannot be edited")
    )));
    doc.update(number(&ability(0), 10.0));
    assert!(!doc.is_dirty());

    // A blob whose layout differs (here: no experience component) is refused.
    let mut blob = newage(&party());
    let name = b"game.experience.v0.ExperienceComponent";
    let at = blob.windows(name.len()).position(|w| w == name).unwrap();
    blob[at + 5] = b'X';
    let doc = open(save(true, Some(globals_lsf(blob))));
    assert!(
        doc.characters()
            .unwrap_err()
            .contains("unsupported game version")
    );
}
