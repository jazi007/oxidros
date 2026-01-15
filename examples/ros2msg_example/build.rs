use heck::ToSnakeCase;
use oxidros_build::msg::Config;
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

/// Information about a discovered ROS2 interface file
#[derive(Debug, Clone)]
struct DiscoveredType {
    package: String,
    interface_type: String, // "msg", "srv", "action"
    name: String,
    json_path: Option<PathBuf>,
}

/// Scan a share directory for interface files with corresponding JSON files
fn scan_for_types_with_json(share_dir: &PathBuf) -> Vec<DiscoveredType> {
    let mut discovered = Vec::new();

    if !share_dir.exists() {
        return discovered;
    }

    // Iterate through package directories in share/
    let entries = match fs::read_dir(share_dir) {
        Ok(e) => e,
        Err(_) => return discovered,
    };

    for entry in entries.flatten() {
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
            if let Ok(file_entries) = fs::read_dir(&interface_dir) {
                for file_entry in file_entries.flatten() {
                    let file_path = file_entry.path();

                    if let Some(ext) = file_path.extension()
                        && let Some(file_name) = file_path.file_stem().and_then(|n| n.to_str())
                    {
                        let ext_str = ext.to_str().unwrap_or("");
                        // Look for .msg, .srv, .action, or .idl files
                        if ext_str == *interface_type || ext_str == "idl" {
                            // Check if corresponding .json file exists
                            let json_path = interface_dir.join(format!("{}.json", file_name));
                            let has_json = json_path.exists();

                            discovered.push(DiscoveredType {
                                package: package_name.clone(),
                                interface_type: interface_type.to_string(),
                                name: file_name.to_string(),
                                json_path: if has_json { Some(json_path) } else { None },
                            });
                        }
                    }
                }
            }
        }
    }

    discovered
}

/// Scan the generated output directory to find what types were actually generated
fn scan_generated_types(generated_dir: &PathBuf) -> HashSet<(String, String, String)> {
    let mut generated = HashSet::new();

    if !generated_dir.exists() {
        return generated;
    }

    // Iterate through package directories
    let entries = match fs::read_dir(generated_dir) {
        Ok(e) => e,
        Err(_) => return generated,
    };

    for entry in entries.flatten() {
        let package_path = entry.path();

        if !package_path.is_dir() {
            continue;
        }

        let package_name = package_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip mod.rs
        if package_name == "mod.rs" {
            continue;
        }

        // Scan msg/, srv/, and action/ subdirectories
        for interface_type in &["msg", "srv", "action"] {
            let interface_dir = package_path.join(interface_type);

            if !interface_dir.exists() {
                continue;
            }

            // Find all .rs files (each represents a generated type)
            if let Ok(file_entries) = fs::read_dir(&interface_dir) {
                for file_entry in file_entries.flatten() {
                    let file_path = file_entry.path();

                    if let Some(ext) = file_path.extension()
                        && ext == "rs"
                        && let Some(file_name) = file_path.file_stem().and_then(|n| n.to_str())
                    {
                        // Skip mod.rs
                        if file_name != "mod" {
                            // Convert snake_case to PascalCase for type name
                            let type_name = file_name
                                .split('_')
                                .map(|s| {
                                    let mut c = s.chars();
                                    match c.next() {
                                        None => String::new(),
                                        Some(f) => {
                                            f.to_uppercase().collect::<String>() + c.as_str()
                                        }
                                    }
                                })
                                .collect::<String>();

                            generated.insert((
                                package_name.clone(),
                                interface_type.to_string(),
                                type_name,
                            ));
                        }
                    }
                }
            }
        }
    }

    generated
}

fn main() {
    oxidros_build::ros2_env_var_changed();

    let config = Config::builder()
        .extra_search_path("~/github/ros2-msg/ros2-windows/share/")
        .build();

    // Generate the message types
    oxidros_build::msg::generate_msgs_with_config(&config);

    // Discover all types with JSON files for test generation
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let generated_dir = out_dir.join("generated");

    // First, find what was actually generated
    let generated_types = scan_generated_types(&generated_dir);

    // Collect all discovered types from all search paths
    let mut all_discovered = Vec::new();
    let mut seen_keys = HashSet::new();

    for search_path in config.get_search_paths() {
        let share_dir = search_path.join("share");
        let discovered = scan_for_types_with_json(&share_dir);

        for d in discovered {
            // Only keep types that have JSON files, were generated, and haven't been seen
            let type_key = (d.package.clone(), d.interface_type.clone(), d.name.clone());
            if d.json_path.is_some() && generated_types.contains(&type_key) {
                let key = format!("{}/{}/{}", d.package, d.interface_type, d.name);
                if seen_keys.insert(key) {
                    all_discovered.push(d);
                }
            }
        }
    }

    // Generate test registry file
    let test_registry_path = out_dir.join("test_registry.rs");
    let mut file = File::create(&test_registry_path).expect("Failed to create test_registry.rs");

    writeln!(
        file,
        "// Auto-generated test registry for type hash validation"
    )
    .unwrap();
    writeln!(file, "// Generated by build.rs").unwrap();
    writeln!(file).unwrap();
    writeln!(file).unwrap();

    // Generate TypeEntry struct
    writeln!(file, "#[derive(Debug, Clone)]").unwrap();
    writeln!(file, "pub struct TypeEntry {{").unwrap();
    writeln!(file, "    pub package: &'static str,").unwrap();
    writeln!(file, "    pub interface_type: &'static str,").unwrap();
    writeln!(file, "    pub name: &'static str,").unwrap();
    writeln!(file, "    pub json_path: &'static str,").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file).unwrap();

    // Generate ALL_TYPES constant
    writeln!(file, "pub const ALL_TYPES: &[TypeEntry] = &[").unwrap();

    for d in &all_discovered {
        if let Some(json_path) = &d.json_path {
            writeln!(
                file,
                "    TypeEntry {{ package: \"{}\", interface_type: \"{}\", name: \"{}\", json_path: r#\"{}\"# }},",
                d.package, d.interface_type, d.name, json_path.display()
            ).unwrap();
        }
    }

    writeln!(file, "];").unwrap();
    writeln!(file).unwrap();

    // Group types by category for the dispatch functions
    let msgs: Vec<_> = all_discovered
        .iter()
        .filter(|d| d.interface_type == "msg")
        .collect();
    let srvs: Vec<_> = all_discovered
        .iter()
        .filter(|d| d.interface_type == "srv")
        .collect();
    let actions: Vec<_> = all_discovered
        .iter()
        .filter(|d| d.interface_type == "action")
        .collect();

    // Generate dispatch function for messages
    writeln!(
        file,
        "/// Dispatch function to get type hash for a message type"
    )
    .unwrap();
    writeln!(
        file,
        "pub fn get_msg_hash(package: &str, name: &str) -> Option<String> {{"
    )
    .unwrap();
    writeln!(file, "    use ros2_types::TypeDescription;").unwrap();
    writeln!(file, "    match (package, name) {{").unwrap();

    for msg in &msgs {
        let module_name = msg.name.to_snake_case();
        writeln!(
            file,
            "        (\"{}\", \"{}\") => crate::generated::{}::msg::{}::{}::compute_hash().ok(),",
            msg.package, msg.name, msg.package, module_name, msg.name
        )
        .unwrap();
    }

    writeln!(file, "        _ => None,").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file).unwrap();

    // Generate dispatch function for services
    writeln!(
        file,
        "/// Dispatch function to get type hash for a service type"
    )
    .unwrap();
    writeln!(
        file,
        "pub fn get_srv_hash(package: &str, name: &str) -> Option<String> {{"
    )
    .unwrap();
    writeln!(file, "    use ros2_types::ServiceTypeDescription;").unwrap();
    writeln!(file, "    match (package, name) {{").unwrap();

    for srv in &srvs {
        let module_name = srv.name.to_snake_case();
        writeln!(
            file,
            "        (\"{}\", \"{}\") => crate::generated::{}::srv::{}::{}::compute_hash().ok(),",
            srv.package, srv.name, srv.package, module_name, srv.name
        )
        .unwrap();
    }

    writeln!(file, "        _ => None,").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file).unwrap();

    // Generate dispatch function for actions
    writeln!(
        file,
        "/// Dispatch function to get type hash for an action type"
    )
    .unwrap();
    writeln!(
        file,
        "pub fn get_action_hash(package: &str, name: &str) -> Option<String> {{"
    )
    .unwrap();
    writeln!(file, "    use ros2_types::ActionTypeDescription;").unwrap();
    writeln!(file, "    match (package, name) {{").unwrap();

    for action in &actions {
        let module_name = action.name.to_snake_case();
        writeln!(
            file,
            "        (\"{}\", \"{}\") => crate::generated::{}::action::{}::{}::compute_hash().ok(),",
            action.package, action.name, action.package, module_name, action.name
        ).unwrap();
    }

    writeln!(file, "        _ => None,").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    println!(
        "cargo:info=Generated test registry with {} types ({} msg, {} srv, {} action)",
        all_discovered.len(),
        msgs.len(),
        srvs.len(),
        actions.len()
    );
}
