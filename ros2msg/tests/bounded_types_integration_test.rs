/// Integration test for bounded types in real ROS2 messages
use ros2msg::generator::{FieldInfo, Generator, ParseCallbacks};
use std::fs;
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

#[test]
fn test_solid_primitive_bounded_sequence() {
    // Test shape_msgs/msg/SolidPrimitive which has: float64[<=3] dimensions
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    // Create the message content matching the real ROS2 message
    let msg_content = r#"# Defines box, sphere, cylinder, cone and prism.
uint8 BOX=1
uint8 SPHERE=2
uint8 CYLINDER=3
uint8 CONE=4
uint8 PRISM=5

uint8 type
float64[<=3] dimensions
"#;

    let msg_dir = temp_dir.path().join("shape_msgs").join("msg");
    fs::create_dir_all(&msg_dir).unwrap();
    let file_path = msg_dir.join("SolidPrimitive.msg");
    fs::write(&file_path, msg_content).unwrap();

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2AttributeCallbacks))
        .include(file_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("shape_msgs")
        .join("msg")
        .join("solid_primitive.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Should have ros2 capacity attribute for bounded sequence
    assert!(
        content.contains("#[ros2(capacity = 3)]"),
        "Missing ros2 capacity attribute. Generated content:\n{}",
        content
    );
    assert!(content.contains("pub dimensions: Vec<f64>"));
}

#[test]
fn test_parameter_descriptor_bounded_nested_sequence() {
    // Test rcl_interfaces/msg/ParameterDescriptor which has:
    // FloatingPointRange[<=1] floating_point_range
    // IntegerRange[<=1] integer_range
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let floating_point_range_content = "float64 from_value\nfloat64 to_value\nfloat64 step\n";
    let integer_range_content = "int64 from_value\nint64 to_value\nint64 step\n";

    // Create dependency messages
    let msg_dir = temp_dir.path().join("rcl_interfaces").join("msg");
    fs::create_dir_all(&msg_dir).unwrap();

    fs::write(
        msg_dir.join("FloatingPointRange.msg"),
        floating_point_range_content,
    )
    .unwrap();

    fs::write(msg_dir.join("IntegerRange.msg"), integer_range_content).unwrap();

    let msg_content = r#"string name
uint8 type
string description
string additional_constraints
bool read_only false
bool dynamic_typing false
FloatingPointRange[<=1] floating_point_range
IntegerRange[<=1] integer_range
"#;

    let file_path = msg_dir.join("ParameterDescriptor.msg");
    fs::write(&file_path, msg_content).unwrap();

    // Include all three files
    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2AttributeCallbacks))
        .include(msg_dir.join("FloatingPointRange.msg").to_str().unwrap())
        .include(msg_dir.join("IntegerRange.msg").to_str().unwrap())
        .include(file_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("rcl_interfaces")
        .join("msg")
        .join("parameter_descriptor.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Check for bounded sequence attributes
    let capacity_count = content.matches("#[ros2(capacity = 1)]").count();
    assert!(
        capacity_count >= 2,
        "Expected at least 2 ros2 capacity attributes (found {}). Generated content:\n{}",
        capacity_count,
        content
    );

    assert!(
        content.contains("pub floating_point_range: Vec<"),
        "Missing floating_point_range field"
    );
    assert!(
        content.contains("pub integer_range: Vec<"),
        "Missing integer_range field"
    );
}

#[test]
fn test_parameter_event_descriptors_unbounded_nested_sequence() {
    // Test rcl_interfaces/msg/ParameterEventDescriptors which has:
    // ParameterDescriptor[] new_parameters
    // ParameterDescriptor[] changed_parameters
    // ParameterDescriptor[] deleted_parameters
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let parameter_descriptor_content = "string name\nuint8 type\n";

    // Create dependency message
    let msg_dir = temp_dir.path().join("rcl_interfaces").join("msg");
    fs::create_dir_all(&msg_dir).unwrap();

    fs::write(
        msg_dir.join("ParameterDescriptor.msg"),
        parameter_descriptor_content,
    )
    .unwrap();

    let msg_content = r#"ParameterDescriptor[] new_parameters
ParameterDescriptor[] changed_parameters
ParameterDescriptor[] deleted_parameters
"#;

    let file_path = msg_dir.join("ParameterEventDescriptors.msg");
    fs::write(&file_path, msg_content).unwrap();

    let result = Generator::new()
        .derive_debug(true)
        .include(msg_dir.join("ParameterDescriptor.msg").to_str().unwrap())
        .include(file_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("rcl_interfaces")
        .join("msg")
        .join("parameter_event_descriptors.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // These should NOT have ros2 capacity attributes (unbounded sequences)
    assert!(
        !content.contains("#[ros2(capacity"),
        "Unbounded sequences should not have ros2 capacity attribute. Generated content:\n{}",
        content
    );

    assert!(
        content.contains("pub new_parameters: Vec<"),
        "Missing new_parameters field"
    );
    assert!(
        content.contains("pub changed_parameters: Vec<"),
        "Missing changed_parameters field"
    );
    assert!(
        content.contains("pub deleted_parameters: Vec<"),
        "Missing deleted_parameters field"
    );
}

#[test]
fn test_bounded_vs_unbounded_sequences() {
    // Test that we correctly distinguish bounded vs unbounded sequences
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("generated");

    let msg_content = r#"int32[] unbounded_seq
int32[<=10] bounded_seq
int32[5] fixed_array
"#;

    let msg_dir = temp_dir.path().join("test_msgs").join("msg");
    fs::create_dir_all(&msg_dir).unwrap();
    let file_path = msg_dir.join("SequenceTest.msg");
    fs::write(&file_path, msg_content).unwrap();

    let result = Generator::new()
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2AttributeCallbacks))
        .include(file_path.to_str().unwrap())
        .output_dir(output_dir.to_str().unwrap())
        .generate();

    assert!(result.is_ok(), "Failed to generate: {:?}", result.err());

    let generated_file = output_dir
        .join("test_msgs")
        .join("msg")
        .join("sequence_test.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    // Unbounded sequence should NOT have capacity
    let unbounded_idx = content.find("pub unbounded_seq:").unwrap();
    let bounded_idx = content.find("pub bounded_seq:").unwrap();

    let unbounded_section = &content[unbounded_idx.saturating_sub(100)..unbounded_idx];
    let bounded_section = &content[bounded_idx.saturating_sub(100)..bounded_idx];

    assert!(
        !unbounded_section.contains("ros2(capacity"),
        "Unbounded sequence should not have ros2 capacity attribute. Section:\n{}",
        unbounded_section
    );

    assert!(
        bounded_section.contains("#[ros2(capacity = 10)]"),
        "Bounded sequence should have ros2 capacity attribute. Section:\n{}",
        bounded_section
    );

    // Fixed array should NOT have capacity (uses Rust array type)
    assert!(content.contains("pub fixed_array: [i32; 5]"));
}
