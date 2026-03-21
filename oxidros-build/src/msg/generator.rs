//! ROS2 message code generation.
//!
//! This module provides the core generation logic for creating Rust types
//! from ROS2 interface definition files.

use ros2msg::generator::Generator;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use super::callbacks::RosCallbacks;
use super::config::Config;

/// Represents the availability state of a ROS2 installation.
///
/// This enum is used to determine how message generation should proceed:
/// - Generate with full linking (sourced ROS)
/// - Generate without linking (common install detected)
/// - Skip generation and use pre-committed files (no ROS)
#[derive(Debug, Clone)]
pub enum RosAvailability {
    /// ROS2 environment is sourced (AMENT_PREFIX_PATH is set).
    /// Full generation with library linking is possible.
    Sourced {
        /// Share paths from AMENT_PREFIX_PATH
        share_paths: Vec<PathBuf>,
    },
    /// A common ROS2 installation was found but not sourced.
    /// Generation is possible but linking may not work.
    CommonInstall {
        /// Share paths from common installation locations
        share_paths: Vec<PathBuf>,
    },
    /// No ROS2 installation detected.
    /// Pre-generated (gitted) files should be used.
    NotAvailable,
}

impl RosAvailability {
    /// Returns true if ROS2 is available (either sourced or common install).
    pub fn is_available(&self) -> bool {
        !matches!(self, RosAvailability::NotAvailable)
    }

    /// Returns true if ROS2 is fully sourced with AMENT_PREFIX_PATH.
    pub fn is_sourced(&self) -> bool {
        matches!(self, RosAvailability::Sourced { .. })
    }

    /// Returns the share paths if ROS2 is available.
    pub fn share_paths(&self) -> Option<&[PathBuf]> {
        match self {
            RosAvailability::Sourced { share_paths } => Some(share_paths),
            RosAvailability::CommonInstall { share_paths } => Some(share_paths),
            RosAvailability::NotAvailable => None,
        }
    }
}

/// Detects the availability of a ROS2 installation.
///
/// This function checks for ROS2 in the following order:
///
/// 1. **Sourced ROS2**: Checks if `AMENT_PREFIX_PATH` is set (standard sourced environment)
/// 2. **Common Install**: Checks common installation paths (`/opt/ros/jazzy`, etc.)
/// 3. **Not Available**: No ROS2 installation found
///
/// # Arguments
///
/// * `config` - Configuration that may contain extra search paths
///
/// # Returns
///
/// A [`RosAvailability`] enum indicating the detection result.
///
/// # Example
///
/// ```rust,ignore
/// use oxidros_build::msg::{Config, detect_ros_availability, RosAvailability};
///
/// let config = Config::builder().build();
/// match detect_ros_availability(&config) {
///     RosAvailability::Sourced { share_paths } => {
///         println!("ROS2 sourced, generating with linking");
///     }
///     RosAvailability::CommonInstall { share_paths } => {
///         println!("ROS2 found but not sourced, generating without linking");
///     }
///     RosAvailability::NotAvailable => {
///         println!("No ROS2 found, using pre-generated files");
///     }
/// }
/// ```
pub fn detect_ros_availability(config: &Config) -> RosAvailability {
    let extra_share_paths: Vec<PathBuf> = config
        .extra_search_paths
        .iter()
        .filter(|p| p.exists())
        .map(|p| {
            let share = p.join("share");
            if share.exists() { share } else { p.clone() }
        })
        .collect();

    // 1. Check AMENT_PREFIX_PATH (sourced ROS2)
    if let Some(ament_paths) = Config::get_ament_prefix_paths() {
        let mut share_paths: Vec<PathBuf> = ament_paths
            .into_iter()
            .map(|p| {
                let share = p.join("share");
                if share.exists() { share } else { p }
            })
            .collect();
        share_paths.extend(extra_share_paths.clone());
        if !share_paths.is_empty() {
            return RosAvailability::Sourced { share_paths };
        }
    }

    // 2. Check common installation paths
    let default_paths = Config::get_default_ros2_paths();
    if !default_paths.is_empty() {
        let mut share_paths: Vec<PathBuf> = default_paths
            .into_iter()
            .map(|p| {
                let share = p.join("share");
                if share.exists() { share } else { p }
            })
            .collect();
        share_paths.extend(extra_share_paths.clone());
        if !share_paths.is_empty() {
            return RosAvailability::CommonInstall { share_paths };
        }
    }
    // 3. Check extra search paths from config
    if !extra_share_paths.is_empty() {
        return RosAvailability::CommonInstall {
            share_paths: extra_share_paths,
        };
    }

    // 4. Nothing found
    RosAvailability::NotAvailable
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
/// - `None` - If no ROS2 installation is detected ([`RosAvailability::NotAvailable`])
///
/// # ROS2 Detection
///
/// The function uses [`detect_ros_availability`] to determine how to proceed:
///
/// - **Sourced**: Full generation with library linking directives
/// - **CommonInstall**: Generation without linking (for pre-generating files)
/// - **NotAvailable**: Returns `None` - use pre-committed gitted files
///
/// # Cargo Directives
///
/// This function emits several cargo directives:
/// - `cargo:rerun-if-env-changed` - For ROS_DISTRO, AMENT_PREFIX_PATH, CMAKE_PREFIX_PATH
/// - Link directives for each package (only when ROS is sourced)
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
/// } else {
///     // No ROS2 available - use pre-generated files
/// }
/// ```
pub fn get_base_generator(config: &Config) -> Option<Generator> {
    // Rerun if ROS2 environment changes
    println!("cargo:rerun-if-env-changed=ROS_DISTRO");
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
    if cfg!(target_os = "windows") {
        println!("cargo:rerun-if-env-changed=CMAKE_PREFIX_PATH");
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    // Detect ROS2 availability
    let availability = detect_ros_availability(config);
    let is_sourced = availability.is_sourced();

    let share_paths = match availability {
        RosAvailability::Sourced { share_paths } => {
            println!("cargo:info=ROS2 environment sourced, generating with linking");
            share_paths
        }
        RosAvailability::CommonInstall { share_paths } => {
            println!("cargo:warning=ROS2 installation found but not sourced");
            println!("cargo:warning=Generating messages without library linking");
            println!("cargo:warning=Source your ROS2 environment for full functionality");
            share_paths
        }
        RosAvailability::NotAvailable => {
            println!("cargo:warning=No ROS2 installation detected");
            println!("cargo:warning=Using pre-generated message files (if available)");
            return None;
        }
    };

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

                    // Filter by requested packages if specified
                    if !packages_filter.is_empty()
                        && !packages_filter.iter().any(|p| p == &pkg_name)
                    {
                        continue;
                    }

                    let files = collect_interface_files(path);
                    if !files.is_empty() {
                        // Only emit link directives when ROS is fully sourced
                        if is_sourced {
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

    println!(
        "cargo:info=Found {} interface files from {} packages",
        all_files.len(),
        packages_found.len()
    );

    // Print some package names for visibility
    let preview: Vec<_> = packages_found.iter().take(10).collect();
    println!("cargo:info=Packages include: {:?}...", preview);

    // Only print library search paths when sourced
    if is_sourced {
        config.print_packages_search_pathes();
    }

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
