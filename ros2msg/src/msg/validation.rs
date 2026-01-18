use super::errors::{ParseResult, invalid_value};
/// Validation utilities for ROS2 message parsing
use regex::Regex;
use std::sync::LazyLock;

// Constants for ROS2 message format
#[allow(missing_docs)]
pub const PACKAGE_NAME_MESSAGE_TYPE_SEPARATOR: &str = "/";
#[allow(missing_docs)]
pub const ANNOTATION_DELIMITER: &str = "@";
#[allow(missing_docs)]
pub const OPTIONAL_ANNOTATION: &str = "@optional";
#[allow(missing_docs)]
pub const COMMENT_DELIMITER: &str = "#";
#[allow(missing_docs)]
pub const CONSTANT_SEPARATOR: &str = "=";
#[allow(missing_docs)]
pub const ARRAY_UPPER_BOUND_TOKEN: &str = "<=";
#[allow(missing_docs)]
pub const STRING_UPPER_BOUND_TOKEN: &str = "<=";
#[allow(missing_docs)]
pub const SERVICE_REQUEST_RESPONSE_SEPARATOR: &str = "---";
#[allow(missing_docs)]
pub const SERVICE_REQUEST_MESSAGE_SUFFIX: &str = "_Request";
#[allow(missing_docs)]
pub const SERVICE_RESPONSE_MESSAGE_SUFFIX: &str = "_Response";
#[allow(missing_docs)]
pub const SERVICE_EVENT_MESSAGE_SUFFIX: &str = "_Event";
#[allow(missing_docs)]
pub const ACTION_REQUEST_RESPONSE_SEPARATOR: &str = "---";
#[allow(missing_docs)]
pub const ACTION_GOAL_SUFFIX: &str = "_Goal";
#[allow(missing_docs)]
pub const ACTION_RESULT_SUFFIX: &str = "_Result";
#[allow(missing_docs)]
pub const ACTION_FEEDBACK_SUFFIX: &str = "_Feedback";

/// ROS2 primitive types
pub const PRIMITIVE_TYPES: &[&str] = &[
    "bool", "byte", "char", "float32", "float64", "int8", "uint8", "int16", "uint16", "int32",
    "uint32", "int64", "uint64", "string", "wstring", "duration", "time",
];

// Regex patterns for validation
static VALID_PACKAGE_NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z]([a-z0-9_])*$").unwrap());

static VALID_MESSAGE_NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]([A-Za-z0-9_])*$").unwrap());

static VALID_FIELD_NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z]([a-z0-9_])*$").unwrap());

static VALID_CONSTANT_NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]([A-Z0-9_])*$").unwrap());

/// Validate a package name
pub fn is_valid_package_name(name: &str) -> bool {
    VALID_PACKAGE_NAME_PATTERN.is_match(name)
}

/// Validate a message name
pub fn is_valid_message_name(name: &str) -> bool {
    VALID_MESSAGE_NAME_PATTERN.is_match(name)
}

/// Validate a field name
pub fn is_valid_field_name(name: &str) -> bool {
    VALID_FIELD_NAME_PATTERN.is_match(name)
}

/// Validate a constant name
pub fn is_valid_constant_name(name: &str) -> bool {
    VALID_CONSTANT_NAME_PATTERN.is_match(name)
}

/// Parse a primitive value from string
///
/// # Errors
///
/// Returns an error if:
/// - The primitive type is not recognized or supported
/// - The value string cannot be parsed for the specified primitive type
/// - The value is out of range for numeric types
/// - String literals have invalid quote formats or escape sequences
pub fn parse_primitive_value_string(
    primitive_type: &str,
    value_string: &str,
) -> ParseResult<PrimitiveValue> {
    match primitive_type {
        "bool" => {
            let true_values = ["true", "1"];
            let false_values = ["false", "0"];
            let lower_value = value_string.to_lowercase();

            if true_values.contains(&lower_value.as_str()) {
                Ok(PrimitiveValue::Bool(true))
            } else if false_values.contains(&lower_value.as_str()) {
                Ok(PrimitiveValue::Bool(false))
            } else {
                Err(invalid_value(
                    value_string,
                    primitive_type,
                    "must be either 'true' / '1' or 'false' / '0'",
                ))
            }
        }
        "byte" | "char" | "uint8" => parse_integer_value(value_string, 0, 255)
            .and_then(|v| {
                u8::try_from(v)
                    .map_err(|_| invalid_value(value_string, "uint8", "value out of range"))
            })
            .map(PrimitiveValue::UInt8),
        "int8" => parse_integer_value(value_string, -128, 127)
            .and_then(|v| {
                i8::try_from(v)
                    .map_err(|_| invalid_value(value_string, "int8", "value out of range"))
            })
            .map(PrimitiveValue::Int8),
        "uint16" => parse_integer_value(value_string, 0, 65535)
            .and_then(|v| {
                u16::try_from(v)
                    .map_err(|_| invalid_value(value_string, "uint16", "value out of range"))
            })
            .map(PrimitiveValue::UInt16),
        "int16" => parse_integer_value(value_string, -32768, 32767)
            .and_then(|v| {
                i16::try_from(v)
                    .map_err(|_| invalid_value(value_string, "int16", "value out of range"))
            })
            .map(PrimitiveValue::Int16),
        "uint32" => parse_unsigned_integer(value_string)
            .and_then(|v| {
                u32::try_from(v)
                    .map_err(|_| invalid_value(value_string, primitive_type, "value out of range"))
            })
            .map(PrimitiveValue::UInt32),
        "int32" => parse_signed_integer(value_string)
            .and_then(|v| {
                i32::try_from(v)
                    .map_err(|_| invalid_value(value_string, primitive_type, "value out of range"))
            })
            .map(PrimitiveValue::Int32),
        "uint64" => parse_unsigned_integer(value_string).map(PrimitiveValue::UInt64),
        "int64" => parse_signed_integer(value_string).map(PrimitiveValue::Int64),
        "float32" => value_string
            .parse::<f32>()
            .map(PrimitiveValue::Float32)
            .map_err(|_| invalid_value(value_string, primitive_type, "must be a valid float")),
        "float64" => value_string
            .parse::<f64>()
            .map(PrimitiveValue::Float64)
            .map_err(|_| invalid_value(value_string, primitive_type, "must be a valid float")),
        "string" | "wstring" => Ok(PrimitiveValue::String(parse_string_literal(value_string)?)),
        "duration" | "time" => {
            // Duration and time are typically represented as strings or structured types
            // For simplicity, we'll treat them as strings for now
            Ok(PrimitiveValue::String(parse_string_literal(value_string)?))
        }
        _ => Err(invalid_value(
            value_string,
            primitive_type,
            "unknown primitive type",
        )),
    }
}

/// Parse string literal, handling escape sequences
///
/// # Errors
///
/// Returns an error if:
/// - The string has mismatched quotes
/// - The string contains invalid escape sequences
///
/// # Panics
///
/// Panics if the trimmed string is empty (this should not happen in normal usage)
pub fn parse_string_literal(value_string: &str) -> ParseResult<String> {
    let trimmed = value_string.trim();

    // Handle quoted strings
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let quote_char = trimmed.chars().next().unwrap();
        let content = &trimmed[1..trimmed.len() - 1];

        // Process escape sequences
        let mut result = String::new();
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        'n' => {
                            result.push('\n');
                            chars.next();
                        }
                        't' => {
                            result.push('\t');
                            chars.next();
                        }
                        'r' => {
                            result.push('\r');
                            chars.next();
                        }
                        '\\' => {
                            result.push('\\');
                            chars.next();
                        }
                        c if c == quote_char => {
                            result.push(c);
                            chars.next();
                        }
                        _ => result.push(ch),
                    }
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    } else {
        // Unquoted string
        Ok(trimmed.to_string())
    }
}

/// Parse integer with range validation
fn parse_integer_value(value_string: &str, min: i64, max: i64) -> ParseResult<i64> {
    parse_signed_integer(value_string).and_then(|v| {
        if v >= min && v <= max {
            Ok(v)
        } else {
            Err(invalid_value(
                value_string,
                "integer",
                &format!("must be between {min} and {max}"),
            ))
        }
    })
}

/// Parse signed integer (supports decimal, hex, octal, binary)
fn parse_signed_integer(value_string: &str) -> ParseResult<i64> {
    value_string
        .parse::<i64>()
        .or_else(|_| {
            // Try parsing with different bases
            if value_string.starts_with("0x") || value_string.starts_with("0X") {
                i64::from_str_radix(&value_string[2..], 16).map_err(|_| ())
            } else if value_string.starts_with("0b") || value_string.starts_with("0B") {
                i64::from_str_radix(&value_string[2..], 2).map_err(|_| ())
            } else if value_string.starts_with('0') && value_string.len() > 1 {
                i64::from_str_radix(&value_string[1..], 8).map_err(|_| ())
            } else {
                Err(())
            }
        })
        .map_err(|()| invalid_value(value_string, "integer", "must be a valid integer"))
}

/// Parse unsigned integer (supports decimal, hex, octal, binary)
fn parse_unsigned_integer(value_string: &str) -> ParseResult<u64> {
    value_string
        .parse::<u64>()
        .or_else(|_| {
            // Try parsing with different bases
            if value_string.starts_with("0x") || value_string.starts_with("0X") {
                u64::from_str_radix(&value_string[2..], 16).map_err(|_| ())
            } else if value_string.starts_with("0b") || value_string.starts_with("0B") {
                u64::from_str_radix(&value_string[2..], 2).map_err(|_| ())
            } else if value_string.starts_with('0') && value_string.len() > 1 {
                u64::from_str_radix(&value_string[1..], 8).map_err(|_| ())
            } else {
                Err(())
            }
        })
        .map_err(|()| {
            invalid_value(
                value_string,
                "unsigned integer",
                "must be a valid unsigned integer",
            )
        })
}

/// Primitive value types
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum PrimitiveValue {
    Bool(bool),
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    String(String),
}

impl std::fmt::Display for PrimitiveValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveValue::Bool(v) => write!(f, "{v}"),
            PrimitiveValue::Int8(v) => write!(f, "{v}"),
            PrimitiveValue::UInt8(v) => write!(f, "{v}"),
            PrimitiveValue::Int16(v) => write!(f, "{v}"),
            PrimitiveValue::UInt16(v) => write!(f, "{v}"),
            PrimitiveValue::Int32(v) => write!(f, "{v}"),
            PrimitiveValue::UInt32(v) => write!(f, "{v}"),
            PrimitiveValue::Int64(v) => write!(f, "{v}"),
            PrimitiveValue::UInt64(v) => write!(f, "{v}"),
            PrimitiveValue::Float32(v) => write!(f, "{v}"),
            PrimitiveValue::Float64(v) => write!(f, "{v}"),
            PrimitiveValue::String(v) => write!(f, "\"{v}\""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_name_validation() {
        assert!(is_valid_package_name("test_package"));
        assert!(is_valid_package_name("geometry_msgs"));
        assert!(!is_valid_package_name("TestPackage")); // uppercase not allowed
        assert!(!is_valid_package_name("test-package")); // hyphen not allowed
    }

    #[test]
    fn test_message_name_validation() {
        assert!(is_valid_message_name("TestMessage"));
        assert!(is_valid_message_name("Pose"));
        assert!(!is_valid_message_name("testMessage")); // must start with uppercase
    }

    #[test]
    fn test_primitive_value_parsing() {
        assert_eq!(
            parse_primitive_value_string("bool", "true").unwrap(),
            PrimitiveValue::Bool(true)
        );
        assert_eq!(
            parse_primitive_value_string("int32", "42").unwrap(),
            PrimitiveValue::Int32(42)
        );
        assert_eq!(
            parse_primitive_value_string("string", "\"hello\"").unwrap(),
            PrimitiveValue::String("hello".to_string())
        );
    }

    #[test]
    fn test_field_name_validation() {
        assert!(is_valid_field_name("my_field"));
        assert!(is_valid_field_name("x"));
        assert!(is_valid_field_name("position_x"));
        assert!(!is_valid_field_name("MyField")); // uppercase not allowed
        assert!(!is_valid_field_name("my-field")); // hyphen not allowed
        assert!(!is_valid_field_name("")); // empty not allowed
    }

    #[test]
    fn test_constant_name_validation() {
        assert!(is_valid_constant_name("MY_CONSTANT"));
        assert!(is_valid_constant_name("MAX_VALUE"));
        assert!(is_valid_constant_name("PI"));
        assert!(!is_valid_constant_name("my_constant")); // must be uppercase
        assert!(!is_valid_constant_name("MyConstant")); // mixed case not allowed
        assert!(!is_valid_constant_name("")); // empty not allowed
    }

    #[test]
    fn test_parse_all_integer_types() {
        assert_eq!(
            parse_primitive_value_string("int8", "127").unwrap(),
            PrimitiveValue::Int8(127)
        );
        assert_eq!(
            parse_primitive_value_string("int8", "-128").unwrap(),
            PrimitiveValue::Int8(-128)
        );
        assert_eq!(
            parse_primitive_value_string("uint8", "255").unwrap(),
            PrimitiveValue::UInt8(255)
        );
        assert_eq!(
            parse_primitive_value_string("int16", "32767").unwrap(),
            PrimitiveValue::Int16(32767)
        );
        assert_eq!(
            parse_primitive_value_string("uint16", "65535").unwrap(),
            PrimitiveValue::UInt16(65535)
        );
        assert_eq!(
            parse_primitive_value_string("int32", "2147483647").unwrap(),
            PrimitiveValue::Int32(2_147_483_647)
        );
        assert_eq!(
            parse_primitive_value_string("uint32", "4294967295").unwrap(),
            PrimitiveValue::UInt32(4_294_967_295)
        );
        assert_eq!(
            parse_primitive_value_string("int64", "9223372036854775807").unwrap(),
            PrimitiveValue::Int64(9_223_372_036_854_775_807)
        );
    }

    #[test]
    fn test_parse_hex_values() {
        assert_eq!(
            parse_primitive_value_string("uint8", "0xFF").unwrap(),
            PrimitiveValue::UInt8(255)
        );
        assert_eq!(
            parse_primitive_value_string("uint16", "0x10").unwrap(),
            PrimitiveValue::UInt16(16)
        );
        assert_eq!(
            parse_primitive_value_string("uint16", "0xDEAD").unwrap(),
            PrimitiveValue::UInt16(0xDEAD)
        );
    }

    #[test]
    fn test_parse_float_values() {
        assert_eq!(
            parse_primitive_value_string("float32", "1.23").unwrap(),
            PrimitiveValue::Float32(1.23)
        );
        assert_eq!(
            parse_primitive_value_string("float64", "4.567").unwrap(),
            PrimitiveValue::Float64(4.567)
        );
        assert_eq!(
            parse_primitive_value_string("float32", "1.5e2").unwrap(),
            PrimitiveValue::Float32(150.0)
        );
        assert_eq!(
            parse_primitive_value_string("float64", "2.5e-3").unwrap(),
            PrimitiveValue::Float64(0.0025)
        );
    }

    #[test]
    fn test_parse_string_escape_sequences() {
        assert_eq!(
            parse_primitive_value_string("string", "\"hello\\nworld\"").unwrap(),
            PrimitiveValue::String("hello\nworld".to_string())
        );
        assert_eq!(
            parse_primitive_value_string("string", "\"tab\\there\"").unwrap(),
            PrimitiveValue::String("tab\there".to_string())
        );
        assert_eq!(
            parse_primitive_value_string("string", "\"quote\\\"test\"").unwrap(),
            PrimitiveValue::String("quote\"test".to_string())
        );
    }

    #[test]
    fn test_parse_invalid_values() {
        assert!(parse_primitive_value_string("bool", "maybe").is_err());
        assert!(parse_primitive_value_string("uint8", "256").is_err());
        assert!(parse_primitive_value_string("int8", "-129").is_err());
        assert!(parse_primitive_value_string("int32", "abc").is_err());
    }

    #[test]
    fn test_primitive_value_display() {
        assert_eq!(PrimitiveValue::Bool(true).to_string(), "true");
        assert_eq!(PrimitiveValue::Int32(42).to_string(), "42");
        assert_eq!(PrimitiveValue::Float64(1.5).to_string(), "1.5");
        assert_eq!(
            PrimitiveValue::String("test".to_string()).to_string(),
            "\"test\""
        );
    }

    #[test]
    fn test_primitive_types_constant() {
        assert!(PRIMITIVE_TYPES.contains(&"bool"));
        assert!(PRIMITIVE_TYPES.contains(&"int32"));
        assert!(PRIMITIVE_TYPES.contains(&"float64"));
        assert!(PRIMITIVE_TYPES.contains(&"string"));
        assert!(!PRIMITIVE_TYPES.contains(&"CustomType"));
    }
}
