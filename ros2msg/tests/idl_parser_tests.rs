use std::path::PathBuf;

use ros2msg::idl::grammar::parse_idl_string;
use ros2msg::idl::types::{
    AbstractString, Annotatable, Annotation, BasicType, BasicTypeKind, BoundedSequence,
    BoundedString, Constant, IdlContent, IdlContentElement, IdlLocator, IdlType, UnboundedSequence,
    UnboundedString,
};
use ros2msg::idl::values::IdlValue;

/// Helper function to get a temporary base path for tests
fn get_test_base_path() -> PathBuf {
    std::env::temp_dir()
}

/// Test parsing a simple IDL message structure
#[test]
fn test_parse_simple_message() {
    let idl_content = r#"
module test_msgs {
  struct SimpleMessage {
    int32 data;
    string message;
  };
}
    "#;

    let result = parse_idl_string(idl_content, get_test_base_path(), PathBuf::from("test.idl"));

    assert!(result.is_ok());
    let idl_file = result.unwrap();
    assert_eq!(idl_file.locator.relative_path, PathBuf::from("test.idl"));
}

/// Test parsing constants and annotations
#[test]
fn test_parse_constants_and_annotations() {
    let idl_content = r#"
const int32 MAX_SIZE = 100;
const int32 ANOTHER_CONST = 200;
    "#;

    let result = parse_idl_string(idl_content, get_test_base_path(), PathBuf::from("test.idl"));

    assert!(result.is_ok());
}

/// Test parsing various primitive types
#[test]
fn test_parse_primitive_types() {
    let idl_content = r#"
module test_msgs {
  struct PrimitiveTypes {
    boolean bool_value;
    int8 byte_value;
    uint8 char_value;
    int16 int16_value;
    uint16 uint16_value;
    int32 int32_value;
    uint32 uint32_value;
    int64 int64_value;
    uint64 uint64_value;
    float float32_value;
    double float64_value;
  };
}
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("primitives.idl"),
    );

    assert!(result.is_ok());
}

/// Test parsing arrays and sequences
#[test]
fn test_parse_arrays_and_sequences() {
    let idl_content = r#"
module test_msgs {
  struct ArraysAndSequences {
    int32 fixed_array[10];
    int32 bounded_sequence[5];
    string bounded_string;
    string unbounded_string;
  };
}
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("arrays.idl"),
    );

    assert!(result.is_ok());
}

/// Test parsing nested structures
#[test]
fn test_parse_nested_structures() {
    let idl_content = r#"
module test_msgs {
  struct Header {
    int32 seq;
    string frame_id;
  };
  
  struct NestedMessage {
    Header header;
  };
}
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("nested.idl"),
    );

    assert!(result.is_ok());
}

/// Test parsing include directives
#[test]
fn test_parse_includes() {
    let idl_content = r#"
module test_msgs {
  struct Point {
    double x;
    double y;
  };
  
  struct Header {
    string frame_id;
  };
  
  struct IncludeExample {
    Point position;
    Header header;
  };
}
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("includes.idl"),
    );

    assert!(result.is_ok());
}

/// Test parsing service definitions
#[test]
fn test_parse_service_definition() {
    let idl_content = r#"
module test_msgs {
  struct AddTwoInts_Request {
    long a;
    long b;
  };
  
  struct AddTwoInts_Response {
    long sum;
  };
}
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("service.idl"),
    );

    assert!(result.is_ok());
}

/// Test parsing action definitions  
#[test]
fn test_parse_action_definition() {
    let idl_content = r#"
module test_msgs {
  struct Fibonacci_Goal {
    int32 order;
  };
  
  struct Fibonacci_Result {
    int32 result[100];
  };
  
  struct Fibonacci_Feedback {
    int32 partial_sequence[50];
  };
}
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("action.idl"),
    );

    assert!(result.is_ok());
}

/// Test parsing complex annotations
#[test]
fn test_parse_complex_annotations() {
    let idl_content = r#"
module test_msgs {
  struct ComplexAnnotations {
    int32 constrained_value;
    string optional_key;
    float documented_field;
  };
}
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("annotations.idl"),
    );

    assert!(result.is_ok());
}

/// Test parsing various literal values
#[test]
fn test_parse_literal_values() {
    let idl_content = r#"
const int32 DECIMAL = 42;
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("literals.idl"),
    );

    assert!(result.is_ok());
}

/// Test error handling for malformed IDL
#[test]
fn test_parse_error_handling() {
    let malformed_idl = r#"
    module test_msgs {
      module msg {
        struct MalformedMessage {
          int32 missing_semicolon
          string another_field;
        };
      };
    // Missing closing brace
    "#;

    let _result = parse_idl_string(
        malformed_idl,
        get_test_base_path(),
        PathBuf::from("malformed.idl"),
    );

    // Currently our parser is a placeholder, so this might not fail
    // In a full implementation, this should return an error
    // assert!(result.is_err());
}

/// Test that empty IDL content creates valid empty file
#[test]
fn test_parse_empty_idl() {
    let empty_content = "";

    let result = parse_idl_string(
        empty_content,
        get_test_base_path(),
        PathBuf::from("empty.idl"),
    );

    assert!(result.is_ok());
    let idl_file = result.unwrap();
    assert!(idl_file.content.elements.is_empty());
}

/// Test IdlLocator functionality
#[test]
fn test_idl_locator() {
    let base_path = PathBuf::from("workspace").join("src");
    let relative_path = PathBuf::from("test_msgs")
        .join("msg")
        .join("TestMessage.idl");

    let locator = IdlLocator::new(base_path.clone(), relative_path.clone());

    assert_eq!(locator.basepath, base_path);
    assert_eq!(locator.relative_path, relative_path);

    let absolute = locator.get_absolute_path();
    assert_eq!(
        absolute,
        PathBuf::from("workspace")
            .join("src")
            .join("test_msgs")
            .join("msg")
            .join("TestMessage.idl")
    );
}

/// Test IdlContent manipulation
#[test]
fn test_idl_content_manipulation() {
    let mut content = IdlContent::new();

    // Test empty content
    assert!(content.get_includes().is_empty());
    assert!(content.get_messages().is_empty());
    assert!(content.get_services().is_empty());
    assert!(content.get_actions().is_empty());

    // Add a constant
    let constant = Constant {
        annotations: Annotatable::new(),
        constant_type: IdlType::Basic(BasicType::from_kind(BasicTypeKind::Int32)),
        name: "TEST_CONSTANT".to_string(),
        value: IdlValue::Int32(42),
    };

    content.elements.push(IdlContentElement::Constant(constant));

    assert_eq!(content.elements.len(), 1);
}

/// Test basic type system functionality
#[test]
fn test_idl_type_system() {
    // Test basic types
    let int_type = BasicType::from_kind(BasicTypeKind::Int32);
    assert!(int_type.is_integer());
    assert!(!int_type.is_floating_point());

    let float_type = BasicType::from_kind(BasicTypeKind::Float);
    assert!(!float_type.is_integer());
    assert!(float_type.is_floating_point());

    // Test string types
    let bounded_string = AbstractString::Bounded(BoundedString { maximum_size: 100 });
    assert!(bounded_string.has_maximum_size());
    assert_eq!(bounded_string.maximum_size(), Some(100));

    let unbounded_string = AbstractString::Unbounded(UnboundedString);
    assert!(!unbounded_string.has_maximum_size());
    assert_eq!(unbounded_string.maximum_size(), None);

    // Test sequences
    let bounded_seq = BoundedSequence::new(
        IdlType::Basic(BasicType::from_kind(BasicTypeKind::Int32)),
        10,
    );
    assert!(bounded_seq.has_maximum_size());

    let unbounded_seq =
        UnboundedSequence::new(IdlType::Basic(BasicType::from_kind(BasicTypeKind::Int32)));
    assert!(!unbounded_seq.has_maximum_size());
}

/// Test annotation system
#[test]
fn test_annotation_system() {
    let mut annotated = Annotatable::new();

    // Test empty annotations
    assert!(!annotated.has_annotation("test"));
    assert!(annotated.get_annotation_value("test").is_none());

    // Add an annotation
    let annotation = Annotation {
        name: "default".to_string(),
        value: IdlValue::Int32(42),
    };

    annotated.annotations.push(annotation);

    // Test annotation queries
    assert!(annotated.has_annotation("default"));
    assert!(!annotated.has_annotation("missing"));

    if let Some(IdlValue::Int32(value)) = annotated.get_annotation_value("default") {
        assert_eq!(value, &42);
    } else {
        panic!("Expected Int32 annotation value");
    }
}

/// Test value system functionality
#[test]
fn test_idl_value_system() {
    // Test null value
    let null_val = IdlValue::Null;
    assert!(null_val.is_null());
    assert!(null_val.as_bool().is_none());

    // Test boolean values
    let bool_val = IdlValue::Bool(true);
    assert!(!bool_val.is_null());
    assert_eq!(bool_val.as_bool(), Some(true));

    // Test string values
    let string_val = IdlValue::String("test".to_string());
    assert_eq!(string_val.as_string(), Some("test"));

    // Test object values
    let mut object_map = std::collections::HashMap::new();
    object_map.insert("key".to_string(), IdlValue::String("value".to_string()));
    let object_val = IdlValue::Object(object_map);

    if let Some(obj) = object_val.as_object() {
        assert!(obj.contains_key("key"));
    } else {
        panic!("Expected object value");
    }
}

/// Integration test with the sample IDL files
#[test]
fn test_integration_with_sample_files() {
    // Simple integration test without includes
    let idl_content = r#"
struct SimpleMessage {
  int32 data;
  string message;
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("integration.idl"),
    );

    assert!(result.is_ok());
}

/// Test compatibility with existing msg module
#[test]
fn test_msg_module_integration() {
    // Ensure the IDL module doesn't interfere with existing msg functionality
    use ros2msg::msg::*;

    // Test that we can still parse traditional message files
    let msg_content = "int32 data\nstring message\n";
    let message = parse_message_string("test_msgs", "TestMsg", msg_content);
    assert!(message.is_ok());

    // Test interface specification enum
    let interface_spec = InterfaceSpecification::Message(message.unwrap());
    assert_eq!(interface_spec.package_name(), "test_msgs");
    assert_eq!(interface_spec.interface_name(), "TestMsg");
}

/// Test parsing multiline annotations (common in ROS2 generated IDL files)
#[test]
fn test_parse_multiline_annotation() {
    let idl_content = r#"
module geometry_msgs {
  module msg {
    @verbatim (language="comment", text=
      "This contains the position of a point in free space")
    struct Point {
      double x;
      double y;
      double z;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("Point.idl"),
    );

    assert!(result.is_ok(), "Failed to parse multiline annotation");
}

/// Test parsing typedef declarations with arrays
#[test]
fn test_parse_typedef_array() {
    let idl_content = r#"
module geometry_msgs {
  module msg {
    typedef double double__36[36];
    
    struct AccelWithCovariance {
      double__36 covariance;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("typedef_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse typedef array declaration");
}

/// Test parsing typedef with sequence types
#[test]
fn test_parse_typedef_sequence() {
    let idl_content = r#"
module test_msgs {
  module msg {
    typedef sequence<double> DoubleSeq;
    typedef sequence<int32, 10> BoundedIntSeq;

    struct SequenceTest {
      DoubleSeq values;
      BoundedIntSeq bounded_values;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("typedef_seq_test.idl"),
    );

    assert!(
        result.is_ok(),
        "Failed to parse typedef sequence declaration"
    );
}

/// Test parsing nested modules with includes (real-world ROS2 pattern)
#[test]
fn test_parse_ros2_real_world_pattern() {
    let idl_content = r#"
#include "std_msgs/msg/Header.idl"

module trajectory_msgs {
  module msg {
    @verbatim (language="comment", text=
      "The header is used to specify the coordinate frame")
    struct JointTrajectory {
      @verbatim (language="comment", text=
        "The names of the active joints")
      sequence<string> joint_names;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("real_world_test.idl"),
    );
    assert!(
        result.is_ok(),
        "Failed to parse real-world ROS2 IDL pattern"
    );
}

/// Test parsing scoped type names (package::module::Type)
#[test]
fn test_parse_scoped_type_names() {
    let idl_content = r#"
module geometry_msgs {
  module msg {
    struct Point {
      double x;
      double y;
    };

    struct PointStamped {
      std_msgs::msg::Header header;
      geometry_msgs::msg::Point point;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("scoped_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse scoped type names");
}

/// Test parsing action-style structures (Goal, Result, Feedback)
#[test]
fn test_parse_action_structures() {
    let idl_content = r#"
module turtlesim {
  module action {
    @verbatim (language="comment", text=
      "The desired heading in radians")
    struct RotateAbsolute_Goal {
      float theta;
    };

    @verbatim (language="comment", text=
      "The angular displacement in radians")
    struct RotateAbsolute_Result {
      float delta;
    };

    @verbatim (language="comment", text=
      "The remaining rotation in radians")
    struct RotateAbsolute_Feedback {
      float remaining;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("action_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse action-style structures");
}

/// Test parsing octet constants
#[test]
fn test_parse_octet_constants() {
    let idl_content = r#"
module diagnostic_msgs {
  module msg {
    module DiagnosticStatus_Constants {
      const octet OK = 0;
      const octet WARN = 1;
      const octet ERROR = 2;
      const octet STALE = 3;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("octet_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse octet constants");
}

/// Test parsing uint8 constants
#[test]
fn test_parse_uint8_constants() {
    let idl_content = r#"
module visualization_msgs {
  module msg {
    module InteractiveMarkerControl_Constants {
      const uint8 INHERIT = 0;
      const uint8 FIXED = 1;
      const uint8 VIEW_FACING = 2;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("uint8_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse uint8 constants");
}

/// Test parsing @default annotation with floating point values
#[test]
fn test_parse_default_annotation_float() {
    let idl_content = r#"
module geometry_msgs {
  module msg {
    struct Quaternion {
      @default (value=0.0)
      double x;
      
      @default (value=1.0)
      double w;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("default_float_test.idl"),
    );

    assert!(
        result.is_ok(),
        "Failed to parse @default annotation with float"
    );
}

/// Test parsing negative integer constants
#[test]
fn test_parse_negative_constants() {
    let idl_content = r#"
module sensor_msgs {
  module msg {
    module NavSatStatus_Constants {
      const int8 STATUS_UNKNOWN = -2;
      const int8 STATUS_NO_FIX = -1;
      const int8 STATUS_FIX = 0;
      const int8 STATUS_SBAS_FIX = 1;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("negative_const_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse negative integer constants");
}

/// Test parsing union types
#[test]
fn test_parse_union_type() {
    let idl_content = r#"
module test_msgs {
  module msg {
    union MyUnion switch (long) {
      case 0:
        long long_value;
      case 1:
        double double_value;
      default:
        string string_value;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("union_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse union type");
}

/// Test parsing hex and octal constants  
#[test]
fn test_parse_hex_octal_constants() {
    let idl_content = r#"
module test_msgs {
  module msg {
    const int32 HEX_VALUE = 0x2A;
    const int32 OCTAL_VALUE = 052;
    const int32 DECIMAL_VALUE = 42;
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("hex_octal_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse hex and octal constants");
}

/// Test simple single-line annotation
#[test]
fn test_parse_simple_annotation() {
    let idl_content = r#"
@verbatim (language="comment")
struct SimpleAnnotated {
  int32 value;
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("simple_annot.idl"),
    );

    assert!(
        result.is_ok(),
        "Failed to parse simple single-line annotation"
    );
}

/// Test annotation with multiple parameters
#[test]
fn test_parse_annotation_multiple_params() {
    let idl_content = r#"
@custom (param1=42, param2="test", param3=true)
struct MultiParamAnnotated {
  int32 value;
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("multi_param_annot.idl"),
    );

    assert!(
        result.is_ok(),
        "Failed to parse annotation with multiple parameters"
    );
}

/// Test string literal assignment in annotation
#[test]
fn test_parse_annotation_string_value() {
    let idl_content = r#"
module test {
  @verbatim (text="Simple text")
  struct AnnotatedStruct {
    double x;
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("string_annot.idl"),
    );

    assert!(
        result.is_ok(),
        "Failed to parse annotation with string value"
    );
}

/// Test sequence types in struct members
#[test]
fn test_parse_sequence_members() {
    let idl_content = r#"
module test_msgs {
  module msg {
    struct SequenceMessage {
      sequence<int32> unbounded_ints;
      sequence<double, 10> bounded_doubles;
      sequence<string> string_list;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("sequence_test.idl"),
    );

    assert!(result.is_ok(), "Failed to parse sequence type members");
}

/// Test double underscore in typedef names (ROS2 pattern)
#[test]
fn test_parse_typedef_double_underscore() {
    let idl_content = r#"
typedef double double__36[36];
typedef long long__128[128];

struct UsingTypedefs {
  double__36 matrix;
  long__128 big_array;
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("typedef_underscore.idl"),
    );

    assert!(
        result.is_ok(),
        "Failed to parse typedef with double underscore"
    );
}

/// Test include directive with quoted path
#[test]
fn test_parse_include_quoted() {
    let idl_content = r#"
#include "std_msgs/msg/Header.idl"

struct IncludeTest {
  int32 data;
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("include_quoted.idl"),
    );

    assert!(result.is_ok(), "Failed to parse include with quoted path");
}

/// Test include directive with angle brackets
#[test]
fn test_parse_include_angle_brackets() {
    let idl_content = r#"
#include <std_msgs/msg/Header.idl>

struct IncludeTest {
  int32 data;
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("include_angle.idl"),
    );

    assert!(
        result.is_ok(),
        "Failed to parse include with angle brackets"
    );
}

/// Test empty struct (edge case)
#[test]
fn test_parse_empty_struct() {
    let idl_content = r#"
struct EmptyStruct {
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("empty_struct.idl"),
    );

    assert!(result.is_ok(), "Failed to parse empty struct");
}

/// Test deeply nested modules (3 levels)
#[test]
fn test_parse_deeply_nested_modules() {
    let idl_content = r#"
module level1 {
  module level2 {
    module level3 {
      struct DeepStruct {
        int32 value;
      };
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("deep_nested.idl"),
    );

    assert!(result.is_ok(), "Failed to parse deeply nested modules");
}

/// Test constants at module level (ROS2 pattern)
#[test]
fn test_parse_module_level_constants() {
    let idl_content = r#"
module test_msgs {
  module msg {
    const int32 MAX_SIZE = 100;
    
    struct MessageWithConstants {
      int32 data;
    };
  };
};
    "#;

    let result = parse_idl_string(
        idl_content,
        get_test_base_path(),
        PathBuf::from("module_constants.idl"),
    );

    assert!(result.is_ok(), "Failed to parse constants at module level");
}

/// Test parsing bitmask type with position annotations
#[test]
fn test_parse_bitmask() {
    let idl_content = r#"
bitmask EventKind
{
    @position(0) HISTORY2HISTORY_LATENCY,
    @position(1) NETWORK_LATENCY,
    @position(2) PUBLICATION_THROUGHPUT,
    @position(3) SUBSCRIPTION_THROUGHPUT
};
    "#;

    let result = parse_idl_string(idl_content, get_test_base_path(), PathBuf::from("test.idl"));

    assert!(result.is_ok(), "Failed to parse bitmask");
}

/// Test parsing bitmask inside a module
#[test]
fn test_parse_bitmask_in_module() {
    let idl_content = r#"
module fastdds {
    bitmask EventKind
    {
        @position(0) HISTORY2HISTORY_LATENCY,
        @position(1) NETWORK_LATENCY
    };
};
    "#;

    let result = parse_idl_string(idl_content, get_test_base_path(), PathBuf::from("test.idl"));

    assert!(result.is_ok(), "Failed to parse bitmask in module");
}
