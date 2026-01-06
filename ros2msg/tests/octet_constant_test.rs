//! Test octet constant generation

use ros2msg::generator::Generator;
use std::fs;

#[test]
fn test_diagnostic_status_byte_constants() {
    // Use the actual DiagnosticStatus.msg from ROS2
    let msg_file = "/opt/ros/jazzy/share/diagnostic_msgs/msg/DiagnosticStatus.msg";

    if !std::path::Path::new(msg_file).exists() {
        eprintln!("Skipping test - ROS2 not installed");
        return;
    }

    let temp_dir = tempfile::tempdir().unwrap();

    let result = Generator::new()
        .includes([msg_file])
        .output_dir(temp_dir.path())
        .generate();

    if let Err(e) = &result {
        eprintln!("Generation error: {}", e);
    }
    assert!(result.is_ok(), "Generation should succeed");

    // Read the generated file
    let generated_file = temp_dir
        .path()
        .join("diagnostic_msgs/msg/diagnostic_status.rs");
    let content = fs::read_to_string(&generated_file).unwrap();

    println!("Generated content:\n{}", content);

    // Also check the IDL generated file if it exists
    let idl_file = temp_dir
        .path()
        .join("diagnostic_msgs/msg/DiagnosticStatus.idl");
    if idl_file.exists() {
        let idl_content = fs::read_to_string(&idl_file).unwrap();
        println!("\nGenerated IDL:\n{}", idl_content);
    }

    // Constants should use u8, not octet
    assert!(
        content.contains("pub const OK: u8") || content.contains("pub const OK : u8"),
        "OK constant should use u8 type, content:\n{}",
        content
    );
    assert!(
        !content.contains(": octet"),
        "Should not contain 'octet' as a type, content:\n{}",
        content
    );
}
