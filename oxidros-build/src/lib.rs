use std::{env, path::Path};

use bindgen::callbacks::ParseCallbacks;

#[cfg(target_os = "windows")]
pub const SEPARATOR: char = ';';
#[cfg(not(target_os = "windows"))]
pub const SEPARATOR: char = ':';

// https://github.com/rust-lang/rust-bindgen/issues/1313
#[derive(Debug)]
struct CustomCallbacks;

impl ParseCallbacks for CustomCallbacks {
    fn process_comment(&self, comment: &str) -> Option<String> {
        Some(format!("````text\n{}\n````", comment))
    }
}

pub fn ros2_env_var_changed() {
    let distro_env = std::env::var_os("ROS_DISTRO").expect("Source your ros2 env");
    let distro = distro_env.to_string_lossy();
    println!("cargo:rustc-cfg=feature=\"{distro}\"");
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=CMAKE_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=ROS_DISTRO");
}

pub fn builder_base() -> bindgen::Builder {
    bindgen::Builder::default()
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .size_t_is_usize(true)
        .parse_callbacks(Box::new(CustomCallbacks))
}

pub fn generate_rcl_bindings(out_dir: &Path) {
    // Get ROS include paths from AMENT_PREFIX_PATH
    let ament_prefix_path = env::var("AMENT_PREFIX_PATH")
        .expect("AMENT_PREFIX_PATH not set. Please source your ROS2 installation.");

    let ros_include = ament_prefix_path
        .split(SEPARATOR)
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

    let bindings = builder_base()
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
        .generate()
        .expect("Unable to generate RCL bindings");

    let rcl_path = out_dir.join("rcl.rs");
    bindings
        .write_to_file(&rcl_path)
        .expect("Couldn't write RCL bindings!");
}

#[cfg(target_os = "windows")]
// for windows find and add all ros2 libraries to avoid link issues
fn print_all_libs(path: std::path::PathBuf) {
    if path.exists() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for pp in entries
                .into_iter()
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap().path())
                .filter(|p| p.is_file() && p.extension().is_some())
            {
                if let Some(p) = pp.to_str() {
                    if p.ends_with("_c.lib") {
                        println!(
                            "cargo:rustc-link-lib={}",
                            pp.file_stem().unwrap().to_str().unwrap()
                        );
                    }
                }
            }
        }
        if let Some(pp) = path.to_str() {
            println!("cargo:rustc-link-search={}", pp);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn print_all_libs(_path: std::path::PathBuf) {}

pub fn link_rcl_ros2_libs() {
    // Link only RCL core libraries (not message libraries - those are in oxidros-msg)
    println!("cargo:rustc-link-lib=rcl");
    println!("cargo:rustc-link-lib=rcl_action");
    println!("cargo:rustc-link-lib=rcutils");
    println!("cargo:rustc-link-lib=rmw");
    println!("cargo:rustc-link-lib=rcl_yaml_param_parser");

    // Add library search paths from AMENT_PREFIX_PATH
    if let Ok(ament_prefix_path) = env::var("AMENT_PREFIX_PATH") {
        for path in ament_prefix_path.split(SEPARATOR) {
            if cfg!(target_os = "windows") {
                print_all_libs(std::path::Path::new(&path).join("Lib"));
            } else {
                println!("cargo:rustc-link-search={}/lib", path);
            }
        }
    }

    if cfg!(target_os = "windows") {
        if let Ok(cmake_prefix_path) = env::var("CMAKE_PREFIX_PATH") {
            for path in cmake_prefix_path.split(SEPARATOR) {
                print_all_libs(std::path::Path::new(&path).join("lib"));
            }
        }
    }
}

pub fn generate_runtime_c(out_dir: &Path) {
    // Get ROS include paths from AMENT_PREFIX_PATH
    let ament_prefix_path = env::var("AMENT_PREFIX_PATH")
        .expect("AMENT_PREFIX_PATH not set. Please source your ROS2 installation.");
    let ros_include = ament_prefix_path
        .split(SEPARATOR)
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

    let bindings = builder_base()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_runtime_c").display()
        ))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("builtin_interfaces").display()
        ))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_typesupport_interface").display()
        ))
        .clang_arg(format!("-I{}", ros_include.join("rcutils").display()))
        .allowlist_type("rosidl_.*")
        .allowlist_function("rosidl_.*")
        .allowlist_var("rosidl_.*")
        .allowlist_type("builtin_interfaces__msg__Time")
        .blocklist_function("atexit")
        .generate()
        .expect("Unable to generate bindings for runtime_c");

    let runtime_c_path = out_dir.join("runtime_c.rs");
    bindings
        .write_to_file(&runtime_c_path)
        .expect("Couldn't write runtime_c bindings!");
}

pub fn link_msg_ros2_libs() {
    // Note: Core RCL libraries and library search paths are handled by oxidros-rcl
    // This only links rosidl_runtime_c and message-specific libraries
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

        // Distro-specific message libraries
        let distro = env::var("ROS_DISTRO").unwrap_or_default();
        match distro.as_str() {
            "jazzy" | "iron" => {
                println!("cargo:rustc-link-lib=service_msgs__rosidl_typesupport_c");
                println!("cargo:rustc-link-lib=service_msgs__rosidl_generator_c");
                println!("cargo:rustc-link-lib=type_description_interfaces__rosidl_typesupport_c");
                println!("cargo:rustc-link-lib=type_description_interfaces__rosidl_generator_c");
            }
            _ => {}
        }
    }
}
