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

    // Create an IDL file with actual IDL 'char' type (not .msg 'char' which is uint8)
    // IDL 'char' is a signed 8-bit character type that maps to c_char
    let idl_dir = temp_dir.path().join("test_msgs").join("msg");
    fs::create_dir_all(&idl_dir).unwrap();
    let idl_file = idl_dir.join("WithChar.idl");
    fs::write(
        &idl_file,
        r#"module test_msgs {
  module msg {
    struct WithChar {
      char c;
    };
  };
};
"#,
    )
    .unwrap();

    let result = Generator::new()
        .derive_debug(true)
        .ctypes_prefix("libc")
        .include(idl_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("with_char.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    assert!(
        content.contains("libc::c_char"),
        "Expected libc::c_char but got:\n{}",
        content
    );
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

#[test]
fn test_generator_list_nodes_service() {
    // Test the ListNodes.srv pattern: empty request, response with arrays
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let srv_file = create_test_srv_file(
        &temp_dir,
        "composition_interfaces",
        "ListNodes",
        "---\nstring[] full_node_names\nuint64[] unique_ids\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .include(srv_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("composition_interfaces")
        .join("srv")
        .join("list_nodes.rs");
    assert!(generated_file.exists(), "Generated file not found");

    let content = fs::read_to_string(&generated_file).unwrap();
    println!("=== Generated Rust Code ===\n{}\n=== End ===", content);

    // Must have both request and response structs
    assert!(
        content.contains("pub struct ListNodes_Request"),
        "Missing ListNodes_Request struct. Content:\n{}",
        content
    );
    assert!(
        content.contains("pub struct ListNodes_Response"),
        "Missing ListNodes_Response struct. Content:\n{}",
        content
    );

    // Request should have dummy member (empty struct)
    assert!(
        content.contains("structure_needs_at_least_one_member"),
        "Empty request should have dummy member. Content:\n{}",
        content
    );

    // Response should have the actual fields
    assert!(
        content.contains("full_node_names"),
        "Missing full_node_names field. Content:\n{}",
        content
    );
    assert!(
        content.contains("unique_ids"),
        "Missing unique_ids field. Content:\n{}",
        content
    );
}

#[test]
fn test_generator_real_list_nodes_service() {
    // Test with actual ROS2 ListNodes.srv from filesystem
    use std::path::Path;
    let srv_path = Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/ListNodes.srv");
    if !srv_path.exists() {
        eprintln!("Skipping test - ListNodes.srv not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let result = Generator::new()
        .derive_debug(true)
        .include(srv_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("composition_interfaces")
        .join("srv")
        .join("list_nodes.rs");
    assert!(
        generated_file.exists(),
        "Generated file not found at {:?}",
        generated_file
    );

    let content = fs::read_to_string(&generated_file).unwrap();
    println!("=== Generated from real file ===\n{}\n=== End ===", content);

    // Must have both request and response structs
    assert!(
        content.contains("pub struct ListNodes_Request"),
        "Missing ListNodes_Request struct"
    );
    assert!(
        content.contains("pub struct ListNodes_Response"),
        "Missing ListNodes_Response struct"
    );
}

#[test]
fn test_generator_multiple_files_same_package() {
    // Test with multiple .srv files from same package to check for file overwriting
    use std::path::Path;
    let list_nodes_path =
        Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/ListNodes.srv");
    let load_node_path = Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/LoadNode.srv");
    let unload_node_path =
        Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/UnloadNode.srv");

    if !list_nodes_path.exists() {
        eprintln!("Skipping test - files not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let ament_paths = vec![PathBuf::from("/opt/ros/jazzy/share")];

    // Include all three services like oxidros-build does
    let result = Generator::new()
        .header("// Auto-generated")
        .derive_debug(true)
        .include(list_nodes_path.to_str().unwrap())
        .include(load_node_path.to_str().unwrap())
        .include(unload_node_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .allowlist_recursively(true)
        .package_search_paths(ament_paths)
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    // Check ListNodes
    let list_nodes_file = output_dir
        .join("composition_interfaces")
        .join("srv")
        .join("list_nodes.rs");
    assert!(list_nodes_file.exists(), "ListNodes file not found");

    let list_nodes_content = fs::read_to_string(&list_nodes_file).unwrap();
    println!("=== ListNodes ===\n{}\n", list_nodes_content);
    assert!(
        list_nodes_content.contains("pub struct ListNodes_Request"),
        "Missing ListNodes_Request"
    );
    assert!(
        list_nodes_content.contains("pub struct ListNodes_Response"),
        "Missing ListNodes_Response"
    );

    // Check LoadNode
    let load_node_file = output_dir
        .join("composition_interfaces")
        .join("srv")
        .join("load_node.rs");
    assert!(load_node_file.exists(), "LoadNode file not found");

    let load_node_content = fs::read_to_string(&load_node_file).unwrap();
    println!("=== LoadNode ===\n{}\n", load_node_content);
    assert!(
        load_node_content.contains("pub struct LoadNode_Request"),
        "Missing LoadNode_Request"
    );
    assert!(
        load_node_content.contains("pub struct LoadNode_Response"),
        "Missing LoadNode_Response"
    );
}

/// Callback that generates ros2 attributes similar to oxidros-build's RosCallbacks
struct FullRos2Callbacks;

impl ParseCallbacks for FullRos2Callbacks {
    fn add_attributes(&self, info: &ros2msg::generator::ItemInfo) -> Vec<String> {
        let package = info.package();
        let interface_type = match info.interface_kind() {
            ros2msg::generator::InterfaceKind::Message => "msg",
            ros2msg::generator::InterfaceKind::Service => "srv",
            ros2msg::generator::InterfaceKind::Action => "action",
        };
        vec![
            format!("#[ros2(package = \"{}\", interface_type = \"{}\")]", package, interface_type),
            "#[cfg_attr(not(feature = \"rcl\"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]".to_string(),
            "#[cfg_attr(not(feature = \"rcl\"), serde(crate = \"ros2_types::serde\"))]".to_string(),
        ]
    }

    fn add_derives(&self, _info: &ros2msg::generator::ItemInfo) -> Vec<String> {
        vec![
            "ros2_types::Ros2Msg".to_string(),
            "ros2_types::TypeDescription".to_string(),
        ]
    }
}

#[test]
fn test_generator_with_full_ros2_callbacks() {
    // Test with callbacks similar to oxidros-build's RosCallbacks
    use std::path::Path;
    let srv_path = Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/ListNodes.srv");
    if !srv_path.exists() {
        eprintln!("Skipping test - ListNodes.srv not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");
    let ament_paths = vec![PathBuf::from("/opt/ros/jazzy/share")];

    let result = Generator::new()
        .header("// Auto-generated")
        .derive_debug(true)
        .parse_callbacks(Box::new(FullRos2Callbacks))
        .include(srv_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .emit_rerun_if_changed(true)
        .allowlist_recursively(true)
        .package_search_paths(ament_paths)
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("composition_interfaces")
        .join("srv")
        .join("list_nodes.rs");
    assert!(generated_file.exists(), "Generated file not found");

    let content = fs::read_to_string(&generated_file).unwrap();
    println!("=== With FullRos2Callbacks ===\n{}\n=== End ===", content);

    // Must have both request and response structs
    assert!(
        content.contains("pub struct ListNodes_Request"),
        "Missing ListNodes_Request. Content:\n{}",
        content
    );
    assert!(
        content.contains("pub struct ListNodes_Response"),
        "Missing ListNodes_Response. Content:\n{}",
        content
    );
}

#[test]
fn test_generator_all_composition_services_with_callbacks() {
    // Test with ALL composition_interfaces services to find file overwriting issues
    use std::path::Path;
    let list_nodes_path =
        Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/ListNodes.srv");
    let load_node_path = Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/LoadNode.srv");
    let unload_node_path =
        Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/UnloadNode.srv");

    if !list_nodes_path.exists() {
        eprintln!("Skipping test - files not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");
    let ament_paths = vec![PathBuf::from("/opt/ros/jazzy/share")];

    let result = Generator::new()
        .header("// Auto-generated")
        .derive_debug(true)
        .parse_callbacks(Box::new(FullRos2Callbacks))
        .include(list_nodes_path.to_str().unwrap())
        .include(load_node_path.to_str().unwrap())
        .include(unload_node_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .emit_rerun_if_changed(true)
        .allowlist_recursively(true)
        .package_search_paths(ament_paths)
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    // Check each service file
    for (name, req_name, resp_name) in [
        ("list_nodes.rs", "ListNodes_Request", "ListNodes_Response"),
        ("load_node.rs", "LoadNode_Request", "LoadNode_Response"),
        (
            "unload_node.rs",
            "UnloadNode_Request",
            "UnloadNode_Response",
        ),
    ] {
        let file_path = output_dir
            .join("composition_interfaces")
            .join("srv")
            .join(name);
        assert!(file_path.exists(), "File not found: {}", name);

        let content = fs::read_to_string(&file_path).unwrap();
        println!("=== {} ===\n{}\n", name, content);

        assert!(
            content.contains(&format!("pub struct {}", req_name)),
            "Missing {} in {}. Content:\n{}",
            req_name,
            name,
            content
        );
        assert!(
            content.contains(&format!("pub struct {}", resp_name)),
            "Missing {} in {}. Content:\n{}",
            resp_name,
            name,
            content
        );
    }
}

#[test]
fn test_generator_with_allowlist_recursively() {
    // Test with allowlist_recursively enabled, simulating how oxidros-build works
    use std::path::Path;
    let srv_path = Path::new("/opt/ros/jazzy/share/composition_interfaces/srv/ListNodes.srv");
    if !srv_path.exists() {
        eprintln!("Skipping test - ListNodes.srv not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let ament_paths = vec![PathBuf::from("/opt/ros/jazzy/share")];

    let result = Generator::new()
        .header("// Auto-generated")
        .derive_debug(true)
        .include(srv_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .allowlist_recursively(true)
        .package_search_paths(ament_paths)
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("composition_interfaces")
        .join("srv")
        .join("list_nodes.rs");
    assert!(
        generated_file.exists(),
        "Generated file not found at {:?}",
        generated_file
    );

    let content = fs::read_to_string(&generated_file).unwrap();
    println!(
        "=== With allowlist_recursively ===\n{}\n=== End ===",
        content
    );

    // Must have both request and response structs
    assert!(
        content.contains("pub struct ListNodes_Request"),
        "Missing ListNodes_Request struct"
    );
    assert!(
        content.contains("pub struct ListNodes_Response"),
        "Missing ListNodes_Response struct"
    );
}

#[test]
fn test_generator_idl_parameter_value() {
    // Test generating from ParameterValue.idl file
    use std::path::Path;
    let idl_path = Path::new("/opt/ros/jazzy/share/rcl_interfaces/msg/ParameterValue.idl");
    if !idl_path.exists() {
        eprintln!("Skipping test - ParameterValue.idl not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let result = Generator::new()
        .derive_debug(true)
        .include(idl_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("rcl_interfaces")
        .join("msg")
        .join("parameter_value.rs");
    assert!(
        generated_file.exists(),
        "Generated file not found at {:?}",
        generated_file
    );

    let content = fs::read_to_string(&generated_file).unwrap();
    println!(
        "=== Generated from ParameterValue.idl ===\n{}\n=== End ===",
        content
    );

    // Must have the ParameterValue struct
    assert!(
        content.contains("pub struct ParameterValue"),
        "Missing ParameterValue struct. Content:\n{}",
        content
    );
}
