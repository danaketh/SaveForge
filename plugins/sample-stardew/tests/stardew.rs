//! Native tests of the `StardewLike` plugin against the sample save in
//! `docs/sample-saves`.

use sample_stardew::{PRIORITY, Stardew, schema};
use saveforge_sdk::form::TABS_ID;
use saveforge_sdk::ui::{Event, Widget};
use saveforge_sdk::{Document, Plugin};
use serde_json::{Value, json};

fn sample_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/sample-saves/StardewLike-Farm.json"
    ))
    .unwrap()
}

fn labels(doc: &impl Document) -> Vec<String> {
    doc.view()
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Field(f) => Some(f.label.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn detects_player_saves_only() {
    let header = sample_bytes();
    assert_eq!(
        Stardew::detect("Farm.json", &header[..header.len().min(512)]),
        Some(PRIORITY)
    );
    assert_eq!(Stardew::detect("other.json", br#"{"terraTokens":1}"#), None);
}

#[test]
fn schema_has_player_and_inventory_pages() {
    let titles: Vec<_> = schema().pages.iter().map(|p| p.title.clone()).collect();
    assert_eq!(titles, ["Player", "Inventory"]);
}

#[test]
fn opens_sample_save_and_shows_player_fields() {
    let doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    assert_eq!(
        labels(&doc),
        [
            "Name",
            "Level",
            "Money",
            "Farm type",
            "Married",
            "Health",
            "Energy"
        ]
    );
}

#[test]
fn spouse_appears_when_married() {
    let mut doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    doc.update(Event::Toggled(("player.married".to_owned(), true)));
    assert!(labels(&doc).contains(&"Spouse".to_owned()));
    // Married without a spouse name is invalid and blocks saving.
    assert!(doc.serialize().is_err());
    doc.update(Event::TextChanged((
        "player.spouse".to_owned(),
        "Abigail".to_owned(),
    )));
    assert!(doc.serialize().is_ok());
}

#[test]
fn inventory_page_lists_items() {
    let mut doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    doc.update(Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    let ids: Vec<_> = tree
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Field(f) => Some(f.id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        ids,
        [
            "inventory.0.id",
            "inventory.0.count",
            "inventory.1.id",
            "inventory.1.count"
        ]
    );
}

#[test]
fn edits_round_trip_and_keep_unknown_data() {
    let mut doc = Stardew::open("Farm.json", sample_bytes(), vec![]).unwrap();
    doc.update(Event::NumberChanged(("player.money".to_owned(), 99_999.0)));
    let saved: Value = serde_json::from_slice(&doc.serialize().unwrap()).unwrap();
    assert_eq!(saved["player"]["money"], json!(99_999));
    assert_eq!(saved["farm"]["name"], json!("Sunny Acres"));
    assert_eq!(saved["playtime_minutes"], json!(1834));
    let original: Value = serde_json::from_slice(&sample_bytes()).unwrap();
    let mut expected = original;
    expected["player"]["money"] = json!(99_999);
    assert_eq!(saved, expected, "nothing else changed");
}
