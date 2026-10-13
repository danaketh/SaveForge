//! Port of the .NET `VisibilityRuleTests`, plus missing values and loose
//! number/string comparison.

use saveforge_sdk::form::{VisibilityOp, VisibilityRule};
use serde_json::{Value, json};

fn rule(op: VisibilityOp, value: Value) -> VisibilityRule {
    VisibilityRule::new("x", op, value)
}

#[test]
fn equals_and_not_equals_strings() {
    assert!(rule(VisibilityOp::Equals, json!("admin")).evaluate(Some(&json!("admin"))));
    assert!(!rule(VisibilityOp::Equals, json!("admin")).evaluate(Some(&json!("user"))));
    assert!(rule(VisibilityOp::NotEquals, json!("admin")).evaluate(Some(&json!("user"))));
}

#[test]
fn greater_and_less_than_numbers() {
    assert!(rule(VisibilityOp::GreaterThan, json!(5)).evaluate(Some(&json!(10))));
    assert!(!rule(VisibilityOp::GreaterThan, json!(5)).evaluate(Some(&json!(5))));
    assert!(rule(VisibilityOp::LessThan, json!(10)).evaluate(Some(&json!(3))));
    assert!(rule(VisibilityOp::LessThan, json!(10.5)).evaluate(Some(&json!(10))));
}

#[test]
fn null_and_missing_values() {
    assert!(rule(VisibilityOp::Equals, Value::Null).evaluate(None));
    assert!(rule(VisibilityOp::Equals, Value::Null).evaluate(Some(&Value::Null)));
    assert!(!rule(VisibilityOp::Equals, json!(true)).evaluate(None));
    assert!(!rule(VisibilityOp::GreaterThan, json!(1)).evaluate(None));
    assert!(!rule(VisibilityOp::LessThan, json!(1)).evaluate(None));
}

#[test]
fn numeric_strings_compare_as_numbers() {
    assert!(rule(VisibilityOp::Equals, json!(5)).evaluate(Some(&json!("5"))));
    assert!(rule(VisibilityOp::GreaterThan, json!("9")).evaluate(Some(&json!(10))));
    assert!(rule(VisibilityOp::LessThan, json!("b")).evaluate(Some(&json!("a"))));
    assert!(!rule(VisibilityOp::GreaterThan, json!("b")).evaluate(Some(&json!(true))));
}

#[test]
fn booleans_match_exactly() {
    assert!(rule(VisibilityOp::Equals, json!(true)).evaluate(Some(&json!(true))));
    assert!(!rule(VisibilityOp::Equals, json!(true)).evaluate(Some(&json!(false))));
}
