//! Runtime values for the Patchwork interpreter.

use std::collections::HashMap;
use std::fmt;

/// A runtime value in the Patchwork language.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The null value.
    Null,
    /// A string value.
    String(String),
    /// A numeric value (always f64, like JavaScript).
    Number(f64),
    /// A boolean value.
    Boolean(bool),
    /// An array of values.
    Array(Vec<Value>),
    /// An object with string keys.
    Object(HashMap<String, Value>),
}

impl Value {
    /// Coerce this value to a string.
    pub fn to_string_value(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::String(s) => s.clone(),
            Value::Number(n) => {
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if *n > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
                } else if *n == n.trunc() && n.abs() < 1e15 {
                    // Integer-like numbers without decimal point
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string_value()).collect();
                items.join(", ")
            }
            Value::Object(_) => "[object Object]".to_string(),
        }
    }

    /// Coerce this value to a boolean.
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Boolean(b) => *b,
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(_) => true,
        }
    }

    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_value())
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}
