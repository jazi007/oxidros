//! Example demonstrating how to use the ros2msg generator
//!
//! This example shows how to:
//! 1. Use build.rs to generate Rust types from ROS2 .msg files
//! 2. Include the generated code in your application
//! 3. Use the generated types
//!
//! ## Prerequisites
//!
//! Before building this example, you need to source your ROS2 environment:
//! ```bash
//! source /opt/ros/jazzy/setup.bash  # or humble, etc.
//! cargo build -p ros2msg_example
//! ```

// Include the generated message types from build.rs
#[allow(
    dead_code,
    unused_imports,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated/mod.rs"));
}

fn main() {
    println!("ros2msg Generator Example");
    println!("========================");
    println!();

    // Demonstrate using the generated types
    println!("Generated message types from std_msgs package:");
    println!();

    // Create a ColorRGBA message
    let color = generated::std_msgs::msg::ColorRGBA {
        r: 1.0,
        g: 0.5,
        b: 0.25,
        a: 1.0,
    };
    println!(
        "  ColorRGBA: r={}, g={}, b={}, a={}",
        color.r, color.g, color.b, color.a
    );

    // Create a simple Int32 message using Default
    let int_msg = generated::std_msgs::msg::Int32::default();
    println!("  Int32 (default): data={}", int_msg.data);

    // Create a Float64 message
    let float_msg = generated::std_msgs::msg::Float64 {
        data: std::f64::consts::PI,
    };
    println!("  Float64: data={:.5}", float_msg.data);

    // Create a Bool message
    let bool_msg = generated::std_msgs::msg::Bool { data: true };
    println!("  Bool: data={}", bool_msg.data);

    println!();
    println!("Key features demonstrated:");
    println!("  - build.rs generates Rust types from ROS2 .msg files");
    println!("  - ParseCallbacks customize derives and attributes");
    println!("  - Ros2Msg derive generates Default and type support code");
    println!();
    println!("See build.rs for the generation code and how to customize it.");
}
