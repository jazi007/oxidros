//! Build script for oxidros-msg
//!
//! Uses ros2msg to generate ROS2 message types with ros2-types-derive for FFI support.

use std::env;
use std::path::Path;

use oxidros_build::msg::get_base_generator;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

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

    // Generate common_interfaces
    let common_interfaces_dir = out_path.join("common_interfaces");
    let generator = get_base_generator(
        &common_interfaces_deps,
        Some("crate::ros2msg".to_string()),
        Some("crate".to_string()),
    )
    .unwrap();
    std::fs::create_dir_all(&common_interfaces_dir).unwrap();

    generator
        .output_dir(&common_interfaces_dir)
        .generate()
        .expect("Failed to generate common_interfaces");

    // Generate interfaces
    let interfaces_dir = out_path.join("interfaces");
    std::fs::create_dir_all(&interfaces_dir).unwrap();
    let generator = get_base_generator(
        &interface_deps,
        Some("crate::ros2msg".to_string()),
        Some("crate".to_string()),
    )
    .unwrap();
    generator
        .output_dir(&interfaces_dir)
        .generate()
        .expect("Failed to generate interfaces");

    // Generate ros2msg
    let ros2msg_dir = out_path.join("ros2msg");
    std::fs::create_dir_all(&ros2msg_dir).unwrap();
    let generator = get_base_generator(
        &ros2msg_deps,
        Some("crate::ros2msg".to_string()),
        Some("crate".to_string()),
    )
    .unwrap();
    generator
        .output_dir(&ros2msg_dir)
        .generate()
        .expect("Failed to generate ros2msg");
    // Generate runtime_c.rs using bindgen
    oxidros_build::generate_runtime_c(out_path);
}
