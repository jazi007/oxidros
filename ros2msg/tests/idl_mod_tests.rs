// Tests for idl::mod to improve coverage

use ros2msg::idl::*;
use std::path::PathBuf;

#[test]
fn test_parse_idl_string() {
    let idl = "module example { struct Point { long x; long y; }; };";
    let result = parse_idl_string(idl);
    assert!(result.is_ok());
}

#[test]
fn test_parse_idl_string_empty() {
    let result = parse_idl_string("");
    assert!(result.is_ok());
}

#[test]
fn test_parse_idl_string_invalid() {
    let idl = "this is not valid idl syntax {{{{";
    let result = parse_idl_string(idl);
    assert!(result.is_err());
}

#[test]
fn test_parse_idl_string_with_path() {
    let idl = "module test { struct Data { long value; }; };";
    let result = parse_idl_string_with_path(idl, PathBuf::from("."), PathBuf::from("test.idl"));
    assert!(result.is_ok());
}

#[test]
fn test_parse_idl_string_with_path_custom_dirs() {
    let idl = "module pkg { struct Type { short num; }; };";
    let result = parse_idl_string_with_path(
        idl,
        PathBuf::from("/some/base/path"),
        PathBuf::from("subdir/file.idl"),
    );
    assert!(result.is_ok());
}

#[test]
fn test_parse_idl_file() {
    let temp_dir = std::env::temp_dir();
    let idl_path = temp_dir.join("test_coverage.idl");
    std::fs::write(&idl_path, "module test { struct Data { long value; }; };").unwrap();

    let result = parse_idl_file(&idl_path);
    assert!(result.is_ok());
    std::fs::remove_file(&idl_path).ok();
}

#[test]
fn test_parse_idl_file_not_found() {
    let result = parse_idl_file(PathBuf::from("/nonexistent/file.idl").as_path());
    assert!(result.is_err());
}

#[test]
fn test_parse_idl_file_invalid_content() {
    let temp_dir = std::env::temp_dir();
    let idl_path = temp_dir.join("invalid.idl");
    std::fs::write(&idl_path, "invalid idl content }{}{").unwrap();

    let result = parse_idl_file(&idl_path);
    assert!(result.is_err());
    std::fs::remove_file(&idl_path).ok();
}
