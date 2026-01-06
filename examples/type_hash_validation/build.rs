//! Build script that automatically discovers and generates all ROS2 messages with TypeDescription support
//!
//! This uses ros2msg Generator with custom callbacks to automatically add:
//! 1. derive(TypeDescription) to all generated structs
//! 2. Custom trait implementations with proper type names
//! 3. Scans ROS_PATH/share for all .msg, .srv, and .action files

use heck::ToSnakeCase;
use ros2msg::generator::{Generator, ItemInfo, ParseCallbacks};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Custom callbacks to add TypeDescription derive and attributes
struct TypeDescCallbacks;

impl ParseCallbacks for TypeDescCallbacks {
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        vec![
            "ros2_types::TypeDescription".to_string(),
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
        ]
    }

    fn add_attributes(&self, info: &ItemInfo) -> Vec<String> {
        vec![format!(
            "#[ros2(package = \"{}\", interface_type = \"{}\")]",
            info.package(),
            info.interface_kind()
        )]
    }

    fn add_field_attributes(&self, field_info: &ros2msg::generator::FieldInfo) -> Vec<String> {
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
        let ros_type_name = field_info.ros_type_name();
        let rust_type = field_info.field_type();
        let is_sequence = ros_type_name.starts_with("sequence")
            || (field_info.capacity().is_some() && field_info.array_size().is_none())
            || (rust_type.starts_with("Vec<") && field_info.array_size().is_none());

        if is_sequence {
            ros2_parts.push("sequence".to_string());
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
            attrs.push("#[serde(with = \"serde_big_array::BigArray\")]".to_string());
        }
        attrs
    }
}

#[derive(Debug)]
struct DiscoveredType {
    file_path: String,
    package: String,
    interface_type: String, // "msg", "srv", "action"
    name: String,
}

/// Recursively scan a directory for .msg, .srv, and .action files
fn scan_ros_interfaces(
    share_dir: &Path,
) -> Result<Vec<DiscoveredType>, Box<dyn std::error::Error>> {
    let mut discovered = Vec::new();

    if !share_dir.exists() {
        return Ok(discovered);
    }

    // Iterate through package directories in share/
    for entry in fs::read_dir(share_dir)? {
        let entry = entry?;
        let package_path = entry.path();

        if !package_path.is_dir() {
            continue;
        }

        let package_name = package_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip common non-package directories
        if package_name.starts_with('.') {
            continue;
        }

        // Scan msg/, srv/, and action/ subdirectories
        for interface_type in &["msg", "srv", "action"] {
            let interface_dir = package_path.join(interface_type);

            if !interface_dir.exists() {
                continue;
            }

            // Find all interface files
            if let Ok(entries) = fs::read_dir(&interface_dir) {
                for file_entry in entries.flatten() {
                    let file_path = file_entry.path();

                    if let Some(ext) = file_path.extension()
                        && ext == *interface_type
                        && let Some(file_name) = file_path.file_stem().and_then(|n| n.to_str())
                    {
                        discovered.push(DiscoveredType {
                            file_path: file_path.to_string_lossy().to_string(),
                            package: package_name.clone(),
                            interface_type: interface_type.to_string(),
                            name: file_name.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(discovered)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=ROS_PATH");

    let ros_path = env::var("ROS_PATH").unwrap_or_else(|_| "/opt/ros/jazzy".to_string());
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    println!("cargo:info=Scanning ROS2 interfaces from: {}", ros_path);

    let share_dir = PathBuf::from(&ros_path).join("share");

    // Discover all interface files
    let mut discovered = scan_ros_interfaces(&share_dir)?;

    if discovered.is_empty() {
        println!(
            "cargo:warning=No ROS2 message files found at {}",
            share_dir.display()
        );
        println!("cargo:info=Set ROS_PATH environment variable to your ROS2 installation");
        return Ok(());
    }

    println!("cargo:info=Discovered {} interface files", discovered.len());

    // Organize by category for better logging
    let mut by_type: HashMap<String, Vec<&DiscoveredType>> = HashMap::new();
    for d in &discovered {
        by_type.entry(d.interface_type.clone()).or_default().push(d);
    }

    for (itype, items) in &by_type {
        println!("cargo:info=  {} {}: {}", items.len(), itype, items.len());
    }

    // Collect all file paths for generation
    let file_paths: Vec<&str> = discovered.iter().map(|d| d.file_path.as_str()).collect();

    // Generate with TypeDescription support
    match Generator::new()
        .header("// Auto-generated ROS2 messages with TypeDescription support")
        .derive_debug(true)
        .derive_clone(true)
        .derive_partialeq(true)
        .parse_callbacks(Box::new(TypeDescCallbacks))
        .includes(&file_paths)
        .allowlist_recursively(true) // Generate dependencies recursively
        .output_dir(&out_dir)
        .generate()
    {
        Ok(_) => {
            println!("cargo:info=Generated messages to: {}", out_dir.display());
        }
        Err(e) => {
            println!("cargo:warning=Generation failed: {}", e);
            println!("cargo:warning=Attempting to filter problematic files and retry...");

            // Try to generate files one by one to identify problematic ones
            let mut successful_paths = Vec::new();
            let mut failed = Vec::new();

            for d in &discovered {
                match Generator::new()
                    .header("// Auto-generated ROS2 messages with TypeDescription support")
                    .derive_debug(true)
                    .derive_clone(true)
                    .derive_partialeq(true)
                    .parse_callbacks(Box::new(TypeDescCallbacks))
                    .includes([d.file_path.as_str()])
                    .allowlist_recursively(false) // Don't recurse when testing individual files
                    .output_dir(&out_dir)
                    .generate()
                {
                    Ok(_) => {
                        successful_paths.push(d.file_path.clone());
                    }
                    Err(_) => {
                        failed.push(d);
                    }
                }
            }

            println!(
                "cargo:warning=Successfully generated: {} files",
                successful_paths.len()
            );
            println!("cargo:warning=Failed to generate: {} files", failed.len());

            if !failed.is_empty() {
                println!("cargo:warning=Failed files:");
                for f in &failed {
                    println!(
                        "cargo:warning=  - {}/{}/{}",
                        f.package, f.interface_type, f.name
                    );
                }
            }

            // Update discovered list to only include successful ones
            discovered.retain(|d| successful_paths.contains(&d.file_path));

            // Now regenerate all successful files together with proper recursion
            // Keep trying until we have a stable set (some files may depend on failed ones)
            if !discovered.is_empty() {
                let mut stable = false;
                let mut iteration = 0;

                while !stable && iteration < 10 {
                    iteration += 1;
                    let successful_file_paths: Vec<&str> =
                        discovered.iter().map(|d| d.file_path.as_str()).collect();

                    println!(
                        "cargo:warning=Regeneration attempt {} with {} files...",
                        iteration,
                        successful_file_paths.len()
                    );

                    match Generator::new()
                        .header("// Auto-generated ROS2 messages with TypeDescription support")
                        .raw_line("use ros2_types::*;")
                        .derive_debug(true)
                        .derive_clone(true)
                        .derive_partialeq(true)
                        .parse_callbacks(Box::new(TypeDescCallbacks))
                        .includes(&successful_file_paths)
                        .allowlist_recursively(true)
                        .output_dir(&out_dir)
                        .generate()
                    {
                        Ok(_) => {
                            println!("cargo:info=Successfully regenerated all working files");
                            stable = true;
                        }
                        Err(e) => {
                            println!(
                                "cargo:warning=Regeneration attempt {} failed: {}",
                                iteration, e
                            );

                            // The error is from the Generator itself, filter files
                            let before_count = discovered.len();
                            let mut new_successful = Vec::new();

                            for d in &discovered {
                                match Generator::new()
                                    .header("// Auto-generated ROS2 messages with TypeDescription support")
                                    .raw_line("use ros2_types::*;")
                                    .derive_debug(true)
                                    .derive_clone(true)
                                    .derive_partialeq(true)
                                    .parse_callbacks(Box::new(TypeDescCallbacks))
                                    .includes([d.file_path.as_str()])
                                    .allowlist_recursively(true)
                                    .output_dir(&out_dir)
                                    .generate()
                                {
                                    Ok(_) => {
                                        new_successful.push(d.file_path.clone());
                                    }
                                    Err(_) => {
                                        println!("cargo:warning=  Filtering out: {}/{}/{}", d.package, d.interface_type, d.name);
                                    }
                                }
                            }

                            discovered.retain(|d| new_successful.contains(&d.file_path));

                            if discovered.len() == before_count {
                                // No progress, might be a compilation issue, not generation
                                // Try compiling to check
                                println!(
                                    "cargo:warning=No generation failures found, issue may be in compilation"
                                );
                                println!(
                                    "cargo:warning=This is likely due to dependencies on failed types"
                                );
                                println!("cargo:warning=Stopping with {} files", discovered.len());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate a registry file that main.rs can use to test all types
    generate_test_registry(&out_dir, &discovered)?;

    Ok(())
}

/// Generate a Rust file containing metadata and test dispatcher for all discovered types
fn generate_test_registry(
    out_dir: &Path,
    discovered: &[DiscoveredType],
) -> Result<(), Box<dyn std::error::Error>> {
    let registry_path = out_dir.join("test_registry.rs");
    let mut file = fs::File::create(registry_path)?;

    writeln!(file, "// Auto-generated test registry")?;
    writeln!(
        file,
        "// This file contains metadata and test dispatch for all discovered ROS2 types\n"
    )?;

    writeln!(file, "#[allow(unused_imports)]")?;
    writeln!(file, "use crate::generated;")?;
    writeln!(file, "use crate::{{TestResult, test_type_impl}};\n")?;

    writeln!(file, "pub struct TypeEntry {{")?;
    writeln!(
        file,
        "    pub ros2_name: &'static str,  // e.g., \"std_msgs/msg/String\""
    )?;
    writeln!(file, "    pub package: &'static str,")?;
    writeln!(
        file,
        "    pub interface_type: &'static str,  // \"msg\", \"srv\", \"action\""
    )?;
    writeln!(file, "    pub name: &'static str,")?;
    writeln!(file, "}}\n")?;

    writeln!(file, "pub const ALL_TYPES: &[TypeEntry] = &[")?;

    for d in discovered {
        // For services, we'll register both Request and Response variants
        if d.interface_type == "srv" {
            writeln!(
                file,
                "    TypeEntry {{ ros2_name: \"{}/{}/{}_Request\", package: \"{}\", interface_type: \"{}\", name: \"{}\" }},",
                d.package, d.interface_type, d.name, d.package, d.interface_type, d.name
            )?;
            writeln!(
                file,
                "    TypeEntry {{ ros2_name: \"{}/{}/{}_Response\", package: \"{}\", interface_type: \"{}\", name: \"{}\" }},",
                d.package, d.interface_type, d.name, d.package, d.interface_type, d.name
            )?;
        } else if d.interface_type == "action" {
            // For actions, register Goal, Result, and Feedback variants
            writeln!(
                file,
                "    TypeEntry {{ ros2_name: \"{}/{}/{}_Goal\", package: \"{}\", interface_type: \"{}\", name: \"{}\" }},",
                d.package, d.interface_type, d.name, d.package, d.interface_type, d.name
            )?;
            writeln!(
                file,
                "    TypeEntry {{ ros2_name: \"{}/{}/{}_Result\", package: \"{}\", interface_type: \"{}\", name: \"{}\" }},",
                d.package, d.interface_type, d.name, d.package, d.interface_type, d.name
            )?;
            writeln!(
                file,
                "    TypeEntry {{ ros2_name: \"{}/{}/{}_Feedback\", package: \"{}\", interface_type: \"{}\", name: \"{}\" }},",
                d.package, d.interface_type, d.name, d.package, d.interface_type, d.name
            )?;
        } else {
            writeln!(
                file,
                "    TypeEntry {{ ros2_name: \"{}/{}/{}\", package: \"{}\", interface_type: \"{}\", name: \"{}\" }},",
                d.package, d.interface_type, d.name, d.package, d.interface_type, d.name
            )?;
        }
    }

    writeln!(file, "];\n")?;

    // Build a map of actual module names from the generated code
    let mut module_names = std::collections::HashMap::new();
    for d in discovered {
        let mod_rs = out_dir
            .join(&d.package)
            .join(&d.interface_type)
            .join("mod.rs");
        if let Ok(content) = fs::read_to_string(&mod_rs) {
            // Parse module names from "pub mod <name>;"
            for line in content.lines() {
                if let Some(mod_name) = line
                    .trim()
                    .strip_prefix("pub mod ")
                    .and_then(|s| s.strip_suffix(';'))
                {
                    // Try to find which discovered type this module corresponds to
                    for check_d in discovered {
                        if check_d.package == d.package
                            && check_d.interface_type == d.interface_type
                        {
                            let test_snake = check_d.name.to_snake_case();
                            if mod_name == test_snake {
                                let key = format!(
                                    "{}/{}/{}",
                                    check_d.package, check_d.interface_type, check_d.name
                                );
                                module_names.insert(key, mod_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate the automatic dispatch function
    writeln!(
        file,
        "pub fn test_type_by_name(ros2_name: &str) -> TestResult {{"
    )?;
    writeln!(file, "    let mut total = 0;")?;
    writeln!(file, "    let mut matches = 0;")?;
    writeln!(file, "    let mut mismatches = 0;")?;
    writeln!(file, "    let mut errors = 0;\n")?;

    writeln!(file, "    match ros2_name {{")?;

    // Generate match arms for all discovered types
    for d in discovered {
        let key = format!("{}/{}/{}", d.package, d.interface_type, d.name);
        let module_name = module_names
            .get(&key)
            .map(|s| s.as_str())
            .unwrap_or_else(|| {
                println!(
                    "cargo:warning=Could not find module name for {}, using fallback",
                    key
                );
                ""
            });

        if module_name.is_empty() {
            continue; // Skip if we couldn't find the module
        }

        if d.interface_type == "srv" {
            // Service Request
            writeln!(
                file,
                "        \"{}/{}/{}_Request\" => test_type_impl::<generated::{}::{}::{}::{}_Request>(ros2_name, &mut total, &mut matches, &mut mismatches, &mut errors),",
                d.package,
                d.interface_type,
                d.name,
                d.package,
                d.interface_type,
                module_name,
                d.name
            )?;
            // Service Response
            writeln!(
                file,
                "        \"{}/{}/{}_Response\" => test_type_impl::<generated::{}::{}::{}::{}_Response>(ros2_name, &mut total, &mut matches, &mut mismatches, &mut errors),",
                d.package,
                d.interface_type,
                d.name,
                d.package,
                d.interface_type,
                module_name,
                d.name
            )?;
        } else if d.interface_type == "action" {
            // Action Goal
            writeln!(
                file,
                "        \"{}/{}/{}_Goal\" => test_type_impl::<generated::{}::{}::{}::{}_Goal>(ros2_name, &mut total, &mut matches, &mut mismatches, &mut errors),",
                d.package,
                d.interface_type,
                d.name,
                d.package,
                d.interface_type,
                module_name,
                d.name
            )?;
            // Action Result
            writeln!(
                file,
                "        \"{}/{}/{}_Result\" => test_type_impl::<generated::{}::{}::{}::{}_Result>(ros2_name, &mut total, &mut matches, &mut mismatches, &mut errors),",
                d.package,
                d.interface_type,
                d.name,
                d.package,
                d.interface_type,
                module_name,
                d.name
            )?;
            // Action Feedback
            writeln!(
                file,
                "        \"{}/{}/{}_Feedback\" => test_type_impl::<generated::{}::{}::{}::{}_Feedback>(ros2_name, &mut total, &mut matches, &mut mismatches, &mut errors),",
                d.package,
                d.interface_type,
                d.name,
                d.package,
                d.interface_type,
                module_name,
                d.name
            )?;
        } else {
            // Regular message
            writeln!(
                file,
                "        \"{}/{}/{}\" => test_type_impl::<generated::{}::{}::{}::{}>(ros2_name, &mut total, &mut matches, &mut mismatches, &mut errors),",
                d.package,
                d.interface_type,
                d.name,
                d.package,
                d.interface_type,
                module_name,
                d.name
            )?;
        }
    }

    writeln!(file, "        _ => {{")?;
    writeln!(
        file,
        "            println!(\"⊘ Skipped (not in generated code)\");"
    )?;
    writeln!(file, "            TestResult::Skipped")?;
    writeln!(file, "        }}")?;
    writeln!(file, "    }}")?;
    writeln!(file, "}}")?;

    println!(
        "cargo:info=Generated test registry with {} entries",
        discovered.len()
    );

    Ok(())
}
