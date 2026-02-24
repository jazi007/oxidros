//! Build script for oxidros-msg
//!
//! Uses ros2msg to generate ROS2 message types with ros2-types-derive for FFI support.
//!
//! # Generation Strategy
//!
//! - **ROS2 Sourced/Installed**: Regenerates message files into `src/generated/`
//! - **No ROS2**: Uses pre-committed files in `src/generated/` (no generation)
//!
//! This allows the crate to be built without a ROS2 installation by using
//! pre-generated message definitions committed to the repository.

use std::env;
use std::path::{Path, PathBuf};

use oxidros_build::msg::{Config, RosAvailability, detect_ros_availability, get_base_generator};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    // Get the crate's source directory for generating into src/generated/
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_generated = PathBuf::from(&manifest_dir).join("src").join("generated");

    oxidros_build::ros2_env_var_changed();

    // Define ROS2 package groups
    let common_interfaces_deps = [
        "actionlib_msgs",
        "diagnostic_msgs",
        "example_interfaces",
        "geometry_msgs",
        "nav_msgs",
        "sensor_msgs",
        "shape_msgs",
        "std_msgs",
        "std_srvs",
        "stereo_msgs",
        "trajectory_msgs",
        "visualization_msgs",
    ];

    let interface_deps = [
        "action_msgs",
        "builtin_interfaces",
        "composition_interfaces",
        "lifecycle_msgs",
        "rcl_interfaces",
        "rosgraph_msgs",
        "service_msgs",
        "statistics_msgs",
        "type_description_interfaces",
    ];

    let ros2msg_deps = ["unique_identifier_msgs"];

    // Check ROS2 availability to decide whether to generate
    let config = Config::builder().build();
    let availability = detect_ros_availability(&config);

    match &availability {
        RosAvailability::Sourced { .. } | RosAvailability::CommonInstall { .. } => {
            println!("cargo:info=ROS2 detected, regenerating message files to src/generated/");

            // Create the generated directory structure
            std::fs::create_dir_all(&src_generated).unwrap();

            // Generate common_interfaces
            let common_interfaces_dir = src_generated.join("common_interfaces");
            std::fs::create_dir_all(&common_interfaces_dir).unwrap();
            let config = Config::builder()
                .packages(&common_interfaces_deps)
                .uuid_path("crate::ros2msg")
                .primitive_path("crate")
                .build();
            if let Some(generator) = get_base_generator(&config) {
                generator
                    .output_dir(&common_interfaces_dir)
                    .generate()
                    .expect("Failed to generate common_interfaces");
            }

            // Generate interfaces
            let interfaces_dir = src_generated.join("interfaces");
            std::fs::create_dir_all(&interfaces_dir).unwrap();
            let config = Config::builder()
                .packages(&interface_deps)
                .uuid_path("crate::ros2msg")
                .primitive_path("crate")
                .build();
            if let Some(generator) = get_base_generator(&config) {
                generator
                    .output_dir(&interfaces_dir)
                    .generate()
                    .expect("Failed to generate interfaces");
            }

            // Generate ros2msg
            let ros2msg_dir = src_generated.join("ros2msg");
            std::fs::create_dir_all(&ros2msg_dir).unwrap();
            let config = Config::builder()
                .packages(&ros2msg_deps)
                .uuid_path("crate::ros2msg")
                .primitive_path("crate")
                .build();
            if let Some(generator) = get_base_generator(&config) {
                generator
                    .output_dir(&ros2msg_dir)
                    .generate()
                    .expect("Failed to generate ros2msg");
            }
        }
        RosAvailability::NotAvailable => {
            println!("cargo:warning=No ROS2 installation detected");
            println!("cargo:warning=Using pre-generated message files from src/generated/");

            // Verify that pre-generated files exist
            if !src_generated
                .join("common_interfaces")
                .join("mod.rs")
                .exists()
            {
                panic!(
                    "Pre-generated message files not found in {}. \
                     Either install ROS2 and regenerate, or ensure the generated files are committed.",
                    src_generated.display()
                );
            }
        }
    }

    // Generate runtime_c.rs using bindgen (only when ROS2 is sourced for rcl feature)
    if availability.is_sourced() {
        oxidros_build::generate_runtime_c(out_path);

        // Link ROS2 libraries (required for tests and standalone use)
        oxidros_build::link_rcl_ros2_libs();
        oxidros_build::link_msg_ros2_libs();
    }
}
