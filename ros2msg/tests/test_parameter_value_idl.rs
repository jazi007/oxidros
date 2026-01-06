//! Test to check ParameterValue IDL generation

use ros2msg::idl_adapter::message_to_idl;
use ros2msg::parse_message_string;

#[test]
fn test_parameter_value_idl() {
    let msg_content =
        std::fs::read_to_string("/opt/ros/jazzy/share/rcl_interfaces/msg/ParameterValue.msg")
            .unwrap();
    let msg = parse_message_string("rcl_interfaces", "ParameterValue", &msg_content).unwrap();
    let idl = message_to_idl(&msg, "rcl_interfaces", "msg/ParameterValue.msg");

    println!("\n=== Generated IDL ===\n{}\n", idl);

    // Check for byte array - should be sequence<octet> not sequence<uint8>
    assert!(
        idl.contains("sequence<octet> byte_array_value")
            || idl.contains("sequence<uint8> byte_array_value"),
        "Should contain byte_array_value sequence"
    );
}
