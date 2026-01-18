// Tests for msg::errors to improve coverage

use ros2msg::msg::errors::*;

#[test]
fn test_parse_error_invalid_resource_name() {
    let err = ParseError::InvalidResourceName {
        name: "bad-name".to_string(),
        reason: "contains dash".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("bad-name"));
    assert!(msg.contains("contains dash"));
}

#[test]
fn test_parse_error_invalid_type() {
    let err = ParseError::InvalidType {
        type_string: "unknown_type".to_string(),
        reason: "not found".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("unknown_type"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_parse_error_invalid_value() {
    let err = ParseError::InvalidValue {
        value: "abc".to_string(),
        type_info: "int32".to_string(),
        reason: "not a number".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("abc"));
    assert!(msg.contains("int32"));
}

#[test]
fn test_parse_error_invalid_service() {
    let err = ParseError::InvalidServiceSpecification {
        reason: "missing separator".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("missing separator"));
}

#[test]
fn test_parse_error_invalid_action() {
    let err = ParseError::InvalidActionSpecification {
        reason: "wrong number of sections".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("wrong number of sections"));
}

#[test]
fn test_parse_error_line_parse() {
    let err = ParseError::LineParseError {
        line: 42,
        message: "syntax error".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("42"));
    assert!(msg.contains("syntax error"));
}

#[test]
fn test_parse_error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = ParseError::IoError(io_err);
    let msg = format!("{}", err);
    assert!(msg.contains("file not found"));
}

#[test]
fn test_parse_error_invalid_array() {
    let err = ParseError::InvalidArray {
        reason: "size must be positive".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("size must be positive"));
}

#[test]
fn test_parse_error_invalid_constant() {
    let err = ParseError::InvalidConstant {
        reason: "invalid value".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("invalid value"));
}

#[test]
fn test_parse_error_invalid_field() {
    let err = ParseError::InvalidField {
        reason: "invalid name".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("invalid name"));
}

#[test]
fn test_parse_error_invalid_annotation() {
    let err = ParseError::InvalidAnnotation {
        annotation: "custom".to_string(),
        reason: "unknown annotation".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("custom"));
    assert!(msg.contains("unknown annotation"));
}

#[test]
fn test_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test error");
    let parse_err: ParseError = io_err.into();
    assert!(matches!(parse_err, ParseError::IoError { .. }));
}

#[test]
fn test_helper_invalid_resource_name() {
    let err = invalid_resource_name("test", "pattern");
    assert!(matches!(err, ParseError::InvalidResourceName { .. }));
    let msg = format!("{}", err);
    assert!(msg.contains("test"));
    assert!(msg.contains("pattern"));
}

#[test]
fn test_helper_invalid_value() {
    let err = invalid_value("value", "type", "reason");
    assert!(matches!(err, ParseError::InvalidValue { .. }));
    let msg = format!("{}", err);
    assert!(msg.contains("value"));
    assert!(msg.contains("type"));
    assert!(msg.contains("reason"));
}

#[test]
fn test_helper_invalid_type() {
    let err = invalid_type("typename", "reason");
    assert!(matches!(err, ParseError::InvalidType { .. }));
    let msg = format!("{}", err);
    assert!(msg.contains("typename"));
    assert!(msg.contains("reason"));
}
