mod common;

use planet_crafter::format::{Object, SaveFile};
use serde_json::{Map, Value, json};

fn parse(text: &str) -> SaveFile {
    SaveFile::parse(text).unwrap()
}

fn parse_err(text: &str) -> String {
    SaveFile::parse(text).unwrap_err()
}

#[test]
fn parses_groups_and_objects() {
    let save = parse(&common::minimal());
    assert_eq!(save.single(0).get("terraTokens"), Some(&json!(100)));
    assert_eq!(save.group(1).objects().len(), 2, "one object per planet");
    assert_eq!(save.group(2).objects().len(), 2);
    assert_eq!(save.group(3).objects().len(), 3);
    assert_eq!(save.group(3).objects()[0].get("pos"), Some(&json!("1,2,3")));
    assert_eq!(
        save.single(8).get("saveDisplayName"),
        Some(&json!("TestSave"))
    );
    assert_eq!(save.group(11).objects().len(), 0, "trailing empty group");
}

#[test]
fn unmodified_save_round_trips_byte_for_byte() {
    let text = common::minimal();
    assert_eq!(parse(&text).to_text(), text);
}

#[test]
fn sample_save_round_trips_byte_for_byte() {
    let bytes = common::sample();
    let text = std::str::from_utf8(&bytes).unwrap();
    let save = parse(text);
    assert_eq!(save.group(2).objects().len(), 5);
    assert_eq!(save.group(3).objects().len(), 7460);
    assert_eq!(save.group(4).objects().len(), 750);
    assert_eq!(save.to_text(), text);
}

#[test]
fn separators_inside_strings_are_text() {
    let mut groups = common::GROUPS;
    groups[8] = &[r#"{"saveDisplayName":"Base @ Lake | 2","planetId":"Prime"}"#];
    groups[4] = &[r#"{"id":5,"woIds":"10|11","size":4}"#];
    let text = common::build(&groups);
    let save = parse(&text);
    assert_eq!(
        save.single(8).get("saveDisplayName"),
        Some(&json!("Base @ Lake | 2"))
    );
    assert_eq!(save.group(4).objects().len(), 1);
    assert_eq!(save.to_text(), text);
}

#[test]
fn odd_whitespace_and_byte_order_mark_round_trip() {
    let mut text = String::from("\u{feff}");
    text.push_str(&common::minimal().replace("|\n", " \t|\r\n  "));
    text.push_str("\n\n");
    let save = parse(&text);
    assert_eq!(save.group(2).objects().len(), 2);
    assert_eq!(save.to_text(), text);
}

#[test]
fn edited_object_is_rewritten_and_keeps_other_number_text() {
    let text = common::minimal();
    let mut save = parse(&text);
    assert!(save.group_mut(1).objects_mut()[0].set("unitHeatLevel", json!(3.5)));
    let expected = text.replace(r#""unitHeatLevel":2.5"#, r#""unitHeatLevel":3.5"#);
    assert_eq!(save.to_text(), expected);
    assert!(expected.contains(r#""unitPlantsLevel":44554280960.0"#));
}

#[test]
fn setting_the_same_value_is_no_edit() {
    let mut save = parse(&common::minimal());
    assert!(!save.single_mut(0).set("terraTokens", json!(100)));
}

#[test]
fn large_ids_keep_their_digits_after_an_edit() {
    let text = common::minimal();
    let mut save = parse(&text);
    assert!(save.group_mut(2).objects_mut()[0].set("name", json!("Alicia")));
    let out = save.to_text();
    assert!(
        out.contains(r#"{"id":76561198047777538,"name":"Alicia","#),
        "{out}"
    );
    assert!(out.contains(r#""playerGaugeOxygen":100.0,"#), "{out}");
    assert!(
        out.contains(r#""playerGaugeThirst":37.343666076660159,"#),
        "{out}"
    );
}

#[test]
fn pushed_objects_copy_the_group_separator() {
    let mut save = parse(&common::minimal());
    let mut fields = Map::new();
    fields.insert("id".to_owned(), json!(13));
    save.group_mut(3).push(Object::new(fields.clone()));
    assert!(
        save.to_text()
            .contains("{\"id\":12,\"gId\":\"Titanium\"}|\n{\"id\":13}\r@")
    );

    let mut groups = common::GROUPS;
    groups[6] = &[];
    let mut save = parse(&common::build(&groups));
    save.group_mut(6).push(Object::new(fields));
    assert!(save.to_text().contains("@\r\r{\"id\":13}@"));
}

#[test]
fn saves_without_the_trailing_group_parse() {
    // Game version 2.008 writes 11 groups and no trailing empty one.
    let text = common::minimal();
    let text = text.strip_suffix('@').unwrap();
    let save = parse(text);
    assert_eq!(save.group(10).objects().len(), 1);
    assert_eq!(save.to_text(), text);
}

#[test]
fn too_few_groups_are_rejected() {
    assert_eq!(
        parse_err("{}@{}@{}"),
        "expected at least 11 groups separated by '@', found 3"
    );
}

#[test]
fn single_groups_must_hold_one_object() {
    let mut groups = common::GROUPS;
    groups[0] = &[r#"{"terraTokens":1}"#, r#"{"terraTokens":2}"#];
    assert_eq!(
        parse_err(&common::build(&groups)),
        "TerraTokens must hold exactly one object, found 2"
    );
    groups = common::GROUPS;
    groups[5] = &[];
    assert_eq!(
        parse_err(&common::build(&groups)),
        "Stats must hold exactly one object, found 0"
    );
}

#[test]
fn malformed_text_is_rejected_with_its_location() {
    let text = common::minimal();
    let err = parse_err(&text.replacen(r#"{"id":11,"gId":"Iron"}"#, r#"{"id":11,"gId":}"#, 1));
    assert!(
        err.starts_with("WorldObjects object 2 is not valid JSON"),
        "{err}"
    );

    let err = parse_err(&text.replacen("|\n{\"id\":11,", "|\nx{\"id\":11,", 1));
    assert!(err.starts_with("unexpected 'x' at byte"), "{err}");
    assert!(err.ends_with("in WorldObjects"), "{err}");

    let err = parse_err(&text.replacen("|\n{\"id\":11,", "\n{\"id\":11,", 1));
    assert!(err.starts_with("unexpected '{' at byte"), "{err}");

    let err = parse_err(&text.replacen("\r@\r{\"id\":5", "|\r@\r{\"id\":5", 1));
    assert!(err.starts_with("unexpected '@' at byte"), "{err}");

    assert_eq!(
        parse_err(&format!("{text}{{\"a\":1}}|")),
        "group 11 ends with '|'"
    );
    assert_eq!(
        parse_err(&text.replacen("{\"terraTokens\"", "[1]", 1)),
        "unexpected '[' at byte 1 in TerraTokens"
    );
}

#[test]
fn values_are_json_objects() {
    let save = parse(&common::minimal());
    let fields = save.single(5).fields();
    assert_eq!(
        Value::Object(fields.clone()),
        json!({ "craftedObjects": 50, "totalSaveFileLoad": 0, "totalSaveFileTime": 3600 })
    );
}

fn edited_text(groups: [&[&str]; 11], edit: impl FnOnce(&mut SaveFile)) -> String {
    let mut save = parse(&common::build(&groups));
    edit(&mut save);
    save.to_text()
}

#[test]
fn edits_keep_the_spelling_of_untouched_values() {
    let mut groups = common::GROUPS;
    groups[0] = &[r#"{"a":"café \/","terraTokens":1,"allTimeTerraTokens":2}"#];
    let text = edited_text(groups, |save| {
        assert!(save.single_mut(0).set("terraTokens", json!(2)));
    });
    assert!(
        text.contains(r#"{"a":"café \/","terraTokens":2,"allTimeTerraTokens":2}"#),
        "{text}"
    );
}

#[test]
fn duplicate_keys_survive_and_the_last_one_is_edited() {
    let mut groups = common::GROUPS;
    groups[9] = &[r#"{"x":1,"x":2}"#];
    let save = parse(&common::build(&groups));
    assert_eq!(save.group(9).objects()[0].get("x"), Some(&json!(2)));
    let text = edited_text(groups, |save| {
        assert!(save.group_mut(9).objects_mut()[0].set("x", json!(3)));
    });
    assert!(text.contains(r#"{"x":1,"x":3}"#), "{text}");
}

#[test]
fn new_keys_are_appended() {
    let mut groups = common::GROUPS;
    groups[7] = &["{}"];
    groups[9] = &[r#"{ "a": 1 }"#];
    groups[10] = &[r#"{"a":1}"#];
    let text = edited_text(groups, |save| {
        assert!(save.group_mut(9).objects_mut()[0].set("a", json!(5)));
        assert!(save.group_mut(10).objects_mut()[0].set("b", json!(true)));
        assert!(save.group_mut(7).objects_mut()[0].set("b", json!(true)));
    });
    assert!(text.contains(r#"{ "a": 5 }"#), "{text}");
    assert!(text.contains(r#"{"a":1,"b":true}"#), "{text}");
    assert!(text.contains(r#"{"b":true}"#), "{text}");
}

#[test]
fn errors_report_positions_in_the_file() {
    let broken = r#"{"id":11,}"#;
    let text = common::minimal().replacen(r#"{"id":11,"gId":"Iron"}"#, broken, 1);
    let start = text.find(broken).unwrap();
    let line = text[..start].matches('\n').count() + 1;
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    // serde points at the closing brace, the 10th character of the object.
    let column = start - line_start + 10;
    let err = parse_err(&text);
    assert!(
        err.starts_with(&format!(
            "WorldObjects object 2 is not valid JSON at line {line} column {column}: "
        )),
        "{err}"
    );
}
