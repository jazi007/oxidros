use ros2msg::generator::{FieldInfo, Generator, ParseCallbacks};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Callback that generates #[ros2(...)] attributes for testing
struct Ros2AttributeCallbacks;

impl ParseCallbacks for Ros2AttributeCallbacks {
    fn add_field_attributes(&self, field_info: &FieldInfo) -> Vec<String> {
        let mut attrs = Vec::new();
        let mut ros2_parts = Vec::new();

        // Add type override if present
        if let Some(type_override) = field_info.ros2_type_override() {
            ros2_parts.push(format!("ros2_type = \"{}\"", type_override));
        }

        // Add capacity if present
        if let Some(capacity) = field_info.capacity() {
            ros2_parts.push(format!("capacity = {}", capacity));
        }

        // Add default value if present
        if let Some(default_value) = field_info.default_value() {
            ros2_parts.push(format!("default = \"{}\"", default_value));
        }

        if !ros2_parts.is_empty() {
            attrs.push(format!("#[ros2({})]", ros2_parts.join(", ")));
        }

        attrs
    }
}

/// Helper to create a temporary test message file
fn create_test_msg_file(dir: &TempDir, package: &str, name: &str, content: &str) -> PathBuf {
    let msg_dir = dir.path().join(package).join("msg");
    fs::create_dir_all(&msg_dir).unwrap();
    let file_path = msg_dir.join(format!("{}.msg", name));
    fs::write(&file_path, content).unwrap();
    file_path
}

/// Helper to create a temporary test service file
fn create_test_srv_file(dir: &TempDir, package: &str, name: &str, content: &str) -> PathBuf {
    let srv_dir = dir.path().join(package).join("srv");
    fs::create_dir_all(&srv_dir).unwrap();
    let file_path = srv_dir.join(format!("{}.srv", name));
    fs::write(&file_path, content).unwrap();
    file_path
}

#[test]
fn test_generator_basic_message() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "SimpleMessage",
        "int32 value\nstring name\n",
    );

    let result = Generator::new()
        .header("// Test generated code")
        .derive_debug(true)
        .derive_clone(true)
        .derive_partialeq(true)
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    // Check generated file exists
    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("simple_message.rs");
    assert!(generated_file.exists(), "Generated file not found");

    // Check file content
    let content = fs::read_to_string(&generated_file).unwrap();
    assert!(content.contains("// Test generated code"));
    assert!(content.contains("pub struct SimpleMessage"));
    assert!(content.contains("pub value: i32"));
    assert!(content.contains("pub name:") && content.contains("String"));
    assert!(
        content.contains("Debug") && content.contains("Clone") && content.contains("PartialEq")
    );
}

#[test]
fn test_generator_service_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let srv_file = create_test_srv_file(
        &temp_dir,
        "test_msgs",
        "AddTwoInts",
        "int64 a\nint64 b\n---\nint64 sum\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .derive_clone(true)
        .include(srv_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    // Check generated service file
    let generated_file = output_dir
        .join("test_msgs")
        .join("srv")
        .join("add_two_ints.rs");
    assert!(generated_file.exists());

    let content = fs::read_to_string(&generated_file).unwrap();
    assert!(content.contains("AddTwoInts_Request") || content.contains("AddTwoIntsRequest"));
    assert!(content.contains("AddTwoInts_Response") || content.contains("AddTwoIntsResponse"));
    assert!(content.contains("pub a: i64"));
    assert!(content.contains("pub b: i64"));
    assert!(content.contains("pub sum: i64"));
}

#[test]
fn test_generator_with_all_derives() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file =
        create_test_msg_file(&temp_dir, "test_msgs", "FullDerives", "int32 x\nint32 y\n");

    let result = Generator::new()
        .derive_debug(true)
        .derive_clone(true)
        .derive_copy(true)
        .derive_default(true)
        .derive_partialeq(true)
        .derive_eq(true)
        .derive_partialord(true)
        .derive_ord(true)
        .derive_hash(true)
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("full_derives.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    assert!(content.contains("Debug"));
    assert!(content.contains("Clone"));
    assert!(content.contains("Copy"));
    assert!(content.contains("Default"));
    assert!(content.contains("PartialEq"));
    assert!(content.contains("Eq"));
    assert!(content.contains("PartialOrd"));
    assert!(content.contains("Ord"));
    assert!(content.contains("Hash"));
}

#[test]
fn test_generator_with_raw_lines() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "WithRawLines", "int32 value\n");

    let result = Generator::new()
        .derive_debug(true)
        .raw_line("#![allow(clippy::all)]")
        .raw_line("use std::fmt;")
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("with_raw_lines.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    assert!(content.contains("#![allow(clippy::all)]"));
    assert!(content.contains("use std::fmt;"));
}

#[test]
fn test_generator_module_structure() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create multiple messages
    let msg1 = create_test_msg_file(&temp_dir, "test_msgs", "Message1", "int32 a\n");
    let msg2 = create_test_msg_file(&temp_dir, "test_msgs", "Message2", "int32 b\n");
    let srv1 = create_test_srv_file(
        &temp_dir,
        "test_msgs",
        "Service1",
        "int32 req\n---\nint32 resp\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .include(msg1.to_str().unwrap())
        .include(msg2.to_str().unwrap())
        .include(srv1.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    // Check module structure
    let package_mod = output_dir.join("test_msgs").join("mod.rs");
    assert!(package_mod.exists());
    let package_mod_content = fs::read_to_string(&package_mod).unwrap();
    assert!(package_mod_content.contains("pub mod msg;"));
    assert!(package_mod_content.contains("pub mod srv;"));

    let msg_mod = output_dir.join("test_msgs").join("msg").join("mod.rs");
    assert!(msg_mod.exists());
    let msg_mod_content = fs::read_to_string(&msg_mod).unwrap();
    assert!(msg_mod_content.contains("pub mod message1;"));
    assert!(msg_mod_content.contains("pub mod message2;"));

    let srv_mod = output_dir.join("test_msgs").join("srv").join("mod.rs");
    assert!(srv_mod.exists());
    let srv_mod_content = fs::read_to_string(&srv_mod).unwrap();
    assert!(srv_mod_content.contains("pub mod service1;"));
}

#[test]
fn test_generator_with_constants() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "WithConstants",
        "int32 MAX_VALUE = 100\nstring DEFAULT_NAME = \"test\"\nint32 value\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("with_constants.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    assert!(content.contains("pub const MAX_VALUE: i32 = 100"));
    assert!(content.contains("pub const DEFAULT_NAME: &str = \"test\""));
}

#[test]
fn test_generator_emit_rerun_if_changed() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "TestMsg", "int32 value\n");

    // emit_rerun_if_changed is only relevant in build.rs context
    // but we can verify it doesn't break the API
    let result = Generator::new()
        .derive_debug(true)
        .emit_rerun_if_changed(true)
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());
}

#[test]
fn test_generator_multiple_packages() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg1 = create_test_msg_file(&temp_dir, "package_a", "MsgA", "int32 a\n");
    let msg2 = create_test_msg_file(&temp_dir, "package_b", "MsgB", "int32 b\n");

    let result = Generator::new()
        .derive_debug(true)
        .include(msg1.to_str().unwrap())
        .include(msg2.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    // Check both packages were generated
    assert!(
        output_dir
            .join("package_a")
            .join("msg")
            .join("msg_a.rs")
            .exists()
    );
    assert!(
        output_dir
            .join("package_b")
            .join("msg")
            .join("msg_b.rs")
            .exists()
    );

    // Check root mod.rs includes both packages
    let root_mod = output_dir.join("mod.rs");
    let root_content = fs::read_to_string(&root_mod).unwrap();
    assert!(root_content.contains("pub mod package_a;"));
    assert!(root_content.contains("pub mod package_b;"));
}

#[test]
fn test_generator_error_no_output_dir() {
    let temp_dir = TempDir::new().unwrap();
    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "Test", "int32 value\n");

    let result = Generator::new()
        .include(msg_file.to_str().unwrap())
        // Missing output_dir
        .generate();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Output directory is required")
    );
}

#[test]
fn test_generator_error_no_input_files() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let result = Generator::new()
        .output_dir(output_dir.to_str().unwrap())
        // Missing include files
        .generate();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No input files provided")
    );
}

#[test]
fn test_generator_with_arrays() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "WithArrays",
        "int32[5] fixed_array\nint32[] dynamic_array\nint32[<=10] bounded_array\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("with_arrays.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    assert!(content.contains("[i32; 5]")); // fixed array
    assert!(content.contains("Vec<i32>")); // dynamic and bounded arrays
}

#[test]
fn test_generator_ctypes_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "WithChar", "char c\n");

    let result = Generator::new()
        .derive_debug(true)
        .ctypes_prefix("libc")
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("with_char.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    assert!(content.contains("libc::c_char"));
}

#[test]
fn test_generator_bounded_string() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "BoundedString",
        "string<=255 name\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2AttributeCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("bounded_string.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should have ros2 capacity attribute for bounded string
    assert!(
        content.contains("#[ros2(capacity = 255)]"),
        "Missing ros2 capacity attribute. Generated content:\n{}",
        content
    );
    assert!(content.contains("pub name: ::std::string::String"));
}

#[test]
fn test_generator_bounded_wstring() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "BoundedWString",
        "wstring<=128 text\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2AttributeCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("bounded_w_string.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should have combined ros2 attribute with type and capacity
    assert!(
        content.contains("#[ros2(ros2_type = \"wstring\", capacity = 128)]"),
        "Missing combined ros2 attribute. Generated content:\n{}",
        content
    );
    assert!(content.contains("pub text: ::std::string::String"));
}

#[test]
fn test_generator_bounded_sequence() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "BoundedSequence",
        "int32[<=100] values\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2AttributeCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("bounded_sequence.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should have ros2 capacity attribute for bounded sequence
    assert!(
        content.contains("#[ros2(capacity = 100)]"),
        "Missing ros2 capacity attribute. Generated content:\n{}",
        content
    );
    assert!(content.contains("pub values: Vec<i32>"));
}

#[test]
fn test_generator_bounded_string_sequence() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "BoundedStringSequence",
        "string<=50[<=10] names\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2AttributeCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("bounded_string_sequence.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should have ros2 capacity attribute for the sequence (not the inner string capacity)
    // The sequence capacity is what matters for the field type
    assert!(
        content.contains("#[ros2(capacity = 10)]"),
        "Missing ros2 capacity attribute for sequence. Generated content:\n{}",
        content
    );
    assert!(content.contains("pub names: Vec<::std::string::String>"));
}
