//! IDL value types for constants and expressions.

#![allow(clippy::must_use_candidate)]

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// IDL value types that can be used in constants and expressions
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum IdlValue {
    /// Boolean value
    Bool(bool),
    /// 8-bit signed integer
    Int8(i8),
    /// 8-bit unsigned integer  
    UInt8(u8),
    /// 16-bit signed integer
    Int16(i16),
    /// 16-bit unsigned integer
    UInt16(u16),
    /// 32-bit signed integer
    Int32(i32),
    /// 32-bit unsigned integer
    UInt32(u32),
    /// 64-bit signed integer
    Int64(i64),
    /// 64-bit unsigned integer
    UInt64(u64),
    /// 32-bit floating point
    Float32(f32),
    /// 64-bit floating point
    Float64(f64),
    /// Character
    Char(char),
    /// String value
    String(String),
    /// Object/dictionary with string keys and arbitrary values
    Object(HashMap<String, IdlValue>),
    /// Array/list of values
    Array(Vec<IdlValue>),
    /// Null/None value
    Null,
}

impl IdlValue {
    /// Check if this value is null
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, IdlValue::Null)
    }

    /// Get as boolean if it is one
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            IdlValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as string if it is one
    pub fn as_string(&self) -> Option<&str> {
        match self {
            IdlValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as object if it is one
    pub fn as_object(&self) -> Option<&HashMap<String, IdlValue>> {
        match self {
            IdlValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Get as array if it is one
    pub fn as_array(&self) -> Option<&[IdlValue]> {
        match self {
            IdlValue::Array(arr) => Some(arr),
            _ => None,
        }
    }
}

impl std::fmt::Display for IdlValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdlValue::Bool(b) => write!(f, "{b}"),
            IdlValue::Int8(i) => write!(f, "{i}"),
            IdlValue::UInt8(u) => write!(f, "{u}"),
            IdlValue::Int16(i) => write!(f, "{i}"),
            IdlValue::UInt16(u) => write!(f, "{u}"),
            IdlValue::Int32(i) => write!(f, "{i}"),
            IdlValue::UInt32(u) => write!(f, "{u}"),
            IdlValue::Int64(i) => write!(f, "{i}"),
            IdlValue::UInt64(u) => write!(f, "{u}"),
            IdlValue::Float32(fl) => write!(f, "{fl}"),
            IdlValue::Float64(fl) => write!(f, "{fl}"),
            IdlValue::Char(c) => write!(f, "'{c}'"),
            IdlValue::String(s) => write!(f, "\"{s}\""),
            IdlValue::Object(_) => write!(f, "{{...}}"),
            IdlValue::Array(arr) => write!(f, "[{}]", arr.len()),
            IdlValue::Null => write!(f, "null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idl_value_is_null() {
        let val = IdlValue::Null;
        assert!(val.is_null());

        let val = IdlValue::Bool(false);
        assert!(!val.is_null());
    }

    #[test]
    fn test_idl_value_as_bool() {
        let val = IdlValue::Bool(true);
        assert_eq!(val.as_bool(), Some(true));

        let val = IdlValue::Int32(42);
        assert_eq!(val.as_bool(), None);
    }

    #[test]
    fn test_idl_value_as_string() {
        let val = IdlValue::String("test".to_string());
        assert_eq!(val.as_string(), Some("test"));

        let val = IdlValue::Bool(true);
        assert_eq!(val.as_string(), None);
    }

    #[test]
    fn test_idl_value_as_object() {
        let mut obj = HashMap::new();
        obj.insert("key".to_string(), IdlValue::Int32(42));
        let val = IdlValue::Object(obj);
        assert!(val.as_object().is_some());

        let val = IdlValue::Bool(true);
        assert!(val.as_object().is_none());
    }

    #[test]
    fn test_idl_value_as_array() {
        let arr = vec![IdlValue::Int32(1), IdlValue::Int32(2)];
        let val = IdlValue::Array(arr);
        assert_eq!(val.as_array().unwrap().len(), 2);

        let val = IdlValue::Bool(true);
        assert!(val.as_array().is_none());
    }

    #[test]
    fn test_idl_value_display() {
        assert_eq!(IdlValue::Bool(true).to_string(), "true");
        assert_eq!(IdlValue::Int32(42).to_string(), "42");
        assert_eq!(IdlValue::Float64(1.5).to_string(), "1.5");
        assert_eq!(IdlValue::Char('x').to_string(), "'x'");
        assert_eq!(IdlValue::String("test".to_string()).to_string(), "\"test\"");
        assert_eq!(IdlValue::Null.to_string(), "null");

        let arr = vec![IdlValue::Int32(1), IdlValue::Int32(2)];
        assert_eq!(IdlValue::Array(arr).to_string(), "[2]");
    }
}
