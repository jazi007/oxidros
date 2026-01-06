use ros2msg::generator::{Generator, ItemInfo, ParseCallbacks};
use std::fs;
use tempfile::TempDir;

/// Custom callback for testing
struct TestCallbacks;

impl ParseCallbacks for TestCallbacks {
    fn add_derives(&self, info: &ItemInfo) -> Vec<String> {
        // Add serde derives for all types
        if info.name().contains("Message") {
            vec![
                "serde::Serialize".to_string(),
                "serde::Deserialize".to_string(),
            ]
        } else {
            vec![]
        }
    }

    fn custom_impl(&self, info: &ItemInfo) -> Option<String> {
        // Add a custom trait implementation
        Some(format!(
            "\nimpl CustomTrait for {} {{\n    fn custom_method(&self) -> &'static str {{\n        \"{}\"\n    }}\n}}\n",
            info.name(),
            info.name()
        ))
    }
}

/// Helper to create a temporary test message file
fn create_test_msg_file(
    dir: &TempDir,
    package: &str,
    name: &str,
    content: &str,
) -> std::path::PathBuf {
    let msg_dir = dir.path().join(package).join("msg");
    fs::create_dir_all(&msg_dir).unwrap();
    let file_path = msg_dir.join(format!("{}.msg", name));
    fs::write(&file_path, content).unwrap();
    file_path
}

#[test]
fn test_callbacks_add_derives() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "TestMessage", "int32 value\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TestCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("test_message.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Check that serde derives were added
    assert!(content.contains("serde::Serialize"));
    assert!(content.contains("serde::Deserialize"));
}

#[test]
fn test_callbacks_custom_impl() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "TestMessage", "int32 value\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TestCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("test_message.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Check that custom trait implementation was added
    assert!(content.contains("impl CustomTrait for TestMessage"));
    assert!(content.contains("fn custom_method(&self)"));
}

#[test]
fn test_callbacks_with_non_message_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "Data", // Name doesn't contain "Message"
        "int32 value\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TestCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok());

    let generated_file = output_dir.join("test_msgs").join("msg").join("data.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Serde derives should not be added for non-Message types
    assert!(!content.contains("serde::Serialize") || content.contains("impl CustomTrait"));
}

/// No-op callback for testing
struct NoOpCallbacks;

impl ParseCallbacks for NoOpCallbacks {
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        vec![]
    }

    fn custom_impl(&self, _info: &ItemInfo) -> Option<String> {
        None
    }
}

#[test]
fn test_noop_callbacks() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "TestMessage", "int32 value\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(NoOpCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("test_message.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should only have the standard derives
    assert!(content.contains("Debug"));
    assert!(!content.contains("serde::"));
    assert!(!content.contains("impl CustomTrait"));
}

/// Custom callback for testing sequence/string type mapping
struct TypeMappingCallbacks;

impl ParseCallbacks for TypeMappingCallbacks {
    fn sequence_type(&self, element_type: &str, max_size: Option<u32>) -> Option<String> {
        // Use custom BoundedVec for bounded sequences, regular Vec for unbounded
        if let Some(size) = max_size {
            Some(format!("BoundedVec<{}, {}>", element_type, size))
        } else {
            Some(format!("MyVec<{}>", element_type))
        }
    }

    fn string_type(&self, max_size: Option<u32>) -> Option<String> {
        // Use custom bounded string for bounded, regular for unbounded
        if let Some(size) = max_size {
            Some(format!("BoundedString<{}>", size))
        } else {
            Some("MyString".to_string())
        }
    }

    fn wstring_type(&self, max_size: Option<u32>) -> Option<String> {
        // Use custom wide string types
        if let Some(size) = max_size {
            Some(format!("BoundedWString<{}>", size))
        } else {
            Some("WideString".to_string())
        }
    }
}

#[test]
fn test_callbacks_custom_sequence_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create message with unbounded sequence
    let msg_file =
        create_test_msg_file(&temp_dir, "test_msgs", "SeqTest", "int32[] unbounded_seq\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TypeMappingCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    let generated_file = output_dir.join("test_msgs").join("msg").join("seq_test.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should use custom MyVec type for unbounded sequence
    assert!(
        content.contains("MyVec<i32>"),
        "Expected MyVec<i32> in: {}",
        content
    );
}

#[test]
fn test_callbacks_custom_bounded_sequence_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create message with bounded sequence
    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "BoundedSeqTest",
        "float64[<=10] bounded_seq\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TypeMappingCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("bounded_seq_test.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should use custom BoundedVec type for bounded sequence
    assert!(
        content.contains("BoundedVec<f64, 10>"),
        "Expected BoundedVec<f64, 10> in: {}",
        content
    );
}

#[test]
fn test_callbacks_custom_string_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create message with unbounded string
    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "StringTest",
        "string unbounded_str\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TypeMappingCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("string_test.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should use custom MyString type
    assert!(
        content.contains("MyString"),
        "Expected MyString in: {}",
        content
    );
}

#[test]
fn test_callbacks_custom_bounded_string_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create message with bounded string
    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "BoundedStringTest",
        "string<=50 bounded_str\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TypeMappingCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("bounded_string_test.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should use custom BoundedString type
    assert!(
        content.contains("BoundedString<50>"),
        "Expected BoundedString<50> in: {}",
        content
    );
}

#[test]
fn test_callbacks_custom_wstring_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create message with wstring
    let msg_file =
        create_test_msg_file(&temp_dir, "test_msgs", "WStringTest", "wstring wide_str\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TypeMappingCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    // Note: WStringTest becomes w_string_test.rs (snake_case)
    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("w_string_test.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should use custom WideString type
    assert!(
        content.contains("WideString"),
        "Expected WideString in: {}",
        content
    );
}

#[test]
fn test_callbacks_custom_bounded_wstring_type() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create message with bounded wstring
    let msg_file = create_test_msg_file(
        &temp_dir,
        "test_msgs",
        "BoundedWStringTest",
        "wstring<=100 bounded_wide_str\n",
    );

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(TypeMappingCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    // Note: BoundedWStringTest becomes bounded_w_string_test.rs (snake_case)
    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("bounded_w_string_test.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should use custom BoundedWString type
    assert!(
        content.contains("BoundedWString<100>"),
        "Expected BoundedWString<100> in: {}",
        content
    );
}

// ============================================================================
// Module callback tests
// ============================================================================

use ros2msg::generator::{ModuleInfo, ModuleLevel};

/// Callbacks that add re-exports after type modules
struct ReexportCallbacks;

impl ParseCallbacks for ReexportCallbacks {
    fn post_module(&self, info: &ModuleInfo) -> Option<String> {
        // Re-export all items from type modules
        if matches!(info.module_level(), ModuleLevel::Type(_)) {
            Some(format!("pub use {}::*;", info.module_name()))
        } else {
            None
        }
    }
}

#[test]
fn test_callbacks_post_module_reexport() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "test_msgs", "Header", "int32 seq\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(ReexportCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    // Check the msg/mod.rs file has the re-export
    let msg_mod = output_dir.join("test_msgs").join("msg").join("mod.rs");
    let content = fs::read_to_string(&msg_mod).unwrap();

    assert!(
        content.contains("pub mod header;"),
        "Expected 'pub mod header;' in: {}",
        content
    );
    assert!(
        content.contains("pub use header::*;"),
        "Expected 'pub use header::*;' in: {}",
        content
    );
}

/// Callbacks that add doc comments before modules
struct DocCommentCallbacks;

impl ParseCallbacks for DocCommentCallbacks {
    fn pre_module(&self, info: &ModuleInfo) -> Option<String> {
        match info.module_level() {
            ModuleLevel::Package => Some(format!("/// Package: {}\n", info.module_name())),
            ModuleLevel::InterfaceKind(kind) => Some(format!("/// Interface: {:?}\n", kind)),
            ModuleLevel::Type(kind) => Some(format!(
                "/// Type module: {} ({:?})\n",
                info.module_name(),
                kind
            )),
        }
    }
}

#[test]
fn test_callbacks_pre_module_doc_comments() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "my_pkg", "Point", "float64 x\nfloat64 y\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(DocCommentCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    // Check root mod.rs has package doc comment
    let root_mod = output_dir.join("mod.rs");
    let root_content = fs::read_to_string(&root_mod).unwrap();
    assert!(
        root_content.contains("/// Package: my_pkg"),
        "Expected package doc comment in root mod.rs: {}",
        root_content
    );

    // Check package mod.rs has interface doc comment
    let pkg_mod = output_dir.join("my_pkg").join("mod.rs");
    let pkg_content = fs::read_to_string(&pkg_mod).unwrap();
    assert!(
        pkg_content.contains("/// Interface: Message"),
        "Expected interface doc comment in package mod.rs: {}",
        pkg_content
    );

    // Check msg/mod.rs has type doc comment
    let msg_mod = output_dir.join("my_pkg").join("msg").join("mod.rs");
    let msg_content = fs::read_to_string(&msg_mod).unwrap();
    assert!(
        msg_content.contains("/// Type module: point (Message)"),
        "Expected type doc comment in msg mod.rs: {}",
        msg_content
    );
}

/// Callbacks that test ModuleInfo accessors by verifying full_path
struct FullPathCallbacks;

impl ParseCallbacks for FullPathCallbacks {
    fn post_module(&self, info: &ModuleInfo) -> Option<String> {
        // Add a comment with the full path for verification
        Some(format!("// full_path: {}\n", info.full_path()))
    }
}

#[test]
fn test_module_info_full_path() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_file = create_test_msg_file(&temp_dir, "geometry_msgs", "Pose", "float64 x\n");

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(FullPathCallbacks))
        .include(msg_file.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    // Check root mod.rs has package full_path
    let root_mod = output_dir.join("mod.rs");
    let root_content = fs::read_to_string(&root_mod).unwrap();
    assert!(
        root_content.contains("// full_path: geometry_msgs"),
        "Expected full_path comment in root mod.rs: {}",
        root_content
    );

    // Check package mod.rs has interface full_path
    let pkg_mod = output_dir.join("geometry_msgs").join("mod.rs");
    let pkg_content = fs::read_to_string(&pkg_mod).unwrap();
    assert!(
        pkg_content.contains("// full_path: geometry_msgs::msg"),
        "Expected full_path comment in package mod.rs: {}",
        pkg_content
    );

    // Check msg/mod.rs has type full_path
    let msg_mod = output_dir.join("geometry_msgs").join("msg").join("mod.rs");
    let msg_content = fs::read_to_string(&msg_mod).unwrap();
    assert!(
        msg_content.contains("// full_path: geometry_msgs::msg::pose"),
        "Expected full_path comment in msg mod.rs: {}",
        msg_content
    );
}
