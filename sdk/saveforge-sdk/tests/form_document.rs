use saveforge_sdk::Document;
use saveforge_sdk::form::{
    ADD_SUFFIX, FormDocument, FormPage, FormSchema, FormSection, JsonCodec, TABS_ID, VisibilityOp,
    VisibilityRule, checkbox, choice, list, number, object, text,
};
use saveforge_sdk::ui::{Event, FieldSpec, Input, UiTree, Widget};
use serde_json::{Value, json};

type Doc = FormDocument<JsonCodec>;

fn schema() -> FormSchema {
    FormSchema::new()
        .page(
            FormPage::new("Player").section(
                FormSection::new()
                    .heading("Identity")
                    .field(text("player.name", "Name").required().max_len(10))
                    .field(text("player.code", "Code").pattern("[A-Z]{3}"))
                    .field(number("player.level", "Level").range(1.0, 100.0))
                    .field(number("player.speed", "Speed").decimals(2))
                    .field(checkbox("player.married", "Married"))
                    .field(text("player.spouse", "Spouse").required().visible_when(
                        VisibilityRule::new("player.married", VisibilityOp::Equals, true),
                    ))
                    .field(choice(
                        "player.farm",
                        "Farm",
                        &[("standard", "Standard"), ("forest", "Forest")],
                    ))
                    .field(text("player.id", "Id").read_only())
                    .field(object(
                        "player.stats",
                        "Stats",
                        vec![number("health", "Health").range(0.0, 100.0)],
                    )),
            ),
        )
        .page(
            FormPage::new("Inventory").section(FormSection::new().field(list(
                "inventory",
                "Items",
                "item",
                vec![
                    text("id", "Item ID").required(),
                    number("count", "Count").min(1.0),
                ],
            ))),
        )
}

fn sample() -> Value {
    json!({
        "player": {
            "name": "Robin",
            "code": "ABC",
            "level": 12,
            "speed": 1.5,
            "married": false,
            "farm": "forest",
            "id": "p-1",
            "stats": { "health": 80 }
        },
        "inventory": [ { "id": "seeds", "count": 15 } ],
        "unknown": { "kept": true }
    })
}

fn open(value: &Value) -> Doc {
    Doc::open(schema(), value.to_string().as_bytes()).unwrap()
}

fn field<'a>(tree: &'a UiTree, id: &str) -> Option<&'a FieldSpec> {
    tree.nodes.iter().find_map(|n| match &n.widget {
        Widget::Field(f) if f.id == id => Some(f),
        _ => None,
    })
}

fn has_button(tree: &UiTree, id: &str) -> bool {
    tree.nodes
        .iter()
        .any(|n| matches!(&n.widget, Widget::Button(b) if b.id == id))
}

fn edit(doc: &mut Doc, event: Event) {
    doc.update(event);
}

fn text_event(id: &str, value: &str) -> Event {
    Event::TextChanged((id.to_owned(), value.to_owned()))
}

#[test]
fn invalid_json_is_rejected() {
    let err = Doc::open(schema(), b"{ nope").unwrap_err();
    assert!(err.contains("not valid JSON"), "{err}");
}

#[test]
fn multi_page_schema_renders_tabs_with_the_selected_page() {
    let mut doc = open(&sample());
    let tree = doc.view();
    assert!(matches!(&tree.nodes[0].widget, Widget::Tabs(t) if t.id == TABS_ID && t.selected == 0));
    assert!(field(&tree, "player.name").is_some());
    assert!(field(&tree, "inventory.0.id").is_none());

    edit(&mut doc, Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    assert!(field(&tree, "inventory.0.id").is_some());
    assert!(field(&tree, "player.name").is_none());
    assert!(!doc.is_dirty(), "switching tabs is not an edit");
}

#[test]
fn fields_show_current_values() {
    let tree = open(&sample()).view();
    let name = field(&tree, "player.name").unwrap();
    assert!(matches!(&name.input, Input::Text(t) if t.value == "Robin" && t.max_len == Some(10)));
    let level = field(&tree, "player.level").unwrap();
    assert!(
        matches!(&level.input, Input::Number(n) if (n.value - 12.0).abs() < f64::EPSILON && n.min == Some(1.0))
    );
    let married = field(&tree, "player.married").unwrap();
    assert_eq!(married.input, Input::Checkbox(false));
    let farm = field(&tree, "player.farm").unwrap();
    assert!(matches!(&farm.input, Input::Pick(p) if p.selected.as_deref() == Some("forest")));
    assert!(field(&tree, "player.id").unwrap().read_only);
    assert!(field(&tree, "player.stats.health").is_some());
}

#[test]
fn edits_update_value_and_dirty() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", "Abigail"));
    edit(
        &mut doc,
        Event::NumberChanged(("player.level".to_owned(), 30.0)),
    );
    edit(
        &mut doc,
        Event::NumberChanged(("player.speed".to_owned(), 2.25)),
    );
    edit(
        &mut doc,
        Event::Toggled(("player.married".to_owned(), true)),
    );
    edit(
        &mut doc,
        Event::Picked(("player.farm".to_owned(), "standard".to_owned())),
    );
    edit(
        &mut doc,
        Event::NumberChanged(("player.stats.health".to_owned(), 90.0)),
    );
    let v = doc.value();
    assert_eq!(v["player"]["name"], json!("Abigail"));
    assert_eq!(
        v["player"]["level"],
        json!(30),
        "whole numbers stay integers"
    );
    assert_eq!(v["player"]["speed"], json!(2.25));
    assert_eq!(v["player"]["married"], json!(true));
    assert_eq!(v["player"]["farm"], json!("standard"));
    assert_eq!(v["player"]["stats"]["health"], json!(90));
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());
}

#[test]
fn read_only_and_unknown_fields_ignore_edits() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.id", "hacked"));
    edit(&mut doc, text_event("not.in.schema", "x"));
    edit(&mut doc, text_event("unknown.kept", "x"));
    assert_eq!(doc.value(), &sample());
    assert!(!doc.is_dirty());
}

#[test]
fn same_value_is_not_an_edit() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", "Robin"));
    assert!(!doc.is_dirty());
}

#[test]
fn list_items_add_and_remove() {
    let mut doc = open(&sample());
    edit(&mut doc, Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    assert!(has_button(&tree, &format!("inventory{ADD_SUFFIX}")));

    edit(&mut doc, Event::Pressed(format!("inventory{ADD_SUFFIX}")));
    assert_eq!(
        doc.value()["inventory"],
        json!([ { "id": "seeds", "count": 15 }, { "id": "", "count": 1 } ])
    );
    edit(&mut doc, text_event("inventory.1.id", "ore"));
    assert_eq!(doc.value()["inventory"][1]["id"], json!("ore"));

    edit(&mut doc, Event::ListRemove(("inventory".to_owned(), 0)));
    assert_eq!(
        doc.value()["inventory"],
        json!([ { "id": "ore", "count": 1 } ])
    );
    edit(&mut doc, Event::ListRemove(("inventory".to_owned(), 7)));
    assert_eq!(doc.value()["inventory"].as_array().unwrap().len(), 1);
    assert!(doc.is_dirty());
}

#[test]
fn adding_to_a_missing_list_creates_it() {
    let mut value = sample();
    value.as_object_mut().unwrap().remove("inventory");
    let mut doc = open(&value);
    edit(&mut doc, Event::Pressed(format!("inventory{ADD_SUFFIX}")));
    assert_eq!(
        doc.value()["inventory"],
        json!([ { "id": "", "count": 1 } ])
    );
}

#[test]
fn hidden_fields_are_not_rendered_or_validated() {
    let mut doc = open(&sample());
    assert!(field(&doc.view(), "player.spouse").is_none());
    assert_eq!(doc.errors(), Vec::<String>::new());

    edit(
        &mut doc,
        Event::Toggled(("player.married".to_owned(), true)),
    );
    let tree = doc.view();
    let spouse = field(&tree, "player.spouse").expect("shown once married");
    assert_eq!(spouse.error.as_deref(), Some("Spouse is required."));
    assert_eq!(doc.errors(), vec!["Spouse is required.".to_owned()]);
}

#[test]
fn validation_messages() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", ""));
    edit(&mut doc, text_event("player.code", "abcd"));
    edit(
        &mut doc,
        Event::NumberChanged(("player.level".to_owned(), 500.0)),
    );
    assert_eq!(
        doc.errors(),
        vec![
            "Name is required.".to_owned(),
            "Code has an invalid format.".to_owned(),
            "Level must be between 1 and 100.".to_owned(),
        ]
    );
    edit(&mut doc, text_event("player.name", "Abcdefghijkl"));
    assert_eq!(doc.errors()[0], "Name must not exceed 10 characters.");

    let tree = doc.view();
    assert_eq!(
        field(&tree, "player.code").unwrap().error.as_deref(),
        Some("Code has an invalid format.")
    );
}

#[test]
fn invalid_document_cannot_be_serialized() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", ""));
    let err = doc.serialize().unwrap_err();
    assert!(
        err.starts_with("Fix 1 invalid field(s) before saving"),
        "{err}"
    );
    assert!(err.contains("Name is required."), "{err}");
}

#[test]
fn serialize_keeps_unknown_keys_and_key_order() {
    let original = r#"{"zeta":1,"player":{"name":"Robin","code":"ABC","level":12,"speed":1.5,"married":false,"farm":"forest","id":"p-1","stats":{"health":80}},"inventory":[],"alpha":{"kept":true}}"#;
    let mut doc = Doc::open(schema(), original.as_bytes()).unwrap();
    edit(&mut doc, text_event("player.name", "Abigail"));
    let bytes = doc.serialize().unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let reparsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(reparsed["player"]["name"], json!("Abigail"));
    assert_eq!(reparsed["alpha"], json!({ "kept": true }));
    let zeta = text.find("\"zeta\"").unwrap();
    let alpha = text.find("\"alpha\"").unwrap();
    assert!(zeta < alpha, "original key order is kept:\n{text}");
    assert!(text.contains('\n'), "written pretty-printed");
}

#[test]
fn numbers_stored_as_strings_show_their_value() {
    let mut value = sample();
    value["player"]["level"] = json!("42");
    let tree = open(&value).view();
    let level = field(&tree, "player.level").unwrap();
    assert!(matches!(&level.input, Input::Number(n) if (n.value - 42.0).abs() < f64::EPSILON));
}

#[test]
fn non_object_list_items_render_and_ignore_edits() {
    let mut value = sample();
    value["inventory"] = json!(["seeds", 7]);
    let mut doc = open(&value);
    edit(&mut doc, Event::TabSelected((TABS_ID.to_owned(), 1)));
    let tree = doc.view();
    assert!(field(&tree, "inventory.0.id").is_some());
    edit(&mut doc, text_event("inventory.0.id", "x"));
    assert_eq!(doc.value()["inventory"], json!(["seeds", 7]));
    assert!(!doc.is_dirty());
    // Rows the form cannot edit must not block saving the rest.
    assert!(doc.serialize().is_ok());
}

#[test]
fn non_object_containers_do_not_block_saving() {
    let schema = FormSchema::new().page(FormPage::new("P").section(FormSection::new().field(
        object("stats", "Stats", vec![text("name", "Name").required()]),
    )));
    let doc = Doc::open(schema, br#"{"stats":5}"#).unwrap();
    assert_eq!(doc.errors(), Vec::<String>::new());
}

#[test]
fn bom_prefixed_save_opens() {
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend(sample().to_string().as_bytes());
    let doc = Doc::open(schema(), &bytes).unwrap();
    assert_eq!(doc.value(), &sample());
}

#[test]
fn numeric_string_edits_keep_the_string_type() {
    let mut value = sample();
    value["player"]["level"] = json!("42");
    let mut doc = open(&value);
    edit(
        &mut doc,
        Event::NumberChanged(("player.level".to_owned(), 42.0)),
    );
    assert!(!doc.is_dirty(), "same number is no edit");
    edit(
        &mut doc,
        Event::NumberChanged(("player.level".to_owned(), 50.0)),
    );
    assert_eq!(doc.value()["player"]["level"], json!("50"));
}

#[test]
fn untouched_floats_round_trip_exactly() {
    let raw = r#"{"player":{"name":"Robin","level":12},"seed":0.09458110960934567}"#;
    let mut doc = Doc::open(schema(), raw.as_bytes()).unwrap();
    edit(&mut doc, text_event("player.name", "Robyn"));
    let saved = String::from_utf8(doc.serialize().unwrap()).unwrap();
    assert!(saved.contains("0.09458110960934567"), "{saved}");
}

#[test]
fn max_len_counts_characters_not_bytes() {
    let mut doc = open(&sample());
    edit(&mut doc, text_event("player.name", "Žluťoučký"));
    assert_eq!(doc.errors(), Vec::<String>::new());
}

#[test]
fn malformed_pattern_in_schema_is_ignored() {
    let schema = FormSchema::new().page(
        FormPage::new("P").section(FormSection::new().field(text("code", "Code").pattern("["))),
    );
    let doc = Doc::open(schema, br#"{"code":"anything"}"#).unwrap();
    assert_eq!(doc.errors(), Vec::<String>::new());
}

#[test]
fn root_must_be_a_json_object() {
    let err = Doc::open(schema(), b"[1, 2]").unwrap_err();
    assert_eq!(err, "the save is not a JSON object");
}
