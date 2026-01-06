/// Comprehensive integration tests for the ROS2 message parser
use ros2msg::*;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_message_parsing() {
        let content = r#"
# This is a comprehensive test message
# It includes various field types and constants

# Constants section
int32 MAX_SIZE=1000
string DEFAULT_NAME="robot"
float64 PI=3.14159
bool DEBUG_MODE=true

# Header with timestamp
std_msgs/Header header

# Basic primitive types
bool enabled
int8 small_int
uint8 byte_value
int16 medium_int
uint16 unsigned_medium
int32 large_int
uint32 unsigned_large
int64 huge_int
uint64 unsigned_huge
float32 precision
float64 high_precision
string name
wstring wide_name

# Bounded strings
string<=50 limited_name
wstring<=100 limited_wide_name

# Arrays
int32[] dynamic_numbers
int32[5] fixed_numbers
int32[<=10] bounded_numbers
string[] dynamic_names
string[3] fixed_names
string[<=5] bounded_names

# Complex types
geometry_msgs/Point position
geometry_msgs/Point[] waypoints
geometry_msgs/Point[<=100] path

# Fields with default values
string description "Default description"
int32 timeout 30
bool active true
float64 rate 10.0

# Optional fields  
@optional
string optional_field
"#;

        let spec = parse_message_string("robot_msgs", "ComplexMessage", content).unwrap();

        // Verify basic info
        assert_eq!(spec.pkg_name, "robot_msgs");
        assert_eq!(spec.msg_name, "ComplexMessage");

        // Verify constants
        assert_eq!(spec.constants.len(), 4);
        let max_size = spec.get_constant("MAX_SIZE").unwrap();
        assert_eq!(max_size.value, PrimitiveValue::Int32(1000));

        let default_name = spec.get_constant("DEFAULT_NAME").unwrap();
        assert_eq!(
            default_name.value,
            PrimitiveValue::String("robot".to_string())
        );

        // Verify fields count
        assert!(spec.fields.len() > 20);

        // Test specific field types
        let header_field = spec.get_field("header").unwrap();
        assert!(!header_field.field_type.is_primitive_type());
        assert_eq!(
            header_field.field_type.base_type.pkg_name,
            Some("std_msgs".to_string())
        );

        let dynamic_numbers = spec.get_field("dynamic_numbers").unwrap();
        assert!(dynamic_numbers.field_type.is_dynamic_array());

        let fixed_numbers = spec.get_field("fixed_numbers").unwrap();
        assert_eq!(fixed_numbers.field_type.array_size, Some(5));
        assert!(!fixed_numbers.field_type.is_upper_bound);

        let bounded_numbers = spec.get_field("bounded_numbers").unwrap();
        assert!(bounded_numbers.field_type.is_bounded_array());
        assert_eq!(bounded_numbers.field_type.array_size, Some(10));

        // Test default values
        let description = spec.get_field("description").unwrap();
        assert!(description.default_value.is_some());

        // Test optional annotation
        let optional_field = spec.get_field("optional_field").unwrap();
        assert!(optional_field.annotations.contains_key("optional"));
    }

    #[test]
    fn test_complete_service_parsing() {
        let content = r#"
# Service for robot navigation
# Request: target pose and options
# Response: success status and path

# Request constants
float64 MAX_DISTANCE=100.0
float64 MIN_DISTANCE=0.1

# Request fields
geometry_msgs/PoseStamped target_pose
float64 tolerance 0.1
bool use_planner true
string planner_id "default"

---

# Response constants  
uint8 SUCCESS=0
uint8 FAILURE=1
uint8 INVALID_GOAL=2

# Response fields
uint8 result
string message
geometry_msgs/Path path
float64 distance
duration planning_time
"#;

        let spec = parse_service_string("navigation_msgs", "NavigateToGoal", content).unwrap();

        // Verify basic info
        assert_eq!(spec.pkg_name, "navigation_msgs");
        assert_eq!(spec.srv_name, "NavigateToGoal");

        // Verify request
        assert_eq!(spec.request.constants.len(), 2);
        assert_eq!(spec.request.fields.len(), 4);

        let max_distance = spec.get_request_constant("MAX_DISTANCE").unwrap();
        assert_eq!(max_distance.value, PrimitiveValue::Float64(100.0));

        let target_pose = spec.get_request_field("target_pose").unwrap();
        assert!(!target_pose.field_type.is_primitive_type());

        // Verify response
        assert_eq!(spec.response.constants.len(), 3);
        assert_eq!(spec.response.fields.len(), 5);

        let success_const = spec.get_response_constant("SUCCESS").unwrap();
        assert_eq!(success_const.value, PrimitiveValue::UInt8(0));

        // Test service event message creation
        let event_msg = create_service_event_message(
            "navigation_msgs",
            "NavigateToGoal",
            &spec.request,
            &spec.response,
        )
        .unwrap();

        assert_eq!(event_msg.msg_name, "NavigateToGoal_Event");
        assert_eq!(event_msg.fields.len(), 3); // info, request[], response[]
    }

    #[test]
    fn test_complete_action_parsing() {
        let content = r#"
# Fibonacci action
# Goal: compute fibonacci sequence up to order n
# Result: complete sequence
# Feedback: partial sequence updates

# Goal constants
int32 MIN_ORDER=0
int32 MAX_ORDER=1000

# Goal fields
int32 order

---

# Result constants
uint8 SUCCESS=0
uint8 INVALID_ORDER=1

# Result fields
uint8 status
int32[] sequence
duration computation_time

---

# Feedback fields
int32[] partial_sequence
int32 current_step
float32 progress
"#;

        let spec = parse_action_string("math_msgs", "Fibonacci", content).unwrap();

        // Verify basic info
        assert_eq!(spec.pkg_name, "math_msgs");
        assert_eq!(spec.action_name, "Fibonacci");

        // Verify goal
        assert_eq!(spec.goal.constants.len(), 2);
        assert_eq!(spec.goal.fields.len(), 1);

        let min_order = spec.get_goal_constant("MIN_ORDER").unwrap();
        assert_eq!(min_order.value, PrimitiveValue::Int32(0));

        // Verify result
        assert_eq!(spec.result.constants.len(), 2);
        assert_eq!(spec.result.fields.len(), 3);

        let sequence_field = spec.get_result_field("sequence").unwrap();
        assert!(sequence_field.field_type.is_dynamic_array());

        // Verify feedback
        assert_eq!(spec.feedback.fields.len(), 3);

        let progress_field = spec.get_feedback_field("progress").unwrap();
        assert_eq!(progress_field.field_type.base_type.type_name, "float32");

        // Verify derived services
        assert_eq!(spec.goal_service.srv_name, "Fibonacci_SendGoal");
        assert_eq!(spec.result_service.srv_name, "Fibonacci_GetResult");

        // Test feedback message creation
        let feedback_msg =
            create_feedback_message("math_msgs", "Fibonacci", &spec.feedback).unwrap();

        assert_eq!(feedback_msg.msg_name, "Fibonacci_FeedbackMessage");
        assert_eq!(feedback_msg.fields.len(), 2); // goal_id + feedback
    }

    #[test]
    fn test_error_cases() {
        // Invalid package name
        let result = parse_message_string("Invalid-Package", "TestMsg", "int32 x");
        assert!(result.is_err());

        // Invalid message name
        let result = parse_message_string("test_msgs", "invalidMessage", "int32 x");
        assert!(result.is_err());

        // Invalid field name
        let result = parse_message_string("test_msgs", "TestMsg", "int32 Invalid-Field");
        assert!(result.is_err());

        // Invalid constant name
        let result = parse_message_string("test_msgs", "TestMsg", "int32 invalid_constant=5");
        assert!(result.is_err());

        // Invalid type
        let result = parse_message_string("test_msgs", "TestMsg", "invalid_type x");
        assert!(result.is_err());

        // Invalid array syntax
        let result = parse_message_string("test_msgs", "TestMsg", "int32[ x");
        assert!(result.is_err());

        // Invalid service (no separator)
        let result = parse_service_string("test_msgs", "TestSrv", "int32 request\nint32 response");
        assert!(result.is_err());

        // Invalid action (wrong number of separators)
        let result =
            parse_action_string("test_msgs", "TestAction", "int32 goal\n---\nint32 result");
        assert!(result.is_err());
    }

    #[test]
    fn test_primitive_value_parsing() {
        // Boolean values
        assert_eq!(
            parse_primitive_value_string("bool", "true").unwrap(),
            PrimitiveValue::Bool(true)
        );
        assert_eq!(
            parse_primitive_value_string("bool", "false").unwrap(),
            PrimitiveValue::Bool(false)
        );
        assert_eq!(
            parse_primitive_value_string("bool", "1").unwrap(),
            PrimitiveValue::Bool(true)
        );
        assert_eq!(
            parse_primitive_value_string("bool", "0").unwrap(),
            PrimitiveValue::Bool(false)
        );

        // Integer values
        assert_eq!(
            parse_primitive_value_string("int32", "42").unwrap(),
            PrimitiveValue::Int32(42)
        );
        assert_eq!(
            parse_primitive_value_string("int32", "-42").unwrap(),
            PrimitiveValue::Int32(-42)
        );
        assert_eq!(
            parse_primitive_value_string("uint32", "42").unwrap(),
            PrimitiveValue::UInt32(42)
        );

        // Hex values
        assert_eq!(
            parse_primitive_value_string("int32", "0xFF").unwrap(),
            PrimitiveValue::Int32(255)
        );
        assert_eq!(
            parse_primitive_value_string("int32", "0x10").unwrap(),
            PrimitiveValue::Int32(16)
        );

        // Float values
        assert_eq!(
            parse_primitive_value_string("float64", "3.15").unwrap(),
            PrimitiveValue::Float64(3.15)
        );
        assert_eq!(
            parse_primitive_value_string("float32", "2.5").unwrap(),
            PrimitiveValue::Float32(2.5)
        );

        // String values
        assert_eq!(
            parse_primitive_value_string("string", "\"hello\"").unwrap(),
            PrimitiveValue::String("hello".to_string())
        );
        assert_eq!(
            parse_primitive_value_string("string", "'world'").unwrap(),
            PrimitiveValue::String("world".to_string())
        );
        assert_eq!(
            parse_primitive_value_string("string", "unquoted").unwrap(),
            PrimitiveValue::String("unquoted".to_string())
        );
    }

    #[test]
    fn test_array_value_parsing() {
        let content = r#"
int32[] numbers [1, 2, 3, 4, 5]
string[] names ["alice", "bob", "charlie"]
bool[] flags [true, false, true]
float64[] values [1.1, 2.2, 3.3]
"#;

        let spec = parse_message_string("test_msgs", "ArrayTest", content).unwrap();

        let numbers_field = spec.get_field("numbers").unwrap();
        if let Some(Value::Array(values)) = &numbers_field.default_value {
            assert_eq!(values.len(), 5);
            assert_eq!(values[0], PrimitiveValue::Int32(1));
            assert_eq!(values[4], PrimitiveValue::Int32(5));
        } else {
            panic!("Expected array default value");
        }

        let names_field = spec.get_field("names").unwrap();
        if let Some(Value::Array(values)) = &names_field.default_value {
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], PrimitiveValue::String("alice".to_string()));
        } else {
            panic!("Expected array default value");
        }
    }

    #[test]
    fn test_comments_and_annotations() {
        let content = r#"
# File level comment
# Second line of file comment

# This constant represents the maximum velocity [m/s]
float64 MAX_VELOCITY=10.0

# Position field with unit annotation [m]
float64 position

# Velocity with multi-line comment
# This represents the current velocity
# measured in meters per second [m/s]  
float64 velocity

@optional
# Optional field
string description
"#;

        let spec = parse_message_string("robot_msgs", "State", content).unwrap();

        // Check file-level comments
        if let Some(AnnotationValue::StringList(comments)) = spec.annotations.get("comment") {
            assert!(comments.len() >= 2);
            assert!(comments[0].contains("File level"));
        }

        // Check constant with unit
        let max_vel = spec.get_constant("MAX_VELOCITY").unwrap();
        if let Some(AnnotationValue::String(unit)) = max_vel.annotations.get("unit") {
            assert_eq!(unit, "m/s");
        }

        // Check field with unit
        let position = spec.get_field("position").unwrap();
        if let Some(AnnotationValue::String(unit)) = position.annotations.get("unit") {
            assert_eq!(unit, "m");
        }

        // Check optional field
        let description = spec.get_field("description").unwrap();
        if let Some(AnnotationValue::Bool(is_optional)) = description.annotations.get("optional") {
            assert!(is_optional);
        }
    }

    #[test]
    fn test_bounded_types() {
        let content = r#"
string<=50 short_string
wstring<=100 wide_string
int32[<=10] bounded_array
float64[5] fixed_array
"#;

        let spec = parse_message_string("test_msgs", "BoundedTest", content).unwrap();

        // Check bounded string
        let short_string = spec.get_field("short_string").unwrap();
        assert_eq!(
            short_string.field_type.base_type.string_upper_bound,
            Some(50)
        );

        let wide_string = spec.get_field("wide_string").unwrap();
        assert_eq!(
            wide_string.field_type.base_type.string_upper_bound,
            Some(100)
        );

        // Check bounded array
        let bounded_array = spec.get_field("bounded_array").unwrap();
        assert!(bounded_array.field_type.is_bounded_array());
        assert_eq!(bounded_array.field_type.array_size, Some(10));

        // Check fixed array
        let fixed_array = spec.get_field("fixed_array").unwrap();
        assert!(fixed_array.field_type.is_array);
        assert!(!fixed_array.field_type.is_upper_bound);
        assert_eq!(fixed_array.field_type.array_size, Some(5));
    }

    #[test]
    fn test_validation_functions() {
        // Package name validation
        assert!(is_valid_package_name("geometry_msgs"));
        assert!(is_valid_package_name("test_package"));
        assert!(!is_valid_package_name("GeometryMsgs")); // uppercase
        assert!(!is_valid_package_name("test-package")); // hyphen

        // Message name validation
        assert!(is_valid_message_name("Point"));
        assert!(is_valid_message_name("TestMessage"));
        assert!(!is_valid_message_name("point")); // lowercase start
        assert!(!is_valid_message_name("test-message")); // hyphen

        // Field name validation
        assert!(is_valid_field_name("position"));
        assert!(is_valid_field_name("test_field"));
        assert!(!is_valid_field_name("Position")); // uppercase start
        assert!(!is_valid_field_name("test-field")); // hyphen

        // Constant name validation
        assert!(is_valid_constant_name("MAX_VALUE"));
        assert!(is_valid_constant_name("PI"));
        assert!(!is_valid_constant_name("max_value")); // lowercase
        assert!(!is_valid_constant_name("Max-Value")); // hyphen
    }

    #[test]
    fn test_display_formatting() {
        // Test message display
        let msg_spec =
            parse_message_string("test_msgs", "TestMsg", "int32 x\nstring name").unwrap();
        let display_str = msg_spec.to_string();
        assert!(display_str.contains("test_msgs/TestMsg"));
        assert!(display_str.contains("int32 x"));
        assert!(display_str.contains("string name"));

        // Test service display
        let srv_spec =
            parse_service_string("test_msgs", "TestSrv", "int32 input\n---\nint32 output").unwrap();
        let display_str = srv_spec.to_string();
        assert!(display_str.contains("test_msgs/TestSrv"));
        assert!(display_str.contains("---"));
        assert!(display_str.contains("int32 input"));
        assert!(display_str.contains("int32 output"));

        // Test action display
        let action_spec = parse_action_string(
            "test_msgs",
            "TestAction",
            "int32 goal\n---\nint32 result\n---\nint32 feedback",
        )
        .unwrap();
        let display_str = action_spec.to_string();
        assert!(display_str.contains("test_msgs/TestAction"));
        assert_eq!(display_str.matches("---").count(), 2);
    }

    #[test]
    fn test_interface_specification_enum() {
        // Test message interface
        let msg_spec = parse_message_string("test_msgs", "TestMsg", "int32 x").unwrap();
        let interface = InterfaceSpecification::Message(msg_spec);

        assert!(interface.is_message());
        assert!(!interface.is_service());
        assert!(!interface.is_action());
        assert_eq!(interface.package_name(), "test_msgs");
        assert_eq!(interface.interface_name(), "TestMsg");
        assert_eq!(interface.full_name(), "test_msgs/TestMsg");
        assert!(interface.as_message().is_some());
        assert!(interface.as_service().is_none());
        assert!(interface.as_action().is_none());

        // Test service interface
        let srv_spec =
            parse_service_string("test_msgs", "TestSrv", "int32 input\n---\nint32 output").unwrap();
        let interface = InterfaceSpecification::Service(srv_spec);

        assert!(!interface.is_message());
        assert!(interface.is_service());
        assert!(!interface.is_action());

        // Test action interface
        let action_spec = parse_action_string(
            "test_msgs",
            "TestAction",
            "int32 goal\n---\nint32 result\n---\nint32 feedback",
        )
        .unwrap();
        let interface = InterfaceSpecification::Action(action_spec);

        assert!(!interface.is_message());
        assert!(!interface.is_service());
        assert!(interface.is_action());
    }
}
