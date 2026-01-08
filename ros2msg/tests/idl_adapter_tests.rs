//! Integration tests for MSG/SRV/Action to IDL converter

use ros2msg::idl_adapter::{action_to_idl, message_to_idl, service_to_idl};
use ros2msg::{parse_action_string, parse_message_string, parse_service_string};

#[test]
fn test_complete_message_to_idl() {
    let msg_content = r#"
# Header
# Multiple line
# comment
uint8 TYPE_A=0
uint8 TYPE_B=1

# Fields with defaults
int32 x 10
float64 y 3.14
string name "default"
bool flag true

# Arrays
int32[5] fixed_array
int32[] dynamic_array
int32[<=10] bounded_array
string<=20 bounded_string
"#;

    let msg = parse_message_string("test_msgs", "CompleteMsg", msg_content).unwrap();
    let idl = message_to_idl(&msg, "test_msgs", "msg/CompleteMsg.msg");

    // Check header
    assert!(idl.contains("// generated from rosidl_adapter/resource/msg.idl.em"));
    assert!(idl.contains("// with input from test_msgs/msg/CompleteMsg.msg"));

    // Check module structure
    assert!(idl.contains("module test_msgs {"));
    assert!(idl.contains("module msg {"));

    // Check constants module
    assert!(idl.contains("module CompleteMsg_Constants {"));
    assert!(idl.contains("const uint8 TYPE_A = 0;"));
    assert!(idl.contains("const uint8 TYPE_B = 1;"));

    // Check struct
    assert!(idl.contains("struct CompleteMsg {"));

    // Check default annotations
    assert!(idl.contains("@default (value=10)"));
    assert!(idl.contains("@default (value=3.14)"));
    assert!(idl.contains("@default (value=\"default\")"));
    assert!(idl.contains("@default (value=TRUE)"));

    // Check field types
    assert!(idl.contains("int32 x;"));
    assert!(idl.contains("double y;"));
    assert!(idl.contains("string name;"));
    assert!(idl.contains("boolean flag;"));

    // Check array types - fixed arrays now use typedefs
    assert!(idl.contains("typedef int32 int32__5[5];"));
    assert!(idl.contains("int32__5 fixed_array;"));
    assert!(idl.contains("sequence<int32> dynamic_array;"));
    assert!(idl.contains("sequence<int32, 10> bounded_array;"));
    assert!(idl.contains("string<20> bounded_string;"));
}

#[test]
fn test_complete_service_to_idl() {
    let srv_content = r#"
# Request constants
uint8 MODE_A=0
---
# Response constants  
uint8 STATUS_OK=0
bool success
"#;

    let srv = parse_service_string("test_srvs", "TestSrv", srv_content).unwrap();
    let idl = service_to_idl(&srv, "test_srvs", "srv/TestSrv.srv");

    // Check header
    assert!(idl.contains("// generated from rosidl_adapter/resource/srv.idl.em"));
    assert!(idl.contains("// with input from test_srvs/srv/TestSrv.srv"));

    // Check module structure
    assert!(idl.contains("module test_srvs {"));
    assert!(idl.contains("module srv {"));

    // Check request and response structs
    assert!(idl.contains("struct TestSrv_Request {"));
    assert!(idl.contains("struct TestSrv_Response {"));

    // Check constants
    assert!(idl.contains("TestSrv_Request_Constants {"));
    assert!(idl.contains("const uint8 MODE_A = 0;"));
    assert!(idl.contains("TestSrv_Response_Constants {"));
    assert!(idl.contains("const uint8 STATUS_OK = 0;"));

    // Check response field
    assert!(idl.contains("boolean success;"));
}

#[test]
fn test_complete_action_to_idl() {
    let action_content = r#"
# Goal
int32 count
---
# Result
bool success
int32 final_count
---
# Feedback
int32 current_count
float32 progress
"#;

    let action = parse_action_string("test_actions", "TestAction", action_content).unwrap();
    let idl = action_to_idl(&action, "test_actions", "action/TestAction.action");

    // Check header
    assert!(idl.contains("// generated from rosidl_adapter/resource/action.idl.em"));
    assert!(idl.contains("// with input from test_actions/action/TestAction.action"));

    // Check module structure
    assert!(idl.contains("module test_actions {"));
    assert!(idl.contains("module action {"));

    // Check all three structs
    assert!(idl.contains("struct TestAction_Goal {"));
    assert!(idl.contains("struct TestAction_Result {"));
    assert!(idl.contains("struct TestAction_Feedback {"));

    // Check goal fields
    assert!(idl.contains("int32 count;"));

    // Check result fields
    assert!(idl.contains("boolean success;"));
    assert!(idl.contains("int32 final_count;"));

    // Check feedback fields
    assert!(idl.contains("int32 current_count;"));
    assert!(idl.contains("float progress;"));
}

#[test]
fn test_type_conversions() {
    let msg_content = r#"
bool bool_field
byte byte_field
char char_field
int8 int8_field
uint8 uint8_field
int16 int16_field
uint16 uint16_field
int32 int32_field
uint32 uint32_field
int64 int64_field
uint64 uint64_field
float32 float32_field
float64 float64_field
string string_field
"#;

    let msg = parse_message_string("test_msgs", "TypeTest", msg_content).unwrap();
    let idl = message_to_idl(&msg, "test_msgs", "msg/TypeTest.msg");

    // Verify type conversions
    assert!(idl.contains("boolean bool_field;"));
    assert!(idl.contains("octet byte_field;"));
    assert!(idl.contains("uint8 char_field;")); // char is mapped to uint8 in IDL
    assert!(idl.contains("int8 int8_field;"));
    assert!(idl.contains("uint8 uint8_field;"));
    assert!(idl.contains("int16 int16_field;"));
    assert!(idl.contains("uint16 uint16_field;"));
    assert!(idl.contains("int32 int32_field;"));
    assert!(idl.contains("uint32 uint32_field;"));
    assert!(idl.contains("int64 int64_field;"));
    assert!(idl.contains("uint64 uint64_field;"));
    assert!(idl.contains("float float32_field;"));
    assert!(idl.contains("double float64_field;"));
    assert!(idl.contains("string string_field;"));
}

#[test]
fn test_multiline_comments() {
    let msg_content = r#"
# This is line 1
# This is line 2
# This is line 3
int32 field
"#;

    let msg = parse_message_string("test_msgs", "CommentTest", msg_content).unwrap();
    let idl = message_to_idl(&msg, "test_msgs", "msg/CommentTest.msg");

    // Verify comment annotation
    assert!(idl.contains("@verbatim (language=\"comment\""));
    assert!(idl.contains("This is line 1"));
    assert!(idl.contains("This is line 2"));
    assert!(idl.contains("This is line 3"));
}

#[test]
fn test_empty_structs() {
    // Empty message
    let msg = parse_message_string("std_msgs", "Empty", "").unwrap();
    let idl = message_to_idl(&msg, "std_msgs", "msg/Empty.msg");
    assert!(idl.contains("uint8 structure_needs_at_least_one_member;"));

    // Empty service request/response
    let srv = parse_service_string("test_srvs", "EmptySrv", "---").unwrap();
    let idl = service_to_idl(&srv, "test_srvs", "srv/EmptySrv.srv");
    assert!(idl.contains("uint8 structure_needs_at_least_one_member;")); // Should appear twice
}

#[test]
fn test_fixed_char_array_gid() {
    // Test rmw_dds_common/msg/Gid.msg: char[16] data
    let msg_content = "char[16] data";
    let msg = parse_message_string("rmw_dds_common", "Gid", msg_content).unwrap();
    let idl = message_to_idl(&msg, "rmw_dds_common", "msg/Gid.msg");

    assert!(idl.contains("module rmw_dds_common {"));
    assert!(idl.contains("struct Gid {"));
    // char in ROS2 MSG maps to uint8 in IDL
    assert!(idl.contains("typedef uint8 uint8__16[16];"));
    assert!(idl.contains("uint8__16 data;"));
}

#[test]
fn test_wstring_type() {
    // Test example_interfaces/msg/WString.msg: wstring data
    let msg_content = "wstring data";
    let msg = parse_message_string("example_interfaces", "WString", msg_content).unwrap();
    let idl = message_to_idl(&msg, "example_interfaces", "msg/WString.msg");

    println!("Generated IDL:\n{}", idl);

    assert!(idl.contains("module example_interfaces {"));
    assert!(idl.contains("struct WString {"));
    assert!(idl.contains("wstring data;"));
}

#[test]
fn test_service_event_info() {
    // Test service_msgs/msg/ServiceEventInfo.msg with constants and char array
    let msg_content = r#"
uint8 REQUEST_SENT = 0
uint8 REQUEST_RECEIVED = 1
uint8 RESPONSE_SENT = 2
uint8 RESPONSE_RECEIVED = 3

uint8 event_type
builtin_interfaces/Time stamp
char[16] client_gid
int64 sequence_number
"#;
    let msg = parse_message_string("service_msgs", "ServiceEventInfo", msg_content).unwrap();
    let idl = message_to_idl(&msg, "service_msgs", "msg/ServiceEventInfo.msg");

    assert!(idl.contains("module service_msgs {"));
    assert!(idl.contains("module ServiceEventInfo_Constants {"));
    assert!(idl.contains("const uint8 REQUEST_SENT = 0;"));
    assert!(idl.contains("const uint8 REQUEST_RECEIVED = 1;"));
    assert!(idl.contains("const uint8 RESPONSE_SENT = 2;"));
    assert!(idl.contains("const uint8 RESPONSE_RECEIVED = 3;"));
    assert!(idl.contains("struct ServiceEventInfo {"));
    assert!(idl.contains("uint8 event_type;"));
    // IDL uses #include and short type name, not fully qualified name
    assert!(idl.contains("#include \"builtin_interfaces/msg/Time.idl\""));
    assert!(idl.contains("Time stamp;"));
    // char in ROS2 MSG maps to uint8 in IDL
    assert!(idl.contains("typedef uint8 uint8__16[16];"));
    assert!(idl.contains("uint8__16 client_gid;"));
    assert!(idl.contains("int64 sequence_number;"));
}

#[test]
fn test_diagnostic_status_octet_constants() {
    // Test diagnostic_msgs/msg/DiagnosticStatus.msg with byte/octet constants
    let msg_content = r#"
byte OK=0
byte WARN=1
byte ERROR=2
byte STALE=3

byte level
string name
string message
string hardware_id
diagnostic_msgs/KeyValue[] values
"#;
    let msg = parse_message_string("diagnostic_msgs", "DiagnosticStatus", msg_content).unwrap();
    let idl = message_to_idl(&msg, "diagnostic_msgs", "msg/DiagnosticStatus.msg");

    assert!(idl.contains("module diagnostic_msgs {"));
    assert!(idl.contains("module DiagnosticStatus_Constants {"));
    assert!(idl.contains("const octet OK = 0;"));
    assert!(idl.contains("const octet WARN = 1;"));
    assert!(idl.contains("const octet ERROR = 2;"));
    assert!(idl.contains("const octet STALE = 3;"));
    assert!(idl.contains("struct DiagnosticStatus {"));
    assert!(idl.contains("octet level;"));
}

#[test]
fn test_list_nodes_service_to_idl() {
    // Test composition_interfaces/srv/ListNodes.srv pattern:
    // Empty request, response with string arrays
    let srv_content = r#"
---
string[] full_node_names
uint64[] unique_ids
"#;

    let srv = parse_service_string("composition_interfaces", "ListNodes", srv_content).unwrap();
    let idl = service_to_idl(&srv, "composition_interfaces", "srv/ListNodes.srv");

    // Print the generated IDL for debugging
    println!("=== Generated IDL ===\n{}\n=== End IDL ===", idl);

    // Check header
    assert!(
        idl.contains("// generated from rosidl_adapter/resource/srv.idl.em"),
        "Missing srv.idl.em header"
    );
    assert!(
        idl.contains("// with input from composition_interfaces/srv/ListNodes.srv"),
        "Missing source file path"
    );

    // Check module structure
    assert!(
        idl.contains("module composition_interfaces {"),
        "Missing package module"
    );
    assert!(idl.contains("module srv {"), "Missing srv module");

    // Check both request and response structs exist
    assert!(
        idl.contains("struct ListNodes_Request {"),
        "Missing ListNodes_Request struct"
    );
    assert!(
        idl.contains("struct ListNodes_Response {"),
        "Missing ListNodes_Response struct"
    );

    // Request should have structure_needs_at_least_one_member (empty struct)
    let request_section = idl
        .split("struct ListNodes_Request")
        .nth(1)
        .expect("Should have Request section");
    assert!(
        request_section.contains("uint8 structure_needs_at_least_one_member;"),
        "Empty request should have dummy member"
    );

    // Response should have the actual fields
    let response_section = idl
        .split("struct ListNodes_Response")
        .nth(1)
        .expect("Should have Response section");
    assert!(
        response_section.contains("sequence<string> full_node_names;"),
        "Response should have full_node_names field"
    );
    assert!(
        response_section.contains("sequence<uint64> unique_ids;"),
        "Response should have unique_ids field"
    );

    // Make sure there's no duplication
    let request_count = idl.matches("struct ListNodes_Request").count();
    let response_count = idl.matches("struct ListNodes_Response").count();
    assert_eq!(
        request_count, 1,
        "Should have exactly one ListNodes_Request, found {}",
        request_count
    );
    assert_eq!(
        response_count, 1,
        "Should have exactly one ListNodes_Response, found {}",
        response_count
    );
}
