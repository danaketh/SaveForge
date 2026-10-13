use saveforge_sdk::form::path::{get, get_mut, join, set};
use serde_json::{Value, json};

#[test]
fn get_walks_objects_and_arrays() {
    let v =
        json!({ "player": { "stats": { "hp": 5 } }, "items": [ { "id": "a" }, { "id": "b" } ] });
    assert_eq!(get(&v, "player.stats.hp"), Some(&json!(5)));
    assert_eq!(get(&v, "items.1.id"), Some(&json!("b")));
    assert_eq!(get(&v, "items.9.id"), None);
    assert_eq!(get(&v, "items.x"), None);
    assert_eq!(get(&v, "player.missing"), None);
    assert_eq!(get(&v, ""), Some(&v));
}

#[test]
fn set_creates_missing_objects_and_reports_changes() {
    let mut v = json!({ "player": {} });
    assert!(set(&mut v, "player.stats.hp", json!(10)));
    assert_eq!(v, json!({ "player": { "stats": { "hp": 10 } } }));
    assert!(
        !set(&mut v, "player.stats.hp", json!(10)),
        "same value is no change"
    );
    assert!(set(&mut v, "player.stats.hp", json!(11)));
}

#[test]
fn set_into_arrays_needs_existing_index() {
    let mut v = json!({ "items": [ { "id": "a" } ] });
    assert!(set(&mut v, "items.0.id", json!("z")));
    assert!(!set(&mut v, "items.5.id", json!("z")));
    assert!(
        !set(&mut v, "items.0.id.deeper", json!(1)),
        "cannot descend into a string"
    );
    assert_eq!(v, json!({ "items": [ { "id": "z" } ] }));
}

#[test]
fn get_mut_and_join() {
    let mut v = json!({ "items": [1, 2] });
    if let Some(Value::Array(items)) = get_mut(&mut v, "items") {
        items.push(json!(3));
    }
    assert_eq!(v, json!({ "items": [1, 2, 3] }));
    assert_eq!(join("", "a"), "a");
    assert_eq!(join("a.0", "b"), "a.0.b");
}
