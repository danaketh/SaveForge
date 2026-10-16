mod common;

use planet_crafter::save::{Item, Save, int};
use serde_json::json;

fn minimal() -> Save {
    Save::parse(common::minimal().as_bytes()).unwrap()
}

fn item(id: i64, gid: &str) -> Item {
    Item {
        world_object_id: id,
        gid: gid.to_owned(),
    }
}

fn wo_ids(save: &Save, inventory: usize) -> String {
    let text = String::from_utf8(save.to_bytes()).unwrap();
    let parsed = planet_crafter::format::SaveFile::parse(&text).unwrap();
    parsed.group(4).objects()[inventory]
        .get("woIds")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_owned()
}

#[test]
fn non_utf8_is_rejected() {
    assert_eq!(
        Save::parse(b"\xff\xfe{}").unwrap_err(),
        "the save is not valid UTF-8"
    );
}

#[test]
fn inventory_items_follow_wo_ids_and_skip_unknown_entries() {
    // Inventory 5 holds "10,11,99,x": 99 has no world object, x is no id.
    assert_eq!(
        minimal().inventory_items(5),
        Some(vec![item(10, "Cobalt"), item(11, "Iron")])
    );
    assert_eq!(minimal().inventory_items(6), Some(vec![]));
}

#[test]
fn missing_inventory_is_none() {
    assert_eq!(minimal().inventory_items(8), None);
}

#[test]
fn wo_ids_split_on_commas_and_pipes() {
    let mut groups = common::GROUPS;
    groups[4] = &[r#"{"id":5,"woIds":" 10 | 12 ,-3","size":4}"#];
    let save = Save::parse(common::build(&groups).as_bytes()).unwrap();
    assert_eq!(
        save.inventory_items(5),
        Some(vec![item(10, "Cobalt"), item(12, "Titanium")])
    );
}

#[test]
fn add_item_creates_a_world_object_with_the_next_id() {
    let mut save = minimal();
    assert_eq!(save.add_item(6, "Iron"), Some(13));
    assert_eq!(save.inventory_items(6), Some(vec![item(13, "Iron")]));
    assert_eq!(wo_ids(&save, 1), "13");
    let text = String::from_utf8(save.to_bytes()).unwrap();
    assert!(text.contains("{\"id\":12,\"gId\":\"Titanium\"}|\n{\"id\":13,\"gId\":\"Iron\"}\r@"));
}

#[test]
fn add_item_rewrites_wo_ids_comma_joined() {
    let mut save = minimal();
    assert_eq!(save.add_item(5, "Titanium"), Some(13));
    assert_eq!(wo_ids(&save, 0), "10,11,99,13");
}

#[test]
fn add_item_to_a_missing_inventory_changes_nothing() {
    let mut save = minimal();
    assert_eq!(save.add_item(8, "Iron"), None);
    assert_eq!(save.to_bytes(), common::minimal().into_bytes());
}

#[test]
fn add_item_to_a_save_without_world_objects_starts_at_one() {
    let mut groups = common::GROUPS;
    groups[3] = &[];
    let mut save = Save::parse(common::build(&groups).as_bytes()).unwrap();
    assert_eq!(save.add_item(6, "Iron"), Some(1));
}

#[test]
fn remove_item_drops_the_id_but_keeps_the_world_object() {
    let mut save = minimal();
    assert!(save.remove_item(5, 11));
    assert_eq!(save.inventory_items(5), Some(vec![item(10, "Cobalt")]));
    assert_eq!(wo_ids(&save, 0), "10,99");
    assert!(save.gids().contains(&"Iron".to_owned()));
}

#[test]
fn remove_item_that_is_not_there_changes_nothing() {
    let mut save = minimal();
    assert!(!save.remove_item(5, 12));
    assert!(!save.remove_item(8, 10));
    assert_eq!(save.to_bytes(), common::minimal().into_bytes());
}

#[test]
fn gids_are_distinct_and_sorted() {
    assert_eq!(minimal().gids(), ["Cobalt", "Iron", "Titanium"]);
}

#[test]
fn int_accepts_whole_floats() {
    assert_eq!(int(Some(&json!(12))), Some(12));
    assert_eq!(int(Some(&json!(12.0))), Some(12));
    assert_eq!(int(Some(&json!(12.5))), None);
    assert_eq!(int(Some(&json!("12"))), None);
}

#[test]
fn sample_save_links_player_inventories() {
    let save = Save::parse(&common::sample()).unwrap();
    let host = &save.players()[0];
    let inventory = int(host.get("inventoryId")).unwrap();
    let items = save.inventory_items(inventory).unwrap();
    assert_ne!(items.len(), 0);
    assert!(items.iter().all(|i| !i.gid.is_empty()));
}
