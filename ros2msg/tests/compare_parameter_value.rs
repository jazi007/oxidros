// Test to compare our ParameterValue IDL generation with official

use ros2msg::idl_adapter::message_to_idl;
use ros2msg::msg::parse_message_string;
use std::fs;

#[test]
fn compare_parameter_value_idl() {
    let msg_file = "/opt/ros/jazzy/share/rcl_interfaces/msg/ParameterValue.msg";

    if !std::path::Path::new(msg_file).exists() {
        eprintln!("Skipping - ROS2 not installed");
        return;
    }

    // Read the original MSG
    let msg_content = fs::read_to_string(msg_file).unwrap();

    // Parse and generate IDL
    let msg = parse_message_string("rcl_interfaces", "ParameterValue", &msg_content).unwrap();

    println!("=== Parsed Message Structure ===");
    println!("Message annotations: {:?}", msg.annotations);
    for (i, field) in msg.fields.iter().enumerate() {
        println!(
            "Field {}: {} - annotations: {:?}",
            i, field.name, field.annotations
        );
    }

    let our_idl = message_to_idl(&msg, "rcl_interfaces", "msg/ParameterValue.msg");

    println!("=== Our Generated IDL ===");
    println!("{}", our_idl);

    // Read official IDL
    let official_idl =
        fs::read_to_string("/opt/ros/jazzy/share/rcl_interfaces/msg/ParameterValue.idl").unwrap();
    println!("\n=== Official IDL ===");
    println!("{}", official_idl);

    // Compare key parts
    assert!(our_idl.contains("struct ParameterValue"));
    assert!(our_idl.contains("uint8 type"));
}
