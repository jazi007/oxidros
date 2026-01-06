// Test to debug WString type hash mismatch

use ros2msg::generator::Generator;
use std::fs;

#[test]
fn debug_wstring_hash() {
    let msg_file = "/opt/ros/jazzy/share/example_interfaces/msg/WString.msg";

    if !std::path::Path::new(msg_file).exists() {
        eprintln!("Skipping - ROS2 not installed");
        return;
    }

    let temp_dir = tempfile::tempdir().unwrap();

    Generator::new()
        .includes([msg_file])
        .output_dir(temp_dir.path())
        .generate()
        .expect("Generation should succeed");

    // Read generated Rust file
    let rs_file = temp_dir.path().join("example_interfaces/msg/w_string.rs");
    let content = fs::read_to_string(&rs_file).unwrap();

    println!("=== Generated Rust ===");
    println!("{}", content);
}
