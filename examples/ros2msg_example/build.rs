//! Build script demonstrating how to use the ros2msg generator
//!
//! This example shows how to generate Rust types from ROS2 interface files
//! (.msg, .srv, .action, .idl) using the ros2msg crate with ros2-type-hash derive macros.
//!
//! It generates types from ALL packages in AMENT_PREFIX_PATH to verify
//! the generator handles the complete ROS2 interface set.

use ros2msg::generator::{
    Generator, InterfaceKind, ItemInfo, ModuleInfo, ModuleLevel, ParseCallbacks,
};
use std::env;
use std::path::Path;

/// Custom callbacks for generating ROS2 FFI-compatible types
///
/// This demonstrates how to customize the code generation to produce
/// types that work with the ros2-type-hash derive macros.
struct ExampleCallbacks;

impl ParseCallbacks for ExampleCallbacks {
    /// Add the ros2 attribute required by the Ros2Msg derive macro
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

    /// Add the Ros2Msg derive to generate type support code
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        vec!["ros2_type_hash::Ros2Msg".to_string()]
    }

    /// Re-export types from their modules for convenience
    fn post_module(&self, info: &ModuleInfo) -> Option<String> {
        match info.module_level() {
            ModuleLevel::Type(_) => Some(format!("pub use {}::*;\n", info.module_name())),
            _ => None,
        }
    }
}

/// Collect all interface files (.msg, .srv, .action, .idl) from a package directory
/// Prefers .msg/.srv/.action files over .idl files to avoid duplicates.
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

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    // Get AMENT_PREFIX_PATH for finding ROS2 packages
    // This is set when you source /opt/ros/<distro>/setup.bash
    let ament_paths: Vec<String> = env::var("AMENT_PREFIX_PATH")
        .unwrap_or_default()
        .split(':')
        .filter(|p| !p.is_empty())
        .map(|p| format!("{}/share", p))
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
                    let files = collect_interface_files(&pkg_path);

                    if !files.is_empty() {
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
        "cargo:warning=Found {} interface files from {} packages",
        all_files.len(),
        packages_found.len()
    );

    // Print some package names for visibility
    let preview: Vec<_> = packages_found.iter().take(10).collect();
    println!("cargo:warning=Packages include: {:?}...", preview);

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
        .package_search_paths(ament_paths.iter().map(|s| s.as_str()))
        .generate()
        .expect("Failed to generate message types");

    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
}
