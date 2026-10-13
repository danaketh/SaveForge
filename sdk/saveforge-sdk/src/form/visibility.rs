//! Conditional fields (port of the .NET `VisibilityRule`).

use std::cmp::Ordering;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityOp {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
}

/// Shows a field only while the value at `depends_on` (an absolute dot path)
/// satisfies `op value`.
#[derive(Debug, Clone, PartialEq)]
pub struct VisibilityRule {
    pub depends_on: String,
    pub op: VisibilityOp,
    pub value: Value,
}

impl VisibilityRule {
    pub fn new(depends_on: impl Into<String>, op: VisibilityOp, value: impl Into<Value>) -> Self {
        Self {
            depends_on: depends_on.into(),
            op,
            value: value.into(),
        }
    }

    /// `current` is the value at `depends_on`; `None` (missing) counts as null.
    /// Numbers compare numerically, also when one side is a numeric string;
    /// strings order lexically. Values that can't be ordered are neither
    /// greater nor less.
    pub fn evaluate(&self, current: Option<&Value>) -> bool {
        let current = current.unwrap_or(&Value::Null);
        match self.op {
            VisibilityOp::Equals => loose_eq(current, &self.value),
            VisibilityOp::NotEquals => !loose_eq(current, &self.value),
            VisibilityOp::GreaterThan => compare(current, &self.value) == Some(Ordering::Greater),
            VisibilityOp::LessThan => compare(current, &self.value) == Some(Ordering::Less),
        }
    }
}

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

fn loose_eq(a: &Value, b: &Value) -> bool {
    if a == b {
        return true;
    }
    match (as_number(a), as_number(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y) == Some(Ordering::Equal),
        _ => false,
    }
}

fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    if let (Some(x), Some(y)) = (as_number(a), as_number(b)) {
        return x.partial_cmp(&y);
    }
    match (a, b) {
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        _ => None,
    }
}
