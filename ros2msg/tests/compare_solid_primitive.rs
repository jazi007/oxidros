// Test to compare SolidPrimitive IDL

use ros2msg::idl_adapter::message_to_idl;
use ros2msg::msg::parse_message_string;
use std::fs;

#[test]
fn compare_solid_primitive() {
    let msg_file = "/opt/ros/jazzy/share/shape_msgs/msg/SolidPrimitive.msg";

    if !std::path::Path::new(msg_file).exists() {
        eprintln!("Skipping - ROS2 not installed");
        return;
    }

    let msg_content = fs::read_to_string(msg_file).unwrap();
    let msg = parse_message_string("shape_msgs", "SolidPrimitive", &msg_content).unwrap();
    let our_idl = message_to_idl(&msg, "shape_msgs", "msg/SolidPrimitive.msg");

    println!("=== Our Generated IDL ===");
    println!("{}", our_idl);
}
