//! Example demonstrating how to use the ros2msg generator
//!
//! This example shows how to:
//! 1. Use build.rs to generate Rust types from ROS2 .msg files
//! 2. Include the generated code in your application
//! 3. Use the generated types
//! 4. Validate type hashes against ROS2 JSON files
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

// Include the test registry for hash validation
#[allow(dead_code)]
mod test_registry {
    include!(concat!(env!("OUT_DIR"), "/test_registry.rs"));
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
#[cfg(test)]
mod tests {
    use super::test_registry::{self, ALL_TYPES};
    use serde::Deserialize;
    use std::fs;

    /// Represents a type hash entry from a JSON file
    #[derive(Debug, Deserialize)]
    struct TypeHash {
        type_name: String,
        hash_string: String,
    }

    /// Represents the JSON file structure
    #[derive(Debug, Deserialize)]
    struct TypeDescriptionJson {
        type_hashes: Vec<TypeHash>,
    }

    /// Read expected hash from JSON file
    fn get_expected_hash(json_path: &str, type_name: &str) -> Option<String> {
        let content = fs::read_to_string(json_path).ok()?;
        let parsed: TypeDescriptionJson = serde_json::from_str(&content).ok()?;

        // Find the hash for this type (first entry is usually the main type)
        parsed
            .type_hashes
            .into_iter()
            .find(|h| h.type_name == type_name)
            .map(|h| h.hash_string)
    }

    /// Test all message type hashes
    #[test]
    fn test_all_type_hashes() {
        let mut total = 0;
        let mut matches = 0;
        let mut mismatches = Vec::new();
        let mut skipped = 0;

        for entry in ALL_TYPES {
            let type_name = format!("{}/{}/{}", entry.package, entry.interface_type, entry.name);

            // Get computed hash based on type
            let computed_hash = match entry.interface_type {
                "msg" => test_registry::get_msg_hash(entry.package, entry.name),
                "srv" => test_registry::get_srv_hash(entry.package, entry.name),
                "action" => test_registry::get_action_hash(entry.package, entry.name),
                _ => None,
            };

            let Some(computed) = computed_hash else {
                skipped += 1;
                continue;
            };

            // Get expected hash from JSON
            let Some(expected) = get_expected_hash(entry.json_path, &type_name) else {
                skipped += 1;
                continue;
            };

            total += 1;

            if computed == expected {
                matches += 1;
                println!("✓ {}: {}", type_name, computed);
            } else {
                mismatches.push((type_name.clone(), computed.clone(), expected.clone()));
                println!("✗ {}", type_name);
                println!("  computed: {}", computed);
                println!("  expected: {}", expected);
            }
        }

        println!();
        println!("=== Summary ===");
        println!("Total tested: {}", total);
        println!("Matches: {}", matches);
        println!("Mismatches: {}", mismatches.len());
        println!("Skipped: {}", skipped);

        if !mismatches.is_empty() {
            println!();
            println!("Failed types:");
            for (name, computed, expected) in &mismatches {
                println!("  {} - got {} expected {}", name, computed, expected);
            }
        }

        assert!(
            mismatches.is_empty(),
            "Type hash mismatches found: {} of {} types",
            mismatches.len(),
            total
        );
    }
}
