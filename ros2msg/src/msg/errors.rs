/// Error types for ROS2 message parsing
use thiserror::Error;

/// Main error type for ROS2 message parsing
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum ParseError {
    #[error("Invalid resource name: {name} - {reason}")]
    InvalidResourceName { name: String, reason: String },

    #[error("Invalid type definition: {type_string} - {reason}")]
    InvalidType { type_string: String, reason: String },

    #[error("Invalid value: {value} for type {type_info} - {reason}")]
    InvalidValue {
        value: String,
        type_info: String,
        reason: String,
    },

    #[error("Invalid service specification: {reason}")]
    InvalidServiceSpecification { reason: String },

    #[error("Invalid action specification: {reason}")]
    InvalidActionSpecification { reason: String },

    #[error("Parse error at line {line}: {message}")]
    LineParseError { line: usize, message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid array definition: {reason}")]
    InvalidArray { reason: String },

    #[error("Invalid constant definition: {reason}")]
    InvalidConstant { reason: String },

    #[error("Invalid field definition: {reason}")]
    InvalidField { reason: String },

    #[error("Invalid annotation: {annotation} - {reason}")]
    InvalidAnnotation { annotation: String, reason: String },

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Helper function to create `InvalidResourceName` errors
#[must_use]
pub fn invalid_resource_name(name: &str, pattern: &str) -> ParseError {
    ParseError::InvalidResourceName {
        name: name.to_string(),
        reason: format!("should match pattern: {pattern}"),
    }
}

/// Helper function to create `InvalidValue` errors
#[must_use]
pub fn invalid_value(value: &str, type_info: &str, reason: &str) -> ParseError {
    ParseError::InvalidValue {
        value: value.to_string(),
        type_info: type_info.to_string(),
        reason: reason.to_string(),
    }
}

/// Helper function to create `InvalidType` errors
#[must_use]
pub fn invalid_type(type_string: &str, reason: &str) -> ParseError {
    ParseError::InvalidType {
        type_string: type_string.to_string(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let parse_err: ParseError = io_err.into();
        assert!(matches!(parse_err, ParseError::IoError(..)));
        assert!(parse_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_parse_error_from_regex_error() {
        #[allow(clippy::invalid_regex)]
        let regex_result = regex::Regex::new(r"[invalid");
        assert!(regex_result.is_err());
        if let Err(regex_err) = regex_result {
            let parse_err: ParseError = regex_err.into();
            assert!(matches!(parse_err, ParseError::RegexError(..)));
        }
    }

    #[test]
    fn test_invalid_resource_name_helper() {
        let err = invalid_resource_name("bad-name", "[a-z_]+");
        assert!(matches!(err, ParseError::InvalidResourceName { .. }));
        let msg = err.to_string();
        assert!(msg.contains("bad-name"));
        assert!(msg.contains("[a-z_]+"));
    }

    #[test]
    fn test_invalid_value_helper() {
        let err = invalid_value("abc", "int32", "not a number");
        assert!(matches!(err, ParseError::InvalidValue { .. }));
        let msg = err.to_string();
        assert!(msg.contains("abc"));
        assert!(msg.contains("int32"));
        assert!(msg.contains("not a number"));
    }

    #[test]
    fn test_invalid_type_helper() {
        let err = invalid_type("badtype", "unknown type");
        assert!(matches!(err, ParseError::InvalidType { .. }));
        let msg = err.to_string();
        assert!(msg.contains("badtype"));
        assert!(msg.contains("unknown type"));
    }

    #[test]
    fn test_error_display_messages() {
        let err = ParseError::InvalidServiceSpecification {
            reason: "no separator".to_string(),
        };
        assert!(err.to_string().contains("no separator"));

        let err = ParseError::InvalidActionSpecification {
            reason: "wrong separators".to_string(),
        };
        assert!(err.to_string().contains("wrong separators"));

        let err = ParseError::LineParseError {
            line: 42,
            message: "syntax error".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("42"));
        assert!(msg.contains("syntax error"));

        let err = ParseError::InvalidArray {
            reason: "size must be > 0".to_string(),
        };
        assert!(err.to_string().contains("size must be > 0"));

        let err = ParseError::InvalidConstant {
            reason: "invalid name".to_string(),
        };
        assert!(err.to_string().contains("invalid name"));

        let err = ParseError::InvalidField {
            reason: "bad field".to_string(),
        };
        assert!(err.to_string().contains("bad field"));

        let err = ParseError::InvalidAnnotation {
            annotation: "@test".to_string(),
            reason: "unknown annotation".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("@test"));
        assert!(msg.contains("unknown annotation"));
    }
}
