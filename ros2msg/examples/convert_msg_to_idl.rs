//! Example demonstrating MSG/SRV/Action to IDL conversion
//!
//! This example shows how to convert ROS2 message, service, and action files
//! to IDL format, matching the behavior of rosidl_adapter.

use ros2msg::idl_adapter::{action_to_idl, message_to_idl, service_to_idl};
use ros2msg::{parse_action_string, parse_message_string, parse_service_string};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ROS2 MSG/SRV/Action to IDL Converter ===\n");

    // Example 1: Convert a simple message to IDL
    println!("Example 1: Simple Message to IDL");
    println!("----------------------------------");
    let msg_content = r#"
# This is a Point message
int32 x
int32 y
int32 z
"#;
    let msg = parse_message_string("geometry_msgs", "Point", msg_content)?;
    let idl = message_to_idl(&msg, "geometry_msgs", "msg/Point.msg");
    println!("{}\n", idl);

    // Example 2: Convert a message with arrays and constants to IDL
    println!("Example 2: Message with Arrays and Constants");
    println!("---------------------------------------------");
    let msg_content = r#"
# Array example with constants
uint8 TYPE_LINEAR=0
uint8 TYPE_ANGULAR=1

# The type of twist
uint8 type

# Fixed array
float64[3] linear
# Unbounded array
float64[] angular
# Bounded array
string[<=5] names
"#;
    let msg = parse_message_string("geometry_msgs", "Twist", msg_content)?;
    let idl = message_to_idl(&msg, "geometry_msgs", "msg/Twist.msg");
    println!("{}\n", idl);

    // Example 3: Convert a service to IDL
    println!("Example 3: Service to IDL");
    println!("--------------------------");
    let srv_content = r#"
# Request
int32 a
int32 b
---
# Response
int32 sum
"#;
    let srv = parse_service_string("example_srvs", "AddTwoInts", srv_content)?;
    let idl = service_to_idl(&srv, "example_srvs", "srv/AddTwoInts.srv");
    println!("{}\n", idl);

    // Example 4: Convert an action to IDL
    println!("Example 4: Action to IDL");
    println!("------------------------");
    let action_content = r#"
# Goal
int32 order
---
# Result
int32[] sequence
---
# Feedback
int32[] partial_sequence
"#;
    let action = parse_action_string("example_actions", "Fibonacci", action_content)?;
    let idl = action_to_idl(&action, "example_actions", "action/Fibonacci.action");
    println!("{}\n", idl);

    // Example 5: Message with default values
    println!("Example 5: Message with Default Values");
    println!("---------------------------------------");
    let msg_content = r#"
int32 x 0
int32 y 0
string name "default_name"
bool active true
float64 rate 10.0
"#;
    let msg = parse_message_string("example_msgs", "Config", msg_content)?;
    let idl = message_to_idl(&msg, "example_msgs", "msg/Config.msg");
    println!("{}\n", idl);

    // Example 6: Empty message (requires placeholder member)
    println!("Example 6: Empty Message");
    println!("------------------------");
    let msg_content = "";
    let msg = parse_message_string("std_msgs", "Empty", msg_content)?;
    let idl = message_to_idl(&msg, "std_msgs", "msg/Empty.msg");
    println!("{}\n", idl);

    Ok(())
}
