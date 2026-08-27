//! First-party JSON parsing, query, and serialization utilities powered by `serde_json`.
//!
//! Provides dynamic querying (`get_string`, `get_float`, `get_int`) and zero-panic
//! structured results per `ADOPTED_DEPENDENCIES.md` and `note.md`.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Structured JSON parsing or serialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub message: String,
}

impl JsonError {
    pub fn new(message: impl fmt::Display) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JsonError: {}", self.message)
    }
}

impl std::error::Error for JsonError {}

/// Dynamic JSON value representing scalar and compound data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    /// Retrieve string value by key if this is an object.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self {
            JsonValue::Object(map) => map.get(key).and_then(|v| v.as_str()),
            _ => None,
        }
    }

    /// Retrieve float value by key if this is an object.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self {
            JsonValue::Object(map) => map.get(key).and_then(|v| v.as_float()),
            _ => None,
        }
    }

    /// Retrieve integer value by key if this is an object.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self {
            JsonValue::Object(map) => map.get(key).and_then(|v| v.as_int()),
            _ => None,
        }
    }

    /// Retrieve boolean value by key if this is an object.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self {
            JsonValue::Object(map) => map.get(key).and_then(|v| v.as_bool()),
            _ => None,
        }
    }

    /// Retrieve array slice by key if this is an object.
    pub fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Object(map) => map.get(key).and_then(|v| v.as_array()),
            _ => None,
        }
    }

    /// Return inner string slice if value is string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Return inner float if value is numeric.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Return inner integer if value is numeric and convertible.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && *n >= (i64::MIN as f64) && *n <= (i64::MAX as f64) {
                    Some(*n as i64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Return inner boolean if value is bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Return inner array slice if value is array.
    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    /// Return inner object map if value is object.
    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(map) => Some(map),
            _ => None,
        }
    }
}

/// Parse a JSON string into a dynamic `JsonValue`.
pub fn parse(text: &str) -> Result<JsonValue, JsonError> {
    serde_json::from_str(text).map_err(|e| JsonError::new(e))
}

/// Serialize a `JsonValue` into a compact JSON string.
pub fn stringify(val: &JsonValue) -> Result<String, JsonError> {
    serde_json::to_string(val).map_err(|e| JsonError::new(e))
}

/// Serialize a `JsonValue` into an indented, human-readable JSON string.
pub fn stringify_pretty(val: &JsonValue) -> Result<String, JsonError> {
    serde_json::to_string_pretty(val).map_err(|e| JsonError::new(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parse_and_query() {
        let json_text = r#"
        {
            "name": "Agam",
            "version": 1,
            "speedup": 4.2,
            "active": true,
            "backends": ["cranelift", "llvm"]
        }
        "#;

        let parsed = parse(json_text);
        assert!(parsed.is_ok());
        if let Ok(v) = parsed {
            assert_eq!(v.get_string("name"), Some("Agam"));
            assert_eq!(v.get_int("version"), Some(1));
            assert_eq!(v.get_float("speedup"), Some(4.2));
            assert_eq!(v.get_bool("active"), Some(true));

            let backends = v.get_array("backends");
            assert!(backends.is_some());
            if let Some(arr) = backends {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0].as_str(), Some("cranelift"));
                assert_eq!(arr[1].as_str(), Some("llvm"));
            }
        }
    }

    #[test]
    fn test_json_stringify_roundtrip() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), JsonValue::String("value".to_string()));
        let original = JsonValue::Object(map);

        let serialized = stringify(&original);
        assert!(serialized.is_ok());
        if let Ok(text) = serialized {
            let deserialized = parse(&text);
            assert!(deserialized.is_ok());
            if let Ok(val) = deserialized {
                assert_eq!(val, original);
            }
        }
    }

    #[test]
    fn test_invalid_json_returns_error() {
        let bad = parse("{ not valid json }");
        assert!(bad.is_err());
    }
}
