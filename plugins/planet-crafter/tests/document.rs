mod common;

use planet_crafter::document::{
    ALL_TIME_TOKENS_ID, CHARACTER, CRAFTED_OBJECTS_ID, DISPLAY_NAME_ID, EQUIPMENT, INVENTORY,
    PLANET_ID_ID, PlanetCrafterDocument, TABS, TERRA_TOKENS_ID, UNLOCKED_GROUPS_ID,
};
use planet_crafter::format::SaveFile;
use planet_crafter::localization::Names;
use planet_crafter::save::Save;
use saveforge_sdk::Document;
use saveforge_sdk::ui::{Event, FieldSpec, Input, UiTree, Widget};
use serde_json::{Value, json};

const NAMES: &str = "GROUP_NAME_Iron**Iron Ore*Fer\nGROUP_NAME_Cobalt**Cobalt*Cobalt\n";

fn open(text: &str) -> PlanetCrafterDocument {
    PlanetCrafterDocument::new(
        Save::parse(text.as_bytes()).unwrap(),
        Names::parse(NAMES.as_bytes()),
    )
}

fn doc() -> PlanetCrafterDocument {
    open(&common::minimal())
}

fn tab(doc: &mut PlanetCrafterDocument, index: u32) -> UiTree {
    doc.update(Event::TabSelected((TABS.to_owned(), index)));
    doc.view()
}

fn field<'a>(tree: &'a UiTree, id: &str) -> &'a FieldSpec {
    tree.nodes
        .iter()
        .find_map(|n| match &n.widget {
            Widget::Field(f) if f.id == id => Some(f),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no field {id}"))
}

fn number(tree: &UiTree, id: &str) -> f64 {
    match &field(tree, id).input {
        Input::Number(n) => n.value,
        other => panic!("{id} is not a number: {other:?}"),
    }
}

fn texts(tree: &UiTree) -> Vec<String> {
    tree.nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

/// Display texts of a list's rows.
fn rows(tree: &UiTree, list_id: &str) -> Vec<String> {
    let list = tree
        .nodes
        .iter()
        .find(|n| matches!(&n.widget, Widget::List(l) if l.id == list_id))
        .unwrap_or_else(|| panic!("no list {list_id}"));
    list.children
        .iter()
        .map(|&i| match &tree.nodes[i as usize].widget {
            Widget::Text(t) => t.text.clone(),
            other => panic!("row is not text: {other:?}"),
        })
        .collect()
}

/// The saved file, parsed again.
fn saved(doc: &PlanetCrafterDocument) -> SaveFile {
    SaveFile::parse(&String::from_utf8(doc.serialize().unwrap()).unwrap()).unwrap()
}

fn num(id: &str, value: f64) -> Event {
    Event::NumberChanged((id.to_owned(), value))
}

#[test]
fn view_shows_four_tabs_and_the_host_first() {
    let tree = doc().view();
    let Widget::Tabs(tabs) = &tree.nodes[0].widget else {
        panic!("root is not tabs");
    };
    assert_eq!(
        tabs.titles,
        ["Characters", "Terra Tokens", "Planet State", "Save Info"]
    );
    let Input::Pick(pick) = &field(&tree, CHARACTER).input else {
        panic!("character is not a pick-list");
    };
    let names: Vec<_> = pick.options.iter().map(|o| o.display.as_str()).collect();
    assert_eq!(names, ["Alice (Host)", "Bob"]);
    assert_eq!(pick.selected.as_deref(), Some("0"));
    assert!((number(&tree, "player.playerGaugeHealth") - 90.0).abs() < f64::EPSILON);
}

#[test]
fn gauges_edit_the_selected_player() {
    let mut doc = doc();
    doc.update(num("player.playerGaugeHealth", 55.5));
    assert!(doc.is_dirty());
    doc.update(Event::Picked((CHARACTER.to_owned(), "1".to_owned())));
    doc.update(num("player.playerGaugeToxic", 12.5));
    let save = saved(&doc);
    let players = save.group(2).objects();
    assert_eq!(players[0].get("playerGaugeHealth"), Some(&json!(55.5)));
    assert_eq!(players[1].get("playerGaugeToxic"), Some(&json!(12.5)));
    assert_eq!(players[1].get("playerGaugeHealth"), Some(&json!(70.0)));
}

#[test]
fn out_of_range_and_unchanged_numbers_are_not_edits() {
    let mut doc = doc();
    doc.update(num("player.playerGaugeHealth", 150.0));
    doc.update(num("player.playerGaugeOxygen", -1.0));
    doc.update(num("player.playerGaugeHealth", f64::NAN));
    doc.update(num("player.playerGaugeHealth", 90.0));
    doc.update(num("player.unknownGauge", 1.0));
    doc.update(num("planet.0.unitPurificationLevel", 1.0));
    doc.update(num("planet.2.unitHeatLevel", 1.0));
    doc.update(num("planet.x.unitHeatLevel", 1.0));
    doc.update(num("planet.unitHeatLevel", 1.0));
    doc.update(num(ALL_TIME_TOKENS_ID, 1.0));
    assert!(!doc.is_dirty());
    assert_eq!(doc.serialize().unwrap(), common::minimal().into_bytes());
}

#[test]
fn oxygen_allows_up_to_370() {
    let mut doc = doc();
    doc.update(num("player.playerGaugeOxygen", 370.0));
    assert_eq!(
        saved(&doc).group(2).objects()[0].get("playerGaugeOxygen"),
        Some(&json!(370.0))
    );
}

#[test]
fn terra_tokens_are_whole_numbers() {
    let mut doc = doc();
    let tree = tab(&mut doc, 1);
    assert!((number(&tree, TERRA_TOKENS_ID) - 100.0).abs() < f64::EPSILON);
    assert!(field(&tree, ALL_TIME_TOKENS_ID).read_only);
    assert!(field(&tree, UNLOCKED_GROUPS_ID).read_only);
    doc.update(num(TERRA_TOKENS_ID, 15000.0));
    let text = String::from_utf8(doc.serialize().unwrap()).unwrap();
    assert!(
        text.contains(r#"{"terraTokens":15000,"allTimeTerraTokens":200,"#),
        "{text}"
    );
}

#[test]
fn planet_levels_edit_each_planet() {
    let mut doc = doc();
    let tree = tab(&mut doc, 2);
    assert!((number(&tree, "planet.0.unitPlantsLevel") - 44_554_280_960.0).abs() < 1.0);
    assert!((number(&tree, "planet.1.unitOxygenLevel") - 166_326.765_625).abs() < 1e-6);
    let titles: Vec<_> = tree
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Section(title) => Some(title.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(titles, ["Prime", "Humble"]);

    doc.update(num("planet.1.unitHeatLevel", 1234.56));
    let save = saved(&doc);
    assert_eq!(
        save.group(1).objects()[1].get("unitHeatLevel"),
        Some(&json!(1234.56))
    );
    assert_eq!(
        save.group(1).objects()[0].get("unitHeatLevel"),
        Some(&json!(2.5))
    );
}

#[test]
fn display_name_is_editable_and_capped_at_64_characters() {
    let mut doc = doc();
    let tree = tab(&mut doc, 3);
    assert!(field(&tree, PLANET_ID_ID).read_only);
    assert!(field(&tree, CRAFTED_OBJECTS_ID).read_only);
    let Input::Text(name) = &field(&tree, CRAFTED_OBJECTS_ID).input else {
        panic!("crafted objects is not text");
    };
    assert_eq!(name.value, "50");

    doc.update(Event::TextChanged((
        DISPLAY_NAME_ID.to_owned(),
        "é".repeat(70),
    )));
    assert_eq!(
        saved(&doc).single(8).get("saveDisplayName"),
        Some(&Value::String("é".repeat(64)))
    );
}

#[test]
fn read_only_text_fields_ignore_edits() {
    let mut doc = doc();
    for id in [PLANET_ID_ID, CRAFTED_OBJECTS_ID, UNLOCKED_GROUPS_ID] {
        doc.update(Event::TextChanged((id.to_owned(), "x".to_owned())));
    }
    assert!(!doc.is_dirty());
}

#[test]
fn inventories_show_localized_names_and_offer_known_items() {
    let tree = doc().view();
    assert_eq!(rows(&tree, INVENTORY), ["Cobalt", "Iron Ore"]);
    assert_eq!(rows(&tree, EQUIPMENT), Vec::<String>::new());
    let list = tree
        .nodes
        .iter()
        .find_map(|n| match &n.widget {
            Widget::List(l) if l.id == INVENTORY => Some(l),
            _ => None,
        })
        .unwrap();
    assert!(list.removable);
    let offered: Vec<_> = list
        .add_options
        .as_ref()
        .unwrap()
        .options
        .iter()
        .map(|o| (o.value.as_str(), o.display.as_str()))
        .collect();
    assert_eq!(offered, [("Cobalt", "Cobalt"), ("Iron", "Iron Ore")]);
}

#[test]
fn adding_and_removing_items() {
    let mut doc = doc();
    doc.update(Event::ListAdd((EQUIPMENT.to_owned(), "Iron".to_owned())));
    assert!(doc.is_dirty());
    assert_eq!(rows(&doc.view(), EQUIPMENT), ["Iron Ore"]);

    // Row 1 is Iron (id 11); the unknown id 99 between does not shift rows.
    doc.update(Event::ListRemove((INVENTORY.to_owned(), 1)));
    assert_eq!(rows(&doc.view(), INVENTORY), ["Cobalt"]);
    let save = saved(&doc);
    assert_eq!(
        save.group(4).objects()[0].get("woIds"),
        Some(&json!("10,99"))
    );
    assert_eq!(save.group(4).objects()[1].get("woIds"), Some(&json!("13")));
}

#[test]
fn removing_past_the_end_or_adding_nothing_is_ignored() {
    let mut doc = doc();
    doc.update(Event::ListRemove((INVENTORY.to_owned(), 2)));
    doc.update(Event::ListAdd((INVENTORY.to_owned(), String::new())));
    doc.update(Event::ListAdd(("other".to_owned(), "Iron".to_owned())));
    assert!(!doc.is_dirty());
}

#[test]
fn a_missing_inventory_shows_a_note_and_ignores_edits() {
    let mut doc = doc();
    doc.update(Event::Picked((CHARACTER.to_owned(), "1".to_owned())));
    let tree = doc.view();
    assert!(texts(&tree).contains(&"This inventory is not in the save.".to_owned()));
    assert_eq!(rows(&tree, INVENTORY), ["Titanium"]);
    doc.update(Event::ListAdd((EQUIPMENT.to_owned(), "Iron".to_owned())));
    doc.update(Event::ListRemove((EQUIPMENT.to_owned(), 0)));
    assert!(!doc.is_dirty());
}

#[test]
fn picking_an_unknown_character_keeps_the_selection() {
    let mut doc = doc();
    doc.update(Event::Picked((CHARACTER.to_owned(), "5".to_owned())));
    doc.update(Event::Picked((CHARACTER.to_owned(), "x".to_owned())));
    let tree = doc.view();
    let Input::Pick(pick) = &field(&tree, CHARACTER).input else {
        panic!("character is not a pick-list");
    };
    assert_eq!(pick.selected.as_deref(), Some("0"));
}

#[test]
fn a_save_without_characters_says_so() {
    let mut groups = common::GROUPS;
    groups[2] = &[];
    let mut doc = open(&common::build(&groups));
    assert!(texts(&doc.view()).contains(&"This save has no characters.".to_owned()));
    doc.update(num("player.playerGaugeHealth", 50.0));
    assert!(!doc.is_dirty());
}

#[test]
fn a_save_without_planets_says_so() {
    let mut groups = common::GROUPS;
    groups[1] = &[];
    let mut doc = open(&common::build(&groups));
    let tree = tab(&mut doc, 2);
    assert!(texts(&tree).contains(&"This save has no planets.".to_owned()));
}

#[test]
fn saving_clears_dirty_and_unmodified_saves_are_identical() {
    let mut doc = doc();
    assert_eq!(doc.serialize().unwrap(), common::minimal().into_bytes());
    doc.update(num(TERRA_TOKENS_ID, 1.0));
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());
}

#[test]
fn sample_save_view_fits_the_host_limits() {
    let doc = PlanetCrafterDocument::new(Save::parse(&common::sample()).unwrap(), Names::default());
    let tree = doc.view();
    assert!(tree.nodes.len() < 2_000, "{} nodes", tree.nodes.len());
    assert!(texts(&tree).iter().all(|t| !t.is_empty()));
}

#[test]
fn display_name_drops_group_and_object_separators() {
    // The game's own loader may split on `@` and `|` without regard to
    // JSON strings, so the editor never writes them.
    let mut doc = doc();
    doc.update(Event::TextChanged((
        DISPLAY_NAME_ID.to_owned(),
        "Base @ Lake | 2".to_owned(),
    )));
    assert_eq!(
        saved(&doc).single(8).get("saveDisplayName"),
        Some(&json!("Base  Lake  2"))
    );
}

#[test]
fn terra_tokens_stay_within_the_game_int_range() {
    let mut doc = doc();
    let tree = tab(&mut doc, 1);
    let Input::Number(tokens) = &field(&tree, TERRA_TOKENS_ID).input else {
        panic!("terra tokens is not a number");
    };
    assert_eq!(tokens.max, Some(f64::from(i32::MAX)));
    doc.update(num(TERRA_TOKENS_ID, 3_000_000_000.0));
    assert!(!doc.is_dirty());
    doc.update(num(TERRA_TOKENS_ID, f64::from(i32::MAX)));
    assert!(doc.is_dirty());
}
