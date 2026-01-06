//! Integration tests comparing IDL conversion against ROS2 Jazzy official IDL files
//!
//! These tests parse ROS2 .msg/.srv/.action files from the Jazzy installation,
//! convert them to IDL using our converter, then parse both our generated IDL
//! and the official IDL to compare the parsed structures.

use ros2msg::idl::parse_idl_string;
use ros2msg::idl_adapter::{action_to_idl, message_to_idl, service_to_idl};
use ros2msg::{parse_action_file, parse_message_file, parse_service_file};
use std::fs;
use std::path::{Path, PathBuf};

const ROS2_JAZZY_SHARE: &str = "/opt/ros/jazzy/share";

/// Compare two parsed IDL structures
fn compare_parsed_idl(
    generated_idl_text: &str,
    official_idl_text: &str,
    context: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Parse both IDLs
    let generated_idl = parse_idl_string(generated_idl_text)?;
    let official_idl = parse_idl_string(official_idl_text)?;

    // Compare the parsed structures
    if generated_idl != official_idl {
        eprintln!("\n=== IDL Structure Mismatch for {} ===", context);
        eprintln!("\n--- Generated IDL Text ---");
        eprintln!("{}", generated_idl_text);
        eprintln!("\n--- Official IDL Text ---");
        eprintln!("{}", official_idl_text);
        eprintln!("\n--- Generated Parsed Structure ---");
        eprintln!("{:#?}", generated_idl);
        eprintln!("\n--- Official Parsed Structure ---");
        eprintln!("{:#?}", official_idl);
        eprintln!("\n=== End Mismatch ===\n");
        Ok(false)
    } else {
        Ok(true)
    }
}

/// Find all files with a given extension in a package directory
fn find_files(package_dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(package_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some(extension) {
                files.push(path);
            }
        }
    }

    files
}

/// Test message conversion for a specific package
fn test_package_messages(package_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let package_path = Path::new(ROS2_JAZZY_SHARE).join(package_name);
    if !package_path.exists() {
        eprintln!("Skipping {} - package not found", package_name);
        return Ok(());
    }

    let msg_dir = package_path.join("msg");
    if !msg_dir.exists() {
        return Ok(()); // No messages in this package
    }

    let msg_files = find_files(&msg_dir, "msg");
    let mut tested = 0;
    let mut passed = 0;

    for msg_file in msg_files {
        let msg_name = msg_file.file_stem().unwrap().to_str().unwrap();
        let idl_file = msg_file.with_extension("idl");

        if !idl_file.exists() {
            eprintln!(
                "Warning: IDL file not found for {}/{}",
                package_name, msg_name
            );
            continue;
        }

        // Parse the message file
        let msg_spec = match parse_message_file(package_name, &msg_file) {
            Ok(spec) => spec,
            Err(e) => {
                eprintln!("Failed to parse {}/{}: {}", package_name, msg_name, e);
                continue;
            }
        };

        // Convert to IDL
        let input_file = format!("msg/{}.msg", msg_name);
        let generated_idl = message_to_idl(&msg_spec, package_name, &input_file);

        // Read official IDL
        let official_idl = fs::read_to_string(&idl_file)?;

        // Compare
        tested += 1;
        match compare_parsed_idl(
            &generated_idl,
            &official_idl,
            &format!("{}/{}", package_name, msg_name),
        ) {
            Ok(true) => passed += 1,
            Ok(false) => {
                eprintln!("FAILED: {}/{}", package_name, msg_name);
            }
            Err(e) => {
                eprintln!("ERROR parsing IDL for {}/{}: {}", package_name, msg_name, e);
            }
        }
    }

    if tested > 0 {
        println!("{}: {}/{} messages passed", package_name, passed, tested);
    }

    assert_eq!(
        passed, tested,
        "{}: Some message conversions don't match official IDL",
        package_name
    );
    Ok(())
}

/// Test service conversion for a specific package
fn test_package_services(package_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let package_path = Path::new(ROS2_JAZZY_SHARE).join(package_name);
    if !package_path.exists() {
        eprintln!("Skipping {} - package not found", package_name);
        return Ok(());
    }

    let srv_dir = package_path.join("srv");
    if !srv_dir.exists() {
        return Ok(()); // No services in this package
    }

    let srv_files = find_files(&srv_dir, "srv");
    let mut tested = 0;
    let mut passed = 0;

    for srv_file in srv_files {
        let srv_name = srv_file.file_stem().unwrap().to_str().unwrap();
        let idl_file = srv_file.with_extension("idl");

        if !idl_file.exists() {
            eprintln!(
                "Warning: IDL file not found for {}/{}",
                package_name, srv_name
            );
            continue;
        }

        // Parse the service file
        let srv_spec = match parse_service_file(package_name, &srv_file) {
            Ok(spec) => spec,
            Err(e) => {
                eprintln!("Failed to parse {}/{}: {}", package_name, srv_name, e);
                continue;
            }
        };

        // Convert to IDL
        let input_file = format!("srv/{}.srv", srv_name);
        let generated_idl = service_to_idl(&srv_spec, package_name, &input_file);

        // Read official IDL
        let official_idl = fs::read_to_string(&idl_file)?;

        // Compare
        tested += 1;
        match compare_parsed_idl(
            &generated_idl,
            &official_idl,
            &format!("{}/{}", package_name, srv_name),
        ) {
            Ok(true) => passed += 1,
            Ok(false) => {
                eprintln!("FAILED: {}/{}", package_name, srv_name);
            }
            Err(e) => {
                eprintln!("ERROR parsing IDL for {}/{}: {}", package_name, srv_name, e);
            }
        }
    }

    if tested > 0 {
        println!("{}: {}/{} services passed", package_name, passed, tested);
    }

    assert_eq!(
        passed, tested,
        "{}: Some service conversions don't match official IDL",
        package_name
    );
    Ok(())
}

/// Test action conversion for a specific package
fn test_package_actions(package_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let package_path = Path::new(ROS2_JAZZY_SHARE).join(package_name);
    if !package_path.exists() {
        eprintln!("Skipping {} - package not found", package_name);
        return Ok(());
    }

    let action_dir = package_path.join("action");
    if !action_dir.exists() {
        return Ok(()); // No actions in this package
    }

    let action_files = find_files(&action_dir, "action");
    let mut tested = 0;
    let mut passed = 0;

    for action_file in action_files {
        let action_name = action_file.file_stem().unwrap().to_str().unwrap();
        let idl_file = action_file.with_extension("idl");

        if !idl_file.exists() {
            eprintln!(
                "Warning: IDL file not found for {}/{}",
                package_name, action_name
            );
            continue;
        }

        // Parse the action file
        let action_spec = match parse_action_file(package_name, &action_file) {
            Ok(spec) => spec,
            Err(e) => {
                eprintln!("Failed to parse {}/{}: {}", package_name, action_name, e);
                continue;
            }
        };

        // Convert to IDL
        let input_file = format!("action/{}.action", action_name);
        let generated_idl = action_to_idl(&action_spec, package_name, &input_file);

        // Read official IDL
        let official_idl = fs::read_to_string(&idl_file)?;

        // Compare
        tested += 1;
        match compare_parsed_idl(
            &generated_idl,
            &official_idl,
            &format!("{}/{}", package_name, action_name),
        ) {
            Ok(true) => passed += 1,
            Ok(false) => {
                eprintln!("FAILED: {}/{}", package_name, action_name);
            }
            Err(e) => {
                eprintln!(
                    "ERROR parsing IDL for {}/{}: {}",
                    package_name, action_name, e
                );
            }
        }
    }

    if tested > 0 {
        println!("{}: {}/{} actions passed", package_name, passed, tested);
    }

    assert_eq!(
        passed, tested,
        "{}: Some action conversions don't match official IDL",
        package_name
    );
    Ok(())
}

// Tests for common ROS2 packages

#[test]
#[ignore] // Run with: cargo test --test jazzy_integration_tests -- --ignored
fn test_std_msgs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("std_msgs")
}

#[test]
#[ignore]
fn test_std_srvs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_services("std_srvs")
}

#[test]
#[ignore]
fn test_geometry_msgs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("geometry_msgs")
}

#[test]
#[ignore]
fn test_sensor_msgs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("sensor_msgs")
}

#[test]
#[ignore]
fn test_nav_msgs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("nav_msgs")
}

#[test]
#[ignore]
fn test_trajectory_msgs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("trajectory_msgs")
}

#[test]
#[ignore]
fn test_action_msgs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("action_msgs")
}

#[test]
#[ignore]
fn test_example_interfaces_msgs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("example_interfaces")
}

#[test]
#[ignore]
fn test_example_interfaces_srvs() -> Result<(), Box<dyn std::error::Error>> {
    test_package_services("example_interfaces")
}

#[test]
#[ignore]
fn test_example_interfaces_actions() -> Result<(), Box<dyn std::error::Error>> {
    test_package_actions("example_interfaces")
}

#[test]
#[ignore]
fn test_builtin_interfaces() -> Result<(), Box<dyn std::error::Error>> {
    test_package_messages("builtin_interfaces")
}

/// Comprehensive test that tests all packages at once
#[test]
#[ignore] // Run with: cargo test test_all_jazzy_packages -- --ignored --nocapture
fn test_all_jazzy_packages() -> Result<(), Box<dyn std::error::Error>> {
    let packages = vec![
        "std_msgs",
        "std_srvs",
        "geometry_msgs",
        "sensor_msgs",
        "nav_msgs",
        "trajectory_msgs",
        "action_msgs",
        "example_interfaces",
        "builtin_interfaces",
        "diagnostic_msgs",
        "shape_msgs",
        "stereo_msgs",
        "visualization_msgs",
    ];

    println!("\n=== Testing IDL Conversion Against ROS2 Jazzy ===\n");

    for package in packages {
        // Test messages
        test_package_messages(package)?;

        // Test services
        test_package_services(package)?;

        // Test actions
        test_package_actions(package)?;
    }

    println!("\n=== All tests passed! ===");

    Ok(())
}

/// Test a single specific message for debugging
#[test]
#[ignore]
fn test_single_message_empty() -> Result<(), Box<dyn std::error::Error>> {
    let msg_file = Path::new(ROS2_JAZZY_SHARE).join("std_msgs/msg/Empty.msg");
    let idl_file = Path::new(ROS2_JAZZY_SHARE).join("std_msgs/msg/Empty.idl");

    let msg_spec = parse_message_file("std_msgs", &msg_file)?;
    let generated_idl = message_to_idl(&msg_spec, "std_msgs", "msg/Empty.msg");
    let official_idl = fs::read_to_string(&idl_file)?;

    assert!(
        compare_parsed_idl(&generated_idl, &official_idl, "std_msgs/Empty")?,
        "Empty message IDL structures don't match"
    );

    Ok(())
}

/// Test a single specific message with fields
#[test]
#[ignore]
fn test_single_message_header() -> Result<(), Box<dyn std::error::Error>> {
    let msg_file = Path::new(ROS2_JAZZY_SHARE).join("std_msgs/msg/Header.msg");
    let idl_file = Path::new(ROS2_JAZZY_SHARE).join("std_msgs/msg/Header.idl");

    let msg_spec = parse_message_file("std_msgs", &msg_file)?;
    let generated_idl = message_to_idl(&msg_spec, "std_msgs", "msg/Header.msg");
    let official_idl = fs::read_to_string(&idl_file)?;

    assert!(
        compare_parsed_idl(&generated_idl, &official_idl, "std_msgs/Header")?,
        "Header message IDL structures don't match"
    );

    Ok(())
}
