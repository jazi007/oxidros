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
    // Generate RCL bindings
    generate_rcl_bindings(out_path);
    // Link ROS2 C libraries
    link_ros2_libs();
}

fn generate_rcl_bindings(out_dir: &Path) {
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

    // Create a C header that includes all RCL headers
    let rcl_c_content = r#"
#include <rcl/rcl.h>
#include <rcl/types.h>
#include <rcl/logging.h>
#include <rcl_action/rcl_action.h>
#include <rcutils/error_handling.h>
#include <action_msgs/srv/cancel_goal.h>
#include <action_msgs/msg/goal_info.h>
"#;

    let wrapper_path = out_dir.join("rcl_wrapper.h");
    std::fs::write(&wrapper_path, rcl_c_content).expect("Failed to write RCL wrapper header");

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg(format!("-I{}", ros_include.join("rcl").display()))
        .clang_arg(format!("-I{}", ros_include.join("rcutils").display()))
        .clang_arg(format!("-I{}", ros_include.join("rmw").display()))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rcl_yaml_param_parser").display()
        ))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_runtime_c").display()
        ))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_typesupport_interface").display()
        ))
        .clang_arg(format!("-I{}", ros_include.join("rcl_action").display()))
        .clang_arg(format!("-I{}", ros_include.join("action_msgs").display()))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("unique_identifier_msgs").display()
        ))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("builtin_interfaces").display()
        ))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_dynamic_typesupport").display()
        ))
        .clang_arg(format!("-I{}", ros_include.join("service_msgs").display()))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("type_description_interfaces").display()
        ))
        .allowlist_type("rcl_.*")
        .allowlist_function("rcl_.*")
        .allowlist_var("rcl_.*")
        .allowlist_var("RCL_.*")
        .allowlist_type("rmw_.*")
        .allowlist_function("rmw_.*")
        .allowlist_var("rmw_.*")
        .allowlist_var("RMW_.*")
        .allowlist_type("rcutils_.*")
        .allowlist_function("rcutils_.*")
        .allowlist_var("rcutils_.*")
        .allowlist_var("RCUTILS_.*")
        .allowlist_type("rosidl_.*")
        .allowlist_var("rosidl_.*")
        .blocklist_function("atexit")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate RCL bindings");

    let rcl_path = out_dir.join("rcl.rs");
    bindings
        .write_to_file(&rcl_path)
        .expect("Couldn't write RCL bindings!");
}

fn link_ros2_libs() {
    // Link only RCL core libraries (not message libraries - those are in oxidros-msg)
    println!("cargo:rustc-link-lib=rcl");
    println!("cargo:rustc-link-lib=rcl_action");
    println!("cargo:rustc-link-lib=rcutils");
    println!("cargo:rustc-link-lib=rmw");
    println!("cargo:rustc-link-lib=rcl_yaml_param_parser");

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
