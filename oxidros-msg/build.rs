use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    let distro_env = std::env::var_os("ROS_DISTRO").unwrap();
    let distro = distro_env.to_string_lossy();
    println!("cargo:rustc-cfg=feature=\"{distro}\"");
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=CMAKE_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=ROS_DISTRO");

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
    generate_runtime_c(out_path);

    // Link ROS2 C libraries
    link_ros2_libs();
}

fn generate_runtime_c(out_dir: &Path) {
    // Get ROS include paths from AMENT_PREFIX_PATH
    let ament_prefix_path = env::var("AMENT_PREFIX_PATH")
        .expect("AMENT_PREFIX_PATH not set. Please source your ROS2 installation.");

    let separator = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };
    let ros_include = ament_prefix_path
        .split(separator)
        .find_map(|path| {
            let include_path = Path::new(path).join("include");
            if include_path.exists() {
                Some(include_path)
            } else {
                None
            }
        })
        .expect("Could not find ROS2 include directory in AMENT_PREFIX_PATH");

    // Create a simple C header to bind
    let msg_c_content = r#"
#include <rosidl_runtime_c/message_initialization.h>
#include <rosidl_runtime_c/message_type_support_struct.h>
#include <rosidl_runtime_c/primitives_sequence.h>
#include <rosidl_runtime_c/primitives_sequence_functions.h>
#include <rosidl_runtime_c/sequence_bound.h>
#include <rosidl_runtime_c/service_type_support_struct.h>
#include <rosidl_runtime_c/string.h>
#include <rosidl_runtime_c/string_functions.h>
#include <rosidl_runtime_c/u16string.h>
#include <rosidl_runtime_c/u16string_functions.h>
#include <rosidl_runtime_c/visibility_control.h>
#include <builtin_interfaces/msg/time.h>
"#;

    let wrapper_path = out_dir.join("msg_wrapper.h");
    std::fs::write(&wrapper_path, msg_c_content).expect("Failed to write wrapper header");

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_runtime_c").display()
        ))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("builtin_interfaces").display()
        ))
        .clang_arg(format!("-I{}", ros_include.join("rcl").display()))
        .clang_arg(format!("-I{}", ros_include.join("rcutils").display()))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_typesupport_interface").display()
        ))
        .allowlist_type("rosidl_.*")
        .allowlist_function("rosidl_.*")
        .allowlist_var("rosidl_.*")
        .allowlist_type("builtin_interfaces__msg__Time")
        .blocklist_function("atexit")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate bindings for runtime_c");

    let runtime_c_path = out_dir.join("runtime_c.rs");
    bindings
        .write_to_file(&runtime_c_path)
        .expect("Couldn't write runtime_c bindings!");
}

fn link_ros2_libs() {
    println!("cargo:rustc-link-lib=rcl");
    println!("cargo:rustc-link-lib=rcl_action");
    println!("cargo:rustc-link-lib=rcutils");
    println!("cargo:rustc-link-lib=rmw");
    println!("cargo:rustc-link-lib=rosidl_runtime_c");

    if !cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=actionlib_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=actionlib_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=action_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=action_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=builtin_interfaces__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=builtin_interfaces__rosidl_generator_c");
        println!("cargo:rustc-link-lib=diagnostic_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=diagnostic_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=geometry_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=geometry_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=nav_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=nav_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=sensor_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=sensor_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=shape_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=shape_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=std_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=std_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=std_srvs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=std_srvs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=stereo_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=stereo_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=trajectory_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=trajectory_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=unique_identifier_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=unique_identifier_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=visualization_msgs__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=visualization_msgs__rosidl_generator_c");
        println!("cargo:rustc-link-lib=rcl_interfaces__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=rcl_interfaces__rosidl_generator_c");
    }

    // Add library search paths from AMENT_PREFIX_PATH
    if let Ok(ament_prefix_path) = env::var("AMENT_PREFIX_PATH") {
        let separator = if cfg!(target_os = "windows") {
            ';'
        } else {
            ':'
        };
        for path in ament_prefix_path.split(separator) {
            println!("cargo:rustc-link-search={}/lib", path);
        }
    }

    if cfg!(target_os = "windows") {
        if let Ok(cmake_prefix_path) = env::var("CMAKE_PREFIX_PATH") {
            for path in cmake_prefix_path.split(';') {
                println!("cargo:rustc-link-search={}/lib", path);
            }
        }
    }
}
