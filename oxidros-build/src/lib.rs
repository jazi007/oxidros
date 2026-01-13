//! Build utilities for the oxidros ROS2 Rust ecosystem.
//!
//! This crate provides build script helpers for generating ROS2 FFI bindings
//! and linking against ROS2 libraries. It is designed to be used in `build.rs`
//! files of crates that need to interface with ROS2.
//!
//! # Overview
//!
//! The crate provides functionality for:
//!
//! - **RCL Bindings Generation**: Generate Rust FFI bindings for the ROS2 RCL (ROS Client Library)
//! - **Message Bindings Generation**: Generate Rust FFI bindings for ROS2 message types
//! - **Library Linking**: Set up cargo link directives for ROS2 shared libraries
//! - **Environment Detection**: Handle ROS2 environment variables (`AMENT_PREFIX_PATH`, `ROS_DISTRO`, etc.)
//!
//! # Requirements
//!
//! - A sourced ROS2 installation (e.g., `source /opt/ros/jazzy/setup.bash`)
//! - `AMENT_PREFIX_PATH` environment variable must be set
//! - `ROS_DISTRO` environment variable must be set
//!
//! # Example Usage
//!
//! In your `build.rs`:
//!
//! ```rust,ignore
//! use oxidros_build::{ros2_env_var_changed, generate_rcl_bindings, link_rcl_ros2_libs};
//! use std::path::PathBuf;
//!
//! fn main() {
//!     // Signal cargo to rebuild if ROS2 environment changes
//!     ros2_env_var_changed();
//!
//!     // Generate RCL bindings
//!     let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
//!     generate_rcl_bindings(&out_dir);
//!
//!     // Link ROS2 libraries
//!     link_rcl_ros2_libs();
//! }
//! ```
//!
//! # Platform Support
//!
//! The crate supports both Linux/Unix and Windows platforms, with platform-specific
//! handling for library paths and linking.

use std::{
    env::{self, VarError},
    path::{Path, PathBuf},
};
pub mod msg;

// use bindgen::callbacks::ParseCallbacks;

#[cfg(target_os = "windows")]
pub const SEPARATOR: char = ';';
#[cfg(not(target_os = "windows"))]
pub const SEPARATOR: char = ':';

// #[derive(Debug)]
// struct CustomCallbacks;
//
// impl ParseCallbacks for CustomCallbacks {
//     fn process_comment(&self, comment: &str) -> Option<String> {
//         Some(format!("````text\n{}\n````", comment))
//     }
// }

/// Emits cargo directives to trigger rebuilds when ROS2 environment variables change.
///
/// This function should be called at the beginning of a build script to ensure
/// the crate is rebuilt whenever the ROS2 environment is modified.
///
/// # Emitted Directives
///
/// - Sets `rustc-cfg=feature="<distro>"` where `<distro>` is the value of `ROS_DISTRO`
/// - Sets `rustc-cfg=feature="rcl"` to enable RCL-dependent code paths
/// - Registers `AMENT_PREFIX_PATH`, `CMAKE_PREFIX_PATH`, and `ROS_DISTRO` for change detection
///
/// # Panics
///
/// Panics if `ROS_DISTRO` environment variable is not set. Make sure to source
/// your ROS2 installation before building.
///
/// # Example
///
/// ```rust,ignore
/// // In build.rs
/// oxidros_build::ros2_env_var_changed();
/// ```
pub fn ros2_env_var_changed() {
    match std::env::var_os("ROS_DISTRO") {
        Some(distro_env) => {
            let distro = distro_env.to_string_lossy();
            println!("cargo:rustc-cfg=feature=\"{distro}\"");
            println!("cargo:rustc-cfg=feature=\"rcl\"");
        }
        None => {
            println!("cargo:rustc-cfg=feature=\"zenoh\"");
        }
    }
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=CMAKE_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=ROS_DISTRO");
}

/// Creates a base `bindgen::Builder` with common settings for ROS2 bindings.
///
/// This function returns a pre-configured [`bindgen::Builder`] with settings
/// optimized for generating ROS2 FFI bindings.
///
/// # Configuration
///
/// The builder is configured with:
/// - Rust-style enum generation (non-exhaustive disabled)
/// - `size_t` mapped to `usize`
/// - Comment generation disabled (ROS2 headers often have formatting issues)
///
/// # Returns
///
/// A [`bindgen::Builder`] instance that can be further customized for specific
/// binding generation tasks.
///
/// # Example
///
/// ```rust,ignore
/// let bindings = oxidros_build::builder_base()
///     .header("wrapper.h")
///     .allowlist_function("my_.*")
///     .generate()
///     .expect("Failed to generate bindings");
/// ```
pub fn builder_base() -> bindgen::Builder {
    bindgen::Builder::default()
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .size_t_is_usize(true)
        // https://github.com/rust-lang/rust-bindgen/issues/1313
        // .parse_callbacks(Box::new(CustomCallbacks))
        .generate_comments(false)
}

/// Generates Rust FFI bindings for the ROS2 RCL (ROS Client Library).
///
/// This function generates comprehensive bindings for:
/// - `rcl` - ROS Client Library core functionality
/// - `rcl_action` - Action support
/// - `rcutils` - ROS utilities
/// - `rmw` - ROS Middleware interface
/// - `rosidl` - ROS IDL types
///
/// # Arguments
///
/// * `out_dir` - The output directory where the generated `rcl.rs` file will be written.
///   Typically this is the `OUT_DIR` environment variable from your build script.
///
/// # Generated Files
///
/// - `rcl_wrapper.h` - Temporary C header file combining all RCL headers
/// - `rcl.rs` - The generated Rust bindings
///
/// # Panics
///
/// - If `AMENT_PREFIX_PATH` is not set
/// - If no ROS2 include directory can be found
/// - If the wrapper header cannot be written
/// - If bindgen fails to generate bindings
///
/// # Example
///
/// ```rust,ignore
/// use std::path::PathBuf;
///
/// let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
/// oxidros_build::generate_rcl_bindings(&out_dir);
///
/// // In your lib.rs:
/// // include!(concat!(env!("OUT_DIR"), "/rcl.rs"));
/// ```
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

/// Prints cargo link directives for all ROS2 C libraries in the given path (Windows only).
///
/// This function scans a directory for `.lib` files ending with `_c.lib` and emits
/// `cargo:rustc-link-lib` directives for each. It also adds the directory to the
/// library search path.
///
/// # Arguments
///
/// * `path` - The directory path to scan for library files.
///
/// # Platform
///
/// This function is only compiled on Windows. On other platforms, a no-op stub is provided.
#[cfg(target_os = "windows")]
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

/// No-op stub for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
fn print_all_libs(_path: std::path::PathBuf) {}

/// Retrieves and parses a path-style environment variable into a list of paths.
///
/// This function reads an environment variable containing paths separated by
/// the platform-specific separator (`:` on Unix, `;` on Windows), and returns
/// a deduplicated, sorted list of [`PathBuf`] entries.
///
/// # Arguments
///
/// * `key` - The name of the environment variable to read (e.g., `"AMENT_PREFIX_PATH"`)
///
/// # Returns
///
/// - `Ok(Vec<PathBuf>)` - A sorted, deduplicated list of paths
/// - `Err(VarError)` - If the environment variable is not set or invalid
///
/// # Example
///
/// ```rust,ignore
/// use oxidros_build::get_paths_from_env;
///
/// match get_paths_from_env("AMENT_PREFIX_PATH") {
///     Ok(paths) => {
///         for path in paths {
///             println!("Found: {}", path.display());
///         }
///     }
///     Err(_) => eprintln!("AMENT_PREFIX_PATH not set"),
/// }
/// ```
pub fn get_paths_from_env(key: &str) -> Result<Vec<PathBuf>, VarError> {
    let path = env::var(key)?;
    let mut paths: Vec<_> = env::split_paths(&path).collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// Emits cargo directives to link against ROS2 RCL core libraries.
///
/// This function adds the necessary `cargo:rustc-link-lib` and `cargo:rustc-link-search`
/// directives to link against the core ROS2 libraries needed for RCL functionality.
///
/// # Linked Libraries
///
/// - `rcl` - ROS Client Library
/// - `rcl_action` - Action support
/// - `rcutils` - Utilities
/// - `rmw` - ROS Middleware interface
/// - `rcl_yaml_param_parser` - YAML parameter parsing
///
/// # Library Search Paths
///
/// On Unix systems, adds `<ament_path>/lib` for each path in `AMENT_PREFIX_PATH`.
/// On Windows, adds paths from both `AMENT_PREFIX_PATH` and `CMAKE_PREFIX_PATH`.
///
/// # Note
///
/// This function only links RCL core libraries. For message type libraries,
/// use [`link_msg_ros2_libs`] instead.
///
/// # Example
///
/// ```rust,ignore
/// // In build.rs
/// oxidros_build::link_rcl_ros2_libs();
/// ```
pub fn link_rcl_ros2_libs() {
    // Link only RCL core libraries (not message libraries - those are in oxidros-msg)
    println!("cargo:rustc-link-lib=rcl");
    println!("cargo:rustc-link-lib=rcl_action");
    println!("cargo:rustc-link-lib=rcutils");
    println!("cargo:rustc-link-lib=rmw");
    println!("cargo:rustc-link-lib=rmw_implementation");
    println!("cargo:rustc-link-lib=rcl_yaml_param_parser");

    // Add library search paths from AMENT_PREFIX_PATH
    for path in get_paths_from_env("AMENT_PREFIX_PATH").unwrap_or_default() {
        if cfg!(target_os = "windows") {
            print_all_libs(path.join("Lib"));
        } else {
            println!("cargo:rustc-link-search={}/lib", path.display());
        }
    }
    if cfg!(target_os = "windows") {
        for path in get_paths_from_env("CMAKE_PREFIX_PATH").unwrap_or_default() {
            print_all_libs(path.join("lib"));
            print_all_libs(path.join("Lib"));
        }
    }
}

/// Generates Rust FFI bindings for the ROS2 runtime C types.
///
/// This function generates bindings specifically for message-related types from
/// `rosidl_runtime_c`, including string types, sequence types, and type support
/// structures. These are the foundational types used by all ROS2 message types.
///
/// # Arguments
///
/// * `out_dir` - The output directory where the generated `runtime_c.rs` file will be written.
///
/// # Generated Files
///
/// - `msg_wrapper.h` - Temporary C header file combining rosidl runtime headers
/// - `runtime_c.rs` - The generated Rust bindings
///
/// # Included Types
///
/// - `rosidl_runtime_c` string and u16string types and functions
/// - Primitive sequence types and functions
/// - Message and service type support structures
/// - `builtin_interfaces::msg::Time` type
///
/// # Panics
///
/// - If `AMENT_PREFIX_PATH` is not set
/// - If no ROS2 include directory can be found
/// - If the wrapper header cannot be written
/// - If bindgen fails to generate bindings
///
/// # Example
///
/// ```rust,ignore
/// use std::path::PathBuf;
///
/// let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
/// oxidros_build::generate_runtime_c(&out_dir);
///
/// // In your lib.rs:
/// // include!(concat!(env!("OUT_DIR"), "/runtime_c.rs"));
/// ```
pub fn generate_runtime_c(out_dir: &Path) {
    // Get ROS include paths from AMENT_PREFIX_PATH
    let ros_include = get_paths_from_env("AMENT_PREFIX_PATH")
        .expect("AMENT_PREFIX_PATH not set")
        .iter()
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
#include <rosidl_runtime_c/action_type_support_struct.h>
#include <rosidl_runtime_c/string.h>
#include <rosidl_runtime_c/string_functions.h>
#include <rosidl_runtime_c/u16string.h>
#include <rosidl_runtime_c/u16string_functions.h>
#include <rosidl_runtime_c/visibility_control.h>
#include <builtin_interfaces/msg/time.h>
#include <rmw/rmw.h>
"#;

    let wrapper_path = out_dir.join("msg_wrapper.h");
    std::fs::write(&wrapper_path, msg_c_content).expect("Failed to write wrapper header");

    let bindings = builder_base()
        .header(wrapper_path.to_str().unwrap())
        .derive_copy(false)
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
        .clang_arg(format!("-I{}", ros_include.join("rmw").display()))
        .clang_arg(format!(
            "-I{}",
            ros_include.join("rosidl_dynamic_typesupport").display()
        ))
        .clang_arg(format!("-I{}", ros_include.join("rcutils").display()))
        .allowlist_type("rosidl_.*")
        .allowlist_function("rosidl_.*")
        .allowlist_function("rmw_deserialize")
        .allowlist_function("rmw_serialize")
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

/// Emits cargo directives to link against ROS2 message libraries.
///
/// This function adds link directives for the `rosidl_runtime_c` library and
/// the type support libraries for common ROS2 message packages.
///
/// # Linked Libraries
///
/// Core library:
/// - `rosidl_runtime_c` - Runtime C library for rosidl
///
/// Message packages (Unix only):
/// - `actionlib_msgs`, `action_msgs`, `builtin_interfaces`
/// - `diagnostic_msgs`, `geometry_msgs`, `nav_msgs`
/// - `sensor_msgs`, `shape_msgs`, `std_msgs`, `std_srvs`
/// - `stereo_msgs`, `trajectory_msgs`, `unique_identifier_msgs`
/// - `visualization_msgs`, `rcl_interfaces`
///
/// For each package, both `*__rosidl_typesupport_c` and `*__rosidl_generator_c`
/// libraries are linked.
///
/// # Distro-Specific Libraries
///
/// On ROS2 Jazzy and Iron, additional libraries are linked:
/// - `service_msgs`
/// - `type_description_interfaces`
///
/// # Note
///
/// This function complements [`link_rcl_ros2_libs`]. The library search paths
/// are assumed to already be set by [`link_rcl_ros2_libs`] or similar.
///
/// # Example
///
/// ```rust,ignore
/// // In build.rs
/// oxidros_build::link_rcl_ros2_libs(); // Sets up library paths
/// oxidros_build::link_msg_ros2_libs(); // Links message libraries
/// ```
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
