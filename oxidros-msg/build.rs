use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    oxidros_build::ros2_env_var_changed();
    // Generate message bindings using ros2-msg-gen
    let common_interfaces_deps = [
        "actionlib_msgs",
        "diagnostic_msgs",
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
    std::fs::create_dir_all(&common_interfaces_dir).unwrap();
    ros2_msg_gen::generate_with_prefix(
        &common_interfaces_dir,
        &common_interfaces_deps,
        Some("crate::"),
    )
    .expect("Failed to generate common_interfaces");

    // Generate interfaces
    let interfaces_dir = out_path.join("interfaces");
    std::fs::create_dir_all(&interfaces_dir).unwrap();
    ros2_msg_gen::generate_with_prefix(&interfaces_dir, &interface_deps, Some("crate::"))
        .expect("Failed to generate interfaces");

    // Generate ros2msg
    let ros2msg_dir = out_path.join("ros2msg");
    std::fs::create_dir_all(&ros2msg_dir).unwrap();
    ros2_msg_gen::generate_with_prefix(&ros2msg_dir, &ros2msg_deps, Some("crate::"))
        .expect("Failed to generate ros2msg");

    // Generate runtime_c.rs using bindgen
    oxidros_build::generate_runtime_c(out_path);

    // Link ROS2 C libraries
    oxidros_build::link_msg_ros2_libs();
}
