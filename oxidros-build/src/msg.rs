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
    FieldInfo, Generator, InterfaceKind, ItemInfo, ModuleInfo, ModuleLevel, ParseCallbacks,
};
use std::env;
use std::path::{Path, PathBuf};

use crate::get_paths_from_env;
/// Callbacks for generating ROS2 FFI code using ros2-types-derive
#[derive(Debug, Default)]
pub struct RosCallbacks {
    /// Path prefix for unique_identifier_msgs (for action types)
    uuid_path: Option<String>,
    primitive_path: Option<String>,
}

impl RosCallbacks {
    pub fn new(uuid_path: &str, primitive_path: &str) -> Self {
        Self {
            uuid_path: Some(uuid_path.to_string()),
            primitive_path: Some(primitive_path.to_string()),
        }
    }

    fn primitive_path(&self) -> &str {
        self.primitive_path
            .as_ref()
            .map_or("oxidros_msg", |v| v.as_str())
    }
}

impl ParseCallbacks for RosCallbacks {
    /// Add the ros2 attribute with package and interface type
    fn add_attributes(&self, info: &ItemInfo) -> Vec<String> {
        let package = info.package();
        let interface_type = match info.interface_kind() {
            InterfaceKind::Message => "msg",
            InterfaceKind::Service => "srv",
            InterfaceKind::Action => "action",
        };
        // For action types, add uuid_path so the derive macro knows how to find unique_identifier_msgs
        let mut attributes = if matches!(info.interface_kind(), InterfaceKind::Action)
            && let Some(uuid_path) = self.uuid_path.as_ref()
        {
            vec![format!(
                "#[ros2(package = \"{}\", interface_type = \"{}\", uuid_path = \"{}\")]",
                package, interface_type, uuid_path
            )]
        } else {
            vec![format!(
                "#[ros2(package = \"{}\", interface_type = \"{}\")]",
                package, interface_type
            )]
        };
        attributes.push("#[cfg_attr(not(feature = \"rcl\"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]".to_string());
        attributes.push(
            "#[cfg_attr(not(feature = \"rcl\"), serde(crate = \"ros2_types::serde\"))]".to_string(),
        );
        attributes
    }

    /// Adds BigArray attribute for fields with large arrays (> 32 elements).
    ///
    /// serde only supports arrays up to 32 elements by default. For larger arrays,
    /// we use `serde_big_array::BigArray` which is re-exported as `ros2_types::BigArray`.
    fn add_field_attributes(&self, field_info: &FieldInfo) -> Vec<String> {
        let mut attrs = Vec::new();

        // Build #[ros2(...)] attributes for type hash metadata
        let mut ros2_parts = Vec::new();

        // Add type override if present
        if let Some(type_override) = field_info.ros2_type_override() {
            ros2_parts.push(format!("ros2_type = \"{}\"", type_override));
        }

        // If the field is a sequence (not a fixed-size array), mark it explicitly
        // so derives know. Fixed-size arrays have `array_size()` set and are
        // represented as arrays in ROS2, not sequences.
        //
        // Detection methods:
        // 1. ROS type name starts with "sequence" (explicit IDL sequence)
        // 2. Has capacity but no array_size (bounded sequence)
        // 3. Rust field type is Vec<...> without array_size (unbounded sequence)
        // 4. Rust field type contains "Seq<" (custom sequence types like BoolSeq<0>, GoalStatusSeq<0>)
        let ros_type_name = field_info.ros_type_name();
        let rust_type = field_info.field_type();
        let is_sequence = ros_type_name.starts_with("sequence")
            || (field_info.capacity().is_some() && field_info.array_size().is_none())
            || (rust_type.starts_with("Vec<") && field_info.array_size().is_none())
            || rust_type.contains("Seq<");

        if is_sequence {
            ros2_parts.push("sequence".to_string());
        }

        // Detect string types (RosString<N>, RosWString<N>)
        let is_string = rust_type.contains("RosString<");
        let is_wstring = rust_type.contains("RosWString<");

        if is_string {
            ros2_parts.push("string".to_string());
        }
        if is_wstring {
            ros2_parts.push("wstring".to_string());
        }

        // Add capacity if present
        if let Some(capacity) = field_info.capacity() {
            ros2_parts.push(format!("capacity = {}", capacity));
        }

        // Add default value if present
        if let Some(default_value) = field_info.default_value() {
            ros2_parts.push(format!("default = \"{}\"", default_value));
        }

        if !ros2_parts.is_empty() {
            attrs.push(format!("#[ros2({})]", ros2_parts.join(", ")));
        }

        // Add serde_big_array attribute for large fixed-size arrays (> 32 elements)
        if let Some(size) = field_info.array_size()
            && size > 32
        {
            attrs.push(
                "#[cfg_attr(not(feature = \"rcl\"), serde(with = \"ros2_types::BigArray\"))]"
                    .to_string(),
            );
        }
        attrs
    }

    /// Add derives for ROS2 types including Ros2Msg from ros2-types-derive
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        vec![
            "ros2_types::Ros2Msg".to_string(),
            "ros2_types::TypeDescription".to_string(),
        ]
    }

    /// Custom type mapping for ROS2 FFI types - strings
    fn string_type(&self, max_size: Option<u32>) -> Option<String> {
        Some(format!(
            "{}::msg::RosString<{}>",
            self.primitive_path(),
            max_size.unwrap_or(0)
        ))
    }

    /// Custom type mapping for ROS2 FFI types - wide strings
    fn wstring_type(&self, max_size: Option<u32>) -> Option<String> {
        Some(format!(
            "{}::msg::RosWString<{}>",
            self.primitive_path(),
            max_size.unwrap_or(0)
        ))
    }

    /// Custom type mapping for sequences
    fn sequence_type(&self, element_type: &str, max_size: Option<u32>) -> Option<String> {
        let size = max_size.unwrap_or(0);
        let path = self.primitive_path();
        match element_type {
            "bool" => Some(format!("{path}::msg::BoolSeq<{size}>")),
            "u8" => Some(format!("{path}::msg::U8Seq<{size}>")),
            "i8" => Some(format!("{path}::msg::I8Seq<{size}>")),
            "u16" => Some(format!("{path}::msg::U16Seq<{size}>")),
            "i16" => Some(format!("{path}::msg::I16Seq<{size}>")),
            "u32" => Some(format!("{path}::msg::U32Seq<{size}>")),
            "i32" => Some(format!("{path}::msg::I32Seq<{size}>")),
            "u64" => Some(format!("{path}::msg::U64Seq<{size}>")),
            "i64" => Some(format!("{path}::msg::I64Seq<{size}>")),
            "f32" => Some(format!("{path}::msg::F32Seq<{size}>")),
            "f64" => Some(format!("{path}::msg::F64Seq<{size}>")),
            s => {
                // Check for RosString sequences
                let ros_string_prefix = format!("{}::msg::RosString<", path);
                if let Some(rest) = s.strip_prefix(&ros_string_prefix) {
                    let str_len = rest.strip_suffix(">").unwrap_or("0");
                    return Some(format!("{path}::msg::RosStringSeq<{str_len}, {size}>"));
                }
                // Check for RosWString sequences
                let ros_wstring_prefix = format!("{}::msg::RosWString<", path);
                if let Some(rest) = s.strip_prefix(&ros_wstring_prefix) {
                    let str_len = rest.strip_suffix(">").unwrap_or("0");
                    return Some(format!("{path}::msg::RosWStringSeq<{str_len}, {size}>"));
                }
                // For custom message types, use the generated XxxSeq<N> type
                // The Ros2Msg derive macro generates these Seq types automatically
                Some(format!("{element_type}Seq<{size}>"))
            }
        }
    }

    /// Add re-exports after type modules
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
fn collect_interface_files(pkg_path: PathBuf) -> Vec<PathBuf> {
    use std::collections::HashSet;

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

pub fn get_base_generator(
    packages: &[&str],
    uuid_path: Option<String>,
    primitive_path: Option<String>,
) -> Option<Generator> {
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
        return None;
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

                    if !packages.is_empty() && !packages.contains(&pkg_name.as_str()) {
                        continue;
                    }
                    let files = collect_interface_files(path);
                    if !files.is_empty() {
                        // Only emit link directives for packages that have interface files
                        emit_ros_idl(&pkg_name);
                        packages_found.push(pkg_name.to_string());
                        all_files.extend(files);
                    }
                }
            }
        }
    }

    if all_files.is_empty() {
        println!("cargo:warning=No interface files found in AMENT_PREFIX_PATH");
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

    // Create output directory
    let generated_dir = out_path.join("generated");
    std::fs::create_dir_all(&generated_dir).expect("Failed to create output directory");

    // Generate Rust code from ALL interface files
    // Note: We don't use derive_default(true) because Ros2Msg derive macro
    // generates its own Default impl that properly handles FFI types
    Some(Generator::new()
        .header(
            "// Auto-generated ROS2 message types\n// Generated by ros2msg from all ROS2 packages",
        )
        .derive_debug(true)
        .parse_callbacks(Box::new(RosCallbacks {
            uuid_path,
            primitive_path,
        }))
        .includes(all_files)
        .output_dir(&generated_dir)
        .emit_rerun_if_changed(true)
        .allowlist_recursively(true)
        .package_search_paths(ament_paths))
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
    let Some(generator) = get_base_generator(packages, None, None) else {
        return;
    };
    generator
        .generate()
        .expect("Failed to generate message types");
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
}
