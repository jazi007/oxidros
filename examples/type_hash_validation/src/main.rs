//! Example that validates ROS2 type hashes against jazzy distribution
//!
//! This example:
//! 1. Uses generated message structs (via build.rs)
//! 2. Automatically discovers ALL ROS2 messages/services in the installation
//! 3. Calls compute_hash() from the derived TypeDescription trait
//! 4. Validates against ROS2 jazzy using ros2 CLI

use std::process::Command;

// Include generated messages
mod generated {
    #![allow(dead_code, non_camel_case_types, clippy::upper_case_acronyms)]
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

// Include the auto-generated test registry with dispatch function
mod test_registry {
    #![allow(dead_code, clippy::upper_case_acronyms)]
    include!(concat!(env!("OUT_DIR"), "/test_registry.rs"));
}

fn main() {
    println!("=== ROS2 Type Hash Validation (Fully Automated) ===\n");
    println!(
        "Discovered {} types to test\n",
        test_registry::ALL_TYPES.len()
    );

    let mut total = 0;
    let mut matches = 0;
    let mut mismatches = 0;
    let mut errors = 0;
    let mut skipped = 0;

    // Group by package for organized output
    let mut current_package = "";

    for entry in test_registry::ALL_TYPES {
        // Print package header when we encounter a new package
        if entry.package != current_package {
            if !current_package.is_empty() {
                println!();
            }
            println!("--- Testing {} ---", entry.package);
            current_package = entry.package;
        }

        // Call the auto-generated dispatch function
        match test_registry::test_type_by_name(entry.ros2_name) {
            TestResult::Match => {
                total += 1;
                matches += 1;
            }
            TestResult::Mismatch => {
                total += 1;
                mismatches += 1;
            }
            TestResult::Error => {
                total += 1;
                errors += 1;
            }
            TestResult::Skipped => {
                skipped += 1;
            }
        }
    }

    // Summary
    println!("\n=== Summary ===");
    println!("Total types tested: {}", total);
    println!("✓ Matches: {}", matches);
    println!("✗ Mismatches: {}", mismatches);
    println!("⚠ Errors: {}", errors);
    if skipped > 0 {
        println!("⊘ Skipped: {}", skipped);
    }

    if mismatches > 0 || errors > 0 {
        std::process::exit(1);
    }
}

#[derive(Debug)]
pub enum TestResult {
    Match,
    Mismatch,
    Error,
    Skipped,
}

pub fn test_type_impl<T: ros2_types::TypeDescription>(
    type_name: &str,
    total: &mut i32,
    matches: &mut i32,
    mismatches: &mut i32,
    errors: &mut i32,
) -> TestResult {
    *total += 1;

    // Debug mode: print JSON representation if DEBUG_HASH env var is set
    let debug = std::env::var("DEBUG_HASH").is_ok();

    match T::compute_hash() {
        Ok(computed_hash) => {
            print!("{}: {}", type_name, computed_hash);

            if debug {
                let desc = T::type_description();
                println!(
                    "\n  Type: {}, Fields: {}, Refs: {}",
                    desc.type_description.type_name,
                    desc.type_description.fields.len(),
                    desc.referenced_type_descriptions.len()
                );
                if debug {
                    println!(
                        "  JSON: {}",
                        serde_json::to_string(&desc).unwrap_or_else(|_| "error".to_string())
                    );
                }
            }

            // Try to get expected hash from ROS2
            match get_ros2_hash(type_name) {
                Ok((expected_hash, ros2_json)) => {
                    if computed_hash == expected_hash {
                        println!(" ✓ MATCH");
                        *matches += 1;
                        TestResult::Match
                    } else {
                        println!(" ✗ MISMATCH");
                        println!("  Expected: {}", expected_hash);

                        // For SaveMap_Request, print both JSONs for comparison
                        if type_name.contains("SaveMap_Request") {
                            let desc = T::type_description();
                            println!("\n=== OUR JSON ===");
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&desc)
                                    .unwrap_or_else(|_| "error".to_string())
                            );
                            println!("\n=== ROS2 JSON ===");
                            println!("{}", ros2_json);
                            println!("=================\n");
                        }

                        *mismatches += 1;
                        TestResult::Mismatch
                    }
                }
                Err(e) => {
                    println!(" ⚠ Cannot verify ({})", e);
                    *errors += 1;
                    TestResult::Error
                }
            }
        }
        Err(e) => {
            println!("{}: ✗ Hash computation failed: {}", type_name, e);
            *errors += 1;
            TestResult::Error
        }
    }
}

fn get_ros2_hash(type_name: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    // Parse type name: pkg/type/Name or pkg/srv/Service_Request
    let parts: Vec<&str> = type_name.split('/').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid type name format: {}", type_name).into());
    }
    let (package, interface_type, full_name) = (parts[0], parts[1], parts[2]);

    // For services and actions, strip _Request/_Response/_Goal/_Result/_Feedback suffix to get the base name
    let (base_name, is_request_response) = if interface_type == "srv" {
        if let Some(name) = full_name.strip_suffix("_Request") {
            (name, Some("Request"))
        } else if let Some(name) = full_name.strip_suffix("_Response") {
            (name, Some("Response"))
        } else {
            (full_name, None)
        }
    } else if interface_type == "action" {
        if let Some(name) = full_name.strip_suffix("_Goal") {
            (name, Some("Goal"))
        } else if let Some(name) = full_name.strip_suffix("_Result") {
            (name, Some("Result"))
        } else if let Some(name) = full_name.strip_suffix("_Feedback") {
            (name, Some("Feedback"))
        } else {
            (full_name, None)
        }
    } else {
        (full_name, None)
    };

    let ros_path = std::env::var("ROS_PATH").unwrap_or_else(|_| "/opt/ros/jazzy".to_string());

    // Try to read pre-computed JSON file first (most ROS2 packages have these)
    let json_path = std::path::PathBuf::from(format!(
        "{}/share/{}/{}/{}.json",
        ros_path, package, interface_type, base_name
    ));

    let json_content = match std::fs::read_to_string(&json_path) {
        Ok(content) => content,
        Err(_) => {
            // If pre-computed JSON doesn't exist, try to compute it with rosidl hash
            let interface_file = format!("{}:{}/{}", package, interface_type, base_name);
            let include_path = format!("{}/share", ros_path);

            // Create temp directory for output
            let temp_dir = std::env::temp_dir().join(format!("rosidl_hash_{}", std::process::id()));
            std::fs::create_dir_all(&temp_dir)?;

            // Run rosidl hash command
            let source_cmd = format!(
                ". {}/setup.bash && rosidl hash {} {} -I {} -o {}",
                ros_path,
                package,
                interface_file,
                include_path,
                temp_dir.display()
            );

            let output = Command::new("bash").args(["-c", &source_cmd]).output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("rosidl hash failed: {}", stderr).into());
            }

            // Read the generated JSON file
            let generated_json_path =
                temp_dir.join(format!("{}/{}.json", interface_type, base_name));
            let content = std::fs::read_to_string(&generated_json_path)?;

            // Cleanup temp directory
            let _ = std::fs::remove_dir_all(&temp_dir);

            content
        }
    };

    // Parse JSON to extract hash
    let json: serde_json::Value = serde_json::from_str(&json_content)?;

    // Find the correct hash based on whether it's Request/Response/Goal/Result/Feedback
    let hash = if let Some(req_resp) = is_request_response {
        // For services/actions, look for the specific type hash
        let target_type = format!("{}/{}/{}_{}", package, interface_type, base_name, req_resp);
        json["type_hashes"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|h| h["type_name"].as_str() == Some(&target_type))
            })
            .and_then(|h| h["hash_string"].as_str())
            .ok_or(format!("Hash not found for {}", target_type))?
            .to_string()
    } else {
        // For messages, look for the exact type
        let target_type = format!("{}/{}/{}", package, interface_type, full_name);
        json["type_hashes"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|h| h["type_name"].as_str() == Some(&target_type))
            })
            .and_then(|h| h["hash_string"].as_str())
            .ok_or(format!("Hash not found for {}", target_type))?
            .to_string()
    };

    Ok((hash, serde_json::to_string_pretty(&json)?))
}
