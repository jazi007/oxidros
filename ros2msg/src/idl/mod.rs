//! ROS2 IDL Parser Module
//!
//! This module provides comprehensive parsing for the full ROS2 IDL specification,
//! including complex types, modules, annotations, and advanced language features.

/// Error handling for IDL parsing
pub mod errors;
/// Grammar definition and parser implementation
pub mod grammar;
/// Full IDL parser implementation using pest
pub mod parser_pest;
/// Parser tests
#[cfg(test)]
mod parser_tests;
/// Core IDL AST types and definitions
pub mod types;
/// Value handling and constant evaluation
pub mod values;

// Re-export commonly used types
pub use errors::{IdlError, IdlResult};
pub use types::*;

/// Parse IDL content from a string with default paths
///
/// This convenience function uses default paths (base: ".", file: "`<string>`").
/// For more control over file paths, use `parse_idl_string_with_path` or
/// `grammar::parse_idl_string` directly.
///
/// # Errors
///
/// Returns an error if the IDL content cannot be parsed
pub fn parse_idl_string(content: &str) -> IdlResult<IdlFile> {
    grammar::parse_idl_string(
        content,
        std::path::PathBuf::from("."),
        std::path::PathBuf::from("<string>"),
    )
}

/// Parse IDL content from a string with explicit paths
///
/// This function allows you to specify the base path and relative file path
/// for better error reporting and include resolution.
///
/// # Arguments
///
/// * `content` - The IDL content to parse
/// * `base_path` - The base directory path
/// * `file_path` - The relative file path (used for error reporting and includes)
///
/// # Errors
///
/// Returns an error if the IDL content cannot be parsed
pub fn parse_idl_string_with_path<P: Into<std::path::PathBuf>>(
    content: &str,
    base_path: P,
    file_path: P,
) -> IdlResult<IdlFile> {
    grammar::parse_idl_string(content, base_path.into(), file_path.into())
}

/// Parse IDL file from a file path
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed
pub fn parse_idl_file<P: AsRef<std::path::Path>>(file_path: P) -> IdlResult<IdlFile> {
    let path = file_path.as_ref();
    let base_path = path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.idl")
        .to_string();

    let locator = IdlLocator::new(base_path, std::path::PathBuf::from(file_name));
    grammar::parse_idl_file(&locator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_idl_string() {
        let idl = "module test { struct Point { long x; long y; }; };";
        let result = parse_idl_string(idl);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_idl_string_empty() {
        let result = parse_idl_string("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_idl_string_with_comments() {
        let idl = "// Comment\n/* Block */\nmodule test { };";
        let result = parse_idl_string(idl);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_idl_string_with_path() {
        let idl = "module test { struct Data { long value; }; };";
        let result = parse_idl_string_with_path(
            idl,
            std::path::PathBuf::from("/tmp"),
            std::path::PathBuf::from("test.idl"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_idl_string_invalid() {
        let idl = "module test { struct Point { long x long y; }; };";
        let result = parse_idl_string(idl);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_idl_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_parse.idl");
        std::fs::write(&file_path, "module test { struct Point { long x; }; };").unwrap();

        let result = parse_idl_file(&file_path);
        assert!(result.is_ok());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_parse_idl_file_not_found() {
        let result = parse_idl_file("/nonexistent/path/file.idl");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_idl_file_invalid_content() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_invalid.idl");
        std::fs::write(&file_path, "invalid idl syntax {{{").unwrap();

        let result = parse_idl_file(&file_path);
        assert!(result.is_err());

        std::fs::remove_file(&file_path).ok();
    }
}
