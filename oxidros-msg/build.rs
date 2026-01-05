//! Build script for oxidros-msg
//!
//! Uses ros2msg to generate ROS2 message types with ros2-type-hash-derive for FFI support.

use std::env;
use std::path::Path;

use ros2msg::generator::{
    Generator, InterfaceKind, ItemInfo, ModuleInfo, ModuleLevel, ParseCallbacks,
};

/// Callbacks for generating ROS2 FFI code using ros2-type-hash-derive
struct Ros2FfiCallbacks {
    /// Path prefix for unique_identifier_msgs (for action types)
    uuid_path: String,
}

impl Ros2FfiCallbacks {
    fn new(uuid_path: &str) -> Self {
        Self {
            uuid_path: uuid_path.to_string(),
        }
    }
}

impl ParseCallbacks for Ros2FfiCallbacks {
    /// Add the ros2 attribute with package and interface type
    fn add_attributes(&self, info: &ItemInfo) -> Vec<String> {
        let package = info.package();
        let interface_type = match info.interface_kind() {
            InterfaceKind::Message => "msg",
            InterfaceKind::Service => "srv",
            InterfaceKind::Action => "action",
        };
        // For action types, add uuid_path so the derive macro knows how to find unique_identifier_msgs
        if matches!(info.interface_kind(), InterfaceKind::Action) {
            vec![format!(
                "#[ros2(package = \"{}\", interface_type = \"{}\", uuid_path = \"{}\")]",
                package, interface_type, self.uuid_path
            )]
        } else {
            vec![format!(
                "#[ros2(package = \"{}\", interface_type = \"{}\")]",
                package, interface_type
            )]
        }
    }

    /// Add derives for ROS2 types including Ros2Msg from ros2-type-hash-derive
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        vec!["ros2_type_hash::Ros2Msg".to_string()]
    }

    /// Custom type mapping for ROS2 FFI types - strings
    fn string_type(&self, max_size: Option<u32>) -> Option<String> {
        Some(format!("crate::msg::RosString<{}>", max_size.unwrap_or(0)))
    }

    /// Custom type mapping for ROS2 FFI types - wide strings
    fn wstring_type(&self, max_size: Option<u32>) -> Option<String> {
        Some(format!("crate::msg::RosWString<{}>", max_size.unwrap_or(0)))
    }

    /// Custom type mapping for sequences
    fn sequence_type(&self, element_type: &str, max_size: Option<u32>) -> Option<String> {
        let size = max_size.unwrap_or(0);
        match element_type {
            "bool" => Some(format!("crate::msg::BoolSeq<{}>", size)),
            "u8" => Some(format!("crate::msg::U8Seq<{}>", size)),
            "i8" => Some(format!("crate::msg::I8Seq<{}>", size)),
            "u16" => Some(format!("crate::msg::U16Seq<{}>", size)),
            "i16" => Some(format!("crate::msg::I16Seq<{}>", size)),
            "u32" => Some(format!("crate::msg::U32Seq<{}>", size)),
            "i32" => Some(format!("crate::msg::I32Seq<{}>", size)),
            "u64" => Some(format!("crate::msg::U64Seq<{}>", size)),
            "i64" => Some(format!("crate::msg::I64Seq<{}>", size)),
            "f32" => Some(format!("crate::msg::F32Seq<{}>", size)),
            "f64" => Some(format!("crate::msg::F64Seq<{}>", size)),
            // RosStringSeq<STRLEN, SEQLEN>
            s if s.starts_with("crate::msg::RosString<") => {
                let str_len = s
                    .strip_prefix("crate::msg::RosString<")
                    .and_then(|s| s.strip_suffix(">"))
                    .unwrap_or("0");
                Some(format!("crate::msg::RosStringSeq<{}, {}>", str_len, size))
            }
            // RosWStringSeq<STRLEN, SEQLEN>
            s if s.starts_with("crate::msg::RosWString<") => {
                let str_len = s
                    .strip_prefix("crate::msg::RosWString<")
                    .and_then(|s| s.strip_suffix(">"))
                    .unwrap_or("0");
                Some(format!("crate::msg::RosWStringSeq<{}, {}>", str_len, size))
            }
            // For custom message types, use the generated XxxSeq<N> type
            // The Ros2Msg derive macro generates these Seq types automatically
            _ => Some(format!("{}Seq<{}>", element_type, size)),
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

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    oxidros_build::ros2_env_var_changed();

    // Define ROS2 package groups
    let common_interfaces_deps = [
        "actionlib_msgs",
        "diagnostic_msgs",
        "example_interfaces",
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

    // Get AMENT_PREFIX_PATH for package search
    let ament_paths: Vec<_> = env::var("AMENT_PREFIX_PATH")
        .unwrap_or_default()
        .split(':')
        .filter(|p| !p.is_empty())
        .map(|p| format!("{}/share", p))
        .collect();

    // Helper to collect .msg/.srv/.action files from a package
    fn collect_files(ament_paths: &[String], packages: &[&str]) -> Vec<String> {
        let mut files = Vec::new();
        for pkg in packages {
            for ament_path in ament_paths {
                let pkg_path = format!("{}/{}", ament_path, pkg);
                for subdir in &["msg", "srv", "action"] {
                    let dir_path = format!("{}/{}", pkg_path, subdir);
                    if let Ok(entries) = std::fs::read_dir(&dir_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if let Some(ext) = path.extension() {
                                if ext == *subdir {
                                    files.push(path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        files
    }

    // Generate common_interfaces
    let common_interfaces_dir = out_path.join("common_interfaces");
    std::fs::create_dir_all(&common_interfaces_dir).unwrap();

    let common_files = collect_files(&ament_paths, &common_interfaces_deps);

    Generator::new()
        .header("// Auto-generated ROS2 message bindings - do not edit")
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2FfiCallbacks::new("crate::ros2msg")))
        .includes(common_files)
        .output_dir(&common_interfaces_dir)
        .emit_rerun_if_changed(true)
        .allowlist_recursively(true)
        .package_search_paths(ament_paths.iter().map(|s| s.as_str()))
        .generate()
        .expect("Failed to generate common_interfaces");

    // Generate interfaces
    let interfaces_dir = out_path.join("interfaces");
    std::fs::create_dir_all(&interfaces_dir).unwrap();

    let interface_files = collect_files(&ament_paths, &interface_deps);

    Generator::new()
        .header("// Auto-generated ROS2 message bindings - do not edit")
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2FfiCallbacks::new("crate::ros2msg")))
        .includes(interface_files)
        .output_dir(&interfaces_dir)
        .emit_rerun_if_changed(true)
        .allowlist_recursively(true)
        .package_search_paths(ament_paths.iter().map(|s| s.as_str()))
        .generate()
        .expect("Failed to generate interfaces");

    // Generate ros2msg
    let ros2msg_dir = out_path.join("ros2msg");
    std::fs::create_dir_all(&ros2msg_dir).unwrap();

    let ros2msg_files = collect_files(&ament_paths, &ros2msg_deps);

    Generator::new()
        .header("// Auto-generated ROS2 message bindings - do not edit")
        .derive_debug(true)
        .parse_callbacks(Box::new(Ros2FfiCallbacks::new("crate::ros2msg")))
        .includes(ros2msg_files)
        .output_dir(&ros2msg_dir)
        .emit_rerun_if_changed(true)
        .allowlist_recursively(true)
        .package_search_paths(ament_paths.iter().map(|s| s.as_str()))
        .generate()
        .expect("Failed to generate ros2msg");

    // Generate runtime_c.rs using bindgen
    oxidros_build::generate_runtime_c(out_path);

    // Emit library search paths for ROS2 libraries
    for ament_path in &ament_paths {
        // ament_paths are like /opt/ros/jazzy/share, we need /opt/ros/jazzy/lib
        let lib_path = Path::new(ament_path).parent().unwrap().join("lib");
        if lib_path.exists() {
            println!("cargo:rustc-link-search={}", lib_path.display());
        }
    }

    // Link ROS2 C libraries
    oxidros_build::link_msg_ros2_libs();

    // Emit additional link directives for the packages
    for pkg in common_interfaces_deps
        .iter()
        .chain(interface_deps.iter())
        .chain(ros2msg_deps.iter())
    {
        println!("cargo:rustc-link-lib={}__rosidl_typesupport_c", pkg);
        println!("cargo:rustc-link-lib={}__rosidl_generator_c", pkg);
    }
}
