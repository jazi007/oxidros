//! ROS2 message code generation utilities.
//!
//! This module provides functionality for generating Rust types from ROS2 interface
//! definition files (`.msg`, `.srv`, `.action`, `.idl`). It uses the [`ros2msg`] crate
//! for parsing and code generation, combined with [`ros2_types`] derive macros for
//! generating type support code.
//!
//! # Overview
//!
//! The module provides two main functions:
//!
//! - [`emit_ros_idl`] - Emits cargo link directives for a single ROS2 package
//! - [`generate_msgs`] - Generates Rust types from all ROS2 interface files
//!
//! # Example
//!
//! ```rust,ignore
//! // In build.rs
//! use oxidros_build::msg::{emit_ros_idl, generate_msgs};
//!
//! fn main() {
//!     // Generate types from specific packages
//!     generate_msgs(&["std_msgs", "geometry_msgs", "sensor_msgs"]);
//!
//!     // Or generate types from ALL packages in AMENT_PREFIX_PATH
//!     generate_msgs(&[]);
//! }
//! ```
//!
//! # Generated Output
//!
//! The generated Rust code includes:
//! - Struct definitions for each message/service/action type
//! - `#[ros2(...)]` attributes for FFI interop
//! - `Ros2Msg` derive macro implementations for type support
//! - Proper module hierarchy matching the ROS2 package structure
//!
//! # Interface File Priority
//!
//! When both `.idl` and native (`.msg`, `.srv`, `.action`) files exist for the
//! same interface, the native files take priority to avoid duplicate definitions.

use ros2msg::generator::{
    Generator, InterfaceKind, ItemInfo, ModuleInfo, ModuleLevel, ParseCallbacks,
};
use std::env;
use std::path::Path;

use crate::get_paths_from_env;

/// Custom callbacks for generating ROS2 FFI-compatible types.
///
/// This struct implements [`ParseCallbacks`] to customize the code generation
/// process for ROS2 message types. It adds the necessary attributes and derives
/// for types to work with the `ros2-type-hash` crate's FFI layer.
///
/// # Customizations
///
/// - Adds `#[ros2(package = "...", interface_type = "...")]` attributes
/// - Adds `ros2_types::Ros2Msg` derive macro to all types
/// - Re-exports types from their modules for convenient access
struct ExampleCallbacks;

impl ParseCallbacks for ExampleCallbacks {
    /// Adds the `#[ros2(...)]` attribute required by the `Ros2Msg` derive macro.
    ///
    /// This attribute provides the package name and interface type information
    /// needed for proper FFI type support generation.
    fn add_attributes(&self, info: &ItemInfo) -> Vec<String> {
        let package = info.package();
        let interface_type = match info.interface_kind() {
            InterfaceKind::Message => "msg",
            InterfaceKind::Service => "srv",
            InterfaceKind::Action => "action",
        };
        vec![format!(
            "#[ros2(package = \"{}\", interface_type = \"{}\")]",
            package, interface_type
        )]
    }

    /// Adds the `Ros2Msg` derive macro to generate type support code.
    ///
    /// The derive macro generates implementations for FFI conversion traits
    /// and type support lookup functions.
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        vec!["ros2_types::Ros2Msg".to_string()]
    }

    /// Re-exports types from their modules for convenient access.
    ///
    /// This allows users to access types without navigating deep module hierarchies.
    /// For example, `std_msgs::msg::String` instead of `std_msgs::msg::string::String`.
    fn post_module(&self, info: &ModuleInfo) -> Option<String> {
        match info.module_level() {
            ModuleLevel::Type(_) => Some(format!("pub use {}::*;\n", info.module_name())),
            _ => None,
        }
    }
}

/// Collects all interface files from a ROS2 package directory.
///
/// This function scans the `msg/`, `srv/`, and `action/` subdirectories of a
/// package for interface definition files. It handles the case where both
/// native (`.msg`, `.srv`, `.action`) and IDL (`.idl`) versions of an interface
/// exist, preferring the native format to avoid duplicate definitions.
///
/// # Arguments
///
/// * `pkg_path` - The path to the ROS2 package directory (e.g., `/opt/ros/jazzy/share/std_msgs`)
///
/// # Returns
///
/// A vector of absolute paths to interface files found in the package.
///
/// # File Priority
///
/// When both `Foo.msg` and `Foo.idl` exist in the same directory, only `Foo.msg`
/// is included. This prevents duplicate type definitions during code generation.
///
/// # Example
///
/// ```rust,ignore
/// let files = collect_interface_files("/opt/ros/jazzy/share/std_msgs");
/// // Returns paths like:
/// // - /opt/ros/jazzy/share/std_msgs/msg/String.msg
/// // - /opt/ros/jazzy/share/std_msgs/msg/Header.msg
/// // etc.
/// ```
fn collect_interface_files(pkg_path: &str) -> Vec<String> {
    use std::collections::HashSet;
    let mut files = Vec::new();
    let mut seen_names: HashSet<String> = HashSet::new();

    // Interface subdirectories and their extensions
    let subdirs = [("msg", "msg"), ("srv", "srv"), ("action", "action")];

    for (subdir, ext) in &subdirs {
        let dir_path = format!("{}/{}", pkg_path, subdir);
        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            // First pass: collect non-IDL files and track their names
            let mut idl_files = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                if path.extension().is_some_and(|e| e == *ext) {
                    // Non-IDL file takes priority
                    files.push(path.to_string_lossy().to_string());
                    seen_names.insert(stem.to_string());
                } else if path.extension().is_some_and(|e| e == "idl") {
                    // Save IDL files for second pass
                    idl_files.push((stem.to_string(), path.to_string_lossy().to_string()));
                }
            }

            // Second pass: only add IDL files if no non-IDL version exists
            for (stem, idl_path) in idl_files {
                if !seen_names.contains(&stem) {
                    files.push(idl_path);
                    seen_names.insert(stem);
                }
            }
        }
    }

    files
}

/// Emits cargo link directives for a ROS2 package's rosidl libraries.
///
/// This function outputs the necessary cargo directives to link against the
/// type support and generator libraries for a specific ROS2 package. It handles
/// both Unix and Windows platforms with appropriate library search paths.
///
/// # Arguments
///
/// * `pkg` - The name of the ROS2 package (e.g., `"std_msgs"`, `"geometry_msgs"`)
///
/// # Emitted Directives
///
/// - `cargo:rustc-link-lib=<pkg>__rosidl_typesupport_c` - Type support library
/// - `cargo:rustc-link-lib=<pkg>__rosidl_generator_c` - Generator library
/// - Library search paths from `AMENT_PREFIX_PATH` and `CMAKE_PREFIX_PATH` (Windows)
///
/// # Platform Behavior
///
/// - **Unix**: Adds `<ament_path>/lib` to search paths
/// - **Windows**: Adds both `<ament_path>/Lib` and `<cmake_path>/lib` to search paths
///
/// # Example
///
/// ```rust,ignore
/// // Link libraries for std_msgs package
/// oxidros_build::msg::emit_ros_idl("std_msgs");
///
/// // After this, you can use std_msgs types in your code
/// ```
pub fn emit_ros_idl(pkg: &str) {
    // Re-run build if environment variables change
    println!("cargo:rustc-link-lib={pkg}__rosidl_typesupport_c");
    println!("cargo:rustc-link-lib={pkg}__rosidl_generator_c");

    if cfg!(target_os = "windows") {
        println!("cargo:rerun-if-env-changed=CMAKE_PREFIX_PATH");
        for path in get_paths_from_env("AMENT_PREFIX_PATH").unwrap_or_default() {
            let p = path.join("Lib");
            if p.exists() {
                println!("cargo:rustc-link-search={}", p.display());
            }
        }
        for path in get_paths_from_env("CMAKE_PREFIX_PATH").unwrap_or_default() {
            let p = path.join("lib");
            if p.exists() {
                println!("cargo:rustc-link-search={}", p.display());
            }
        }
    } else if let Ok(paths) = get_paths_from_env("AMENT_PREFIX_PATH") {
        for path in paths {
            println!("cargo:rustc-link-search={}/lib", path.display());
        }
    }
}

/// Generates Rust types from ROS2 interface files for the specified packages.
///
/// This function scans `AMENT_PREFIX_PATH` for ROS2 packages and generates Rust
/// code from their interface definition files (`.msg`, `.srv`, `.action`, `.idl`).
/// The generated code includes proper type support for FFI interop with ROS2.
///
/// # Arguments
///
/// * `packages` - A slice of package names to generate types for. If empty,
///   types are generated for ALL packages found in `AMENT_PREFIX_PATH`.
///
/// # Generated Output
///
/// The function creates a `generated/` directory inside `OUT_DIR` containing
/// Rust modules for each package with interface files. Each generated type has:
///
/// - The `#[ros2(package = "...", interface_type = "...")]` attribute
/// - The `Ros2Msg` derive macro for type support generation
/// - Debug trait derivation
///
/// # Cargo Directives
///
/// For each package with interface files, the function emits:
/// - `cargo:rustc-link-lib=<pkg>__rosidl_typesupport_c`
/// - `cargo:rustc-link-lib=<pkg>__rosidl_generator_c`
/// - `cargo:rerun-if-env-changed=AMENT_PREFIX_PATH`
/// - `cargo:rerun-if-env-changed=ROS_DISTRO`
///
/// # Example
///
/// ```rust,ignore
/// // Generate types for specific packages
/// oxidros_build::msg::generate_msgs(&["std_msgs", "geometry_msgs"]);
///
/// // Generate types for ALL packages (useful for comprehensive testing)
/// oxidros_build::msg::generate_msgs(&[]);
///
/// // In your lib.rs, include the generated code:
/// // include!(concat!(env!("OUT_DIR"), "/generated/mod.rs"));
/// ```
///
/// # Notes
///
/// - Packages starting with `ament_`, `ros2`, or named `cmake` or `colcon-core`
///   are automatically skipped.
/// - When both `.idl` and native interface files exist, native files take priority.
/// - If `AMENT_PREFIX_PATH` is not set, a warning is emitted and no code is generated.
pub fn generate_msgs(packages: &[&str]) {
    if std::env::var_os("ROS_DISTRO").is_some() {
        println!("cargo:rustc-cfg=feature=\"rcl\"");
        println!("cargo:rerun-if-env-changed=ROS_DISTRO");
        println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
        if cfg!(target_os = "windows") {
            println!("cargo:rerun-if-env-changed=CMAKE_PREFIX_PATH");
        }
    }
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);
    let ament_paths: Vec<_> = get_paths_from_env("AMENT_PREFIX_PATH")
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            let p = p.join("share");
            if p.exists() { Some(p) } else { None }
        })
        .collect();
    if ament_paths.is_empty() {
        println!("cargo:warning=AMENT_PREFIX_PATH not set. Source your ROS2 setup.bash first.");
        println!("cargo:warning=Example: source /opt/ros/jazzy/setup.bash");
        return;
    }

    // Collect ALL interface files from ALL packages in AMENT_PREFIX_PATH
    let mut all_files = Vec::new();
    let mut packages_found = Vec::new();

    for ament_path in &ament_paths {
        // List all directories in the share path (each is a package)
        if let Ok(entries) = std::fs::read_dir(ament_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let pkg_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    // Skip ament-specific directories and non-package directories
                    if pkg_name.starts_with("ament_")
                        || pkg_name.starts_with("ros2")
                        || pkg_name == "cmake"
                        || pkg_name == "colcon-core"
                        || pkg_name.is_empty()
                    {
                        continue;
                    }

                    let pkg_path = path.to_string_lossy().to_string();
                    if !packages.is_empty() && !packages.contains(&pkg_name) {
                        continue;
                    }
                    let files = collect_interface_files(&pkg_path);
                    if !files.is_empty() {
                        // Only emit link directives for packages that have interface files
                        emit_ros_idl(pkg_name);
                        packages_found.push(pkg_name.to_string());
                        all_files.extend(files);
                    }
                }
            }
        }
    }

    if all_files.is_empty() {
        println!("cargo:warning=No interface files found in AMENT_PREFIX_PATH");
        return;
    }

    println!(
        "cargo:info=Found {} interface files from {} packages",
        all_files.len(),
        packages_found.len()
    );

    // Print some package names for visibility
    let preview: Vec<_> = packages_found.iter().take(10).collect();
    println!("cargo:info=Packages include: {:?}...", preview);

    // Create output directory
    let generated_dir = out_path.join("generated");
    std::fs::create_dir_all(&generated_dir).expect("Failed to create output directory");

    // Generate Rust code from ALL interface files
    // Note: We don't use derive_default(true) because Ros2Msg derive macro
    // generates its own Default impl that properly handles FFI types
    Generator::new()
        .header(
            "// Auto-generated ROS2 message types\n// Generated by ros2msg from all ROS2 packages",
        )
        .derive_debug(true)
        .parse_callbacks(Box::new(ExampleCallbacks))
        .includes(all_files)
        .output_dir(&generated_dir)
        .emit_rerun_if_changed(true)
        .allowlist_recursively(true)
        .package_search_paths(ament_paths)
        .generate()
        .expect("Failed to generate message types");

    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
}
