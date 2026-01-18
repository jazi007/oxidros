//! ROS2 message code generation.
//!
//! This module provides the core generation logic for creating Rust types
//! from ROS2 interface definition files.

use ros2msg::generator::Generator;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use crate::msg::is_ros2_env;

use super::callbacks::RosCallbacks;
use super::config::Config;

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
pub(crate) fn collect_interface_files(pkg_path: PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut seen_bases: HashSet<String> = HashSet::new();

    // Interface subdirectories
    let subdirs = ["msg", "srv", "action"];

    for subdir in &subdirs {
        let dir_path = pkg_path.join(subdir);
        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            // First pass: collect all .idl files (priority)
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "idl")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    seen_bases.insert(format!("{}/{}", subdir, stem));
                    files.push(path);
                }
            }
        }

        // Second pass: collect native files (.msg/.srv/.action) only if no .idl exists
        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str())
                    && ext == *subdir
                {
                    // Native file extension matches subdirectory (e.g., .srv in srv/)
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let key = format!("{}/{}", subdir, stem);
                        if !seen_bases.contains(&key) {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }

    files.sort();
    files
}

/// Emits cargo link directives for a ROS2 package's rosidl libraries.
///
/// This function outputs the necessary cargo directives to link against the
/// type support and generator libraries for a specific ROS2 package. It uses
/// the provided [`Config`] to determine library search paths.
///
/// # Arguments
///
/// * `pkg` - The name of the ROS2 package (e.g., `"std_msgs"`, `"geometry_msgs"`)
///
/// # Emitted Directives
///
/// - `cargo:rustc-link-lib=<pkg>__rosidl_typesupport_c` - Type support library
/// - `cargo:rustc-link-lib=<pkg>__rosidl_generator_c` - Generator library
///
/// # Example
///
/// ```rust,ignore
/// use oxidros_build::msg::Config;
///
/// // Link libraries for std_msgs package
/// oxidros_build::msg::emit_ros_idl("std_msgs");
/// ```
pub(crate) fn emit_ros_idl(pkg: &str) {
    println!("cargo:rustc-link-lib={pkg}__rosidl_typesupport_c");
    println!("cargo:rustc-link-lib={pkg}__rosidl_generator_c");
}

/// Creates a base generator configured with the provided [`Config`].
///
/// This function sets up a [`Generator`] from the `ros2msg` crate with all the
/// necessary callbacks, search paths, and configuration options for generating
/// Rust types from ROS2 interface files.
///
/// # Arguments
///
/// * `config` - Configuration specifying packages, paths, and generation options
///
/// # Returns
///
/// - `Some(Generator)` - A configured generator ready to generate code
/// - `None` - If no interface files were found or paths are not configured
///
/// # Cargo Directives
///
/// This function emits several cargo directives:
/// - `cargo:rustc-cfg=feature="rcl"` - If ROS_DISTRO is set
/// - `cargo:rerun-if-env-changed` - For ROS_DISTRO, AMENT_PREFIX_PATH, CMAKE_PREFIX_PATH
/// - Link directives for each package with interface files
///
/// # Example
///
/// ```rust,ignore
/// use oxidros_build::msg::{Config, get_base_generator};
///
/// let config = Config::builder()
///     .packages(&["std_msgs", "geometry_msgs"])
///     .build();
///
/// if let Some(generator) = get_base_generator(&config) {
///     generator.generate().expect("Failed to generate");
/// }
/// ```
pub fn get_base_generator(config: &Config) -> Option<Generator> {
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
    // Get search paths from config (handles AMENT_PREFIX_PATH fallback automatically)
    let share_paths = config.get_share_paths();

    if share_paths.is_empty() {
        println!("cargo:warning=No ros2 message search paths found.");
        println!(
            "cargo:warning=Either source your ROS2 setup.bash (source /opt/ros/jazzy/setup.bash)"
        );
        println!("cargo:warning=or add paths using Config::builder().extra_search_path(...)");
        return None;
    }

    // Collect ALL interface files from ALL packages
    let mut all_files = Vec::new();
    let mut packages_found = Vec::new();
    let packages_filter = config.packages();

    for share_path in &share_paths {
        // List all directories in the share path (each is a package)
        if let Ok(entries) = std::fs::read_dir(share_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let pkg_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    // Skip ament-specific directories and non-package directories
                    if pkg_name.starts_with("ament_")
                        || pkg_name.starts_with("ros2")
                        || pkg_name == "cmake"
                        || pkg_name == "colcon-core"
                        || pkg_name.is_empty()
                    {
                        continue;
                    }

                    // Filter by requested packages if specified
                    if !packages_filter.is_empty()
                        && !packages_filter.iter().any(|p| p == &pkg_name)
                    {
                        continue;
                    }

                    let files = collect_interface_files(path);
                    if !files.is_empty() {
                        // Only emit link directives for packages that have interface files
                        if is_ros2_env() {
                            emit_ros_idl(&pkg_name);
                        }
                        packages_found.push(pkg_name.to_string());
                        all_files.extend(files);
                    }
                }
            }
        }
    }

    if all_files.is_empty() {
        println!("cargo:warning=No interface files found in search paths");
        return None;
    }
    if is_ros2_env() {
        config.print_packages_search_pathes();
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
    Some(
        Generator::new()
            .header(
                "// Auto-generated ROS2 message types\n// Generated by ros2msg from all ROS2 packages",
            )
            .derive_debug(true)
            .parse_callbacks(Box::new(RosCallbacks::new(
                config.uuid_path.clone(),
                config.primitive_path.clone(),
            )))
            .includes(all_files)
            .output_dir(&generated_dir)
            .emit_rerun_if_changed(true)
            .allowlist_recursively(true)
            .package_search_paths(share_paths),
    )
}

/// Generates Rust types from ROS2 interface files for the specified packages.
///
/// This is a convenience function that uses default configuration. For more
/// control, use [`Config::builder()`] and [`get_base_generator()`].
///
/// # Arguments
///
/// * `packages` - A slice of package names to generate types for. If empty,
///   types are generated for ALL packages found in the search paths.
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
/// - If no ROS2 installation is found, a warning is emitted and no code is generated.
pub fn generate_msgs(packages: &[&str]) {
    let config = Config::builder().packages(packages).build();
    generate_msgs_with_config(&config);
}

/// Generates Rust types from ROS2 interface files using the provided configuration.
///
/// This function provides full control over the generation process through
/// the [`Config`] struct.
///
/// # Arguments
///
/// * `config` - Configuration specifying packages, paths, and generation options
///
/// # Example
///
/// ```rust,ignore
/// use oxidros_build::msg::{Config, generate_msgs_with_config};
///
/// let config = Config::builder()
///     .packages(&["std_msgs", "geometry_msgs"])
///     .uuid_path("my_crate::unique_identifier_msgs")
///     .extra_search_path("/custom/ros2/share")
///     .build();
///
/// generate_msgs_with_config(&config);
/// ```
pub fn generate_msgs_with_config(config: &Config) {
    let Some(generator) = get_base_generator(config) else {
        return;
    };
    generator
        .generate()
        .expect("Failed to generate message types");
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
}
