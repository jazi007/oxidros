//! # Transpiler from ROS2's message types to Rust's types.
//!
//! See [resource_index](https://github.com/ament/ament_cmake/blob/master/ament_cmake_core/doc/resource_index.md)

pub(crate) mod generator;
pub(crate) mod idl;
pub(crate) mod parser;

use std::{
    collections::BTreeSet,
    error::Error,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

pub type DynError = Box<dyn Error + Send + Sync + 'static>;

#[cfg(target_os = "windows")]
const SEP: char = ';';
#[cfg(not(target_os = "windows"))]
const SEP: char = ':';

/// Transpile ROS2's message types to Rust's types.
/// Dependencies will be automatically
///
/// # Example
///
/// ```
/// use ros2_msg_gen;
/// use std::path::Path;
///
/// let dependencies = ["std_msgs", "std_srvs"];
/// ros2_msg_gen::generate(&Path::new("/tmp/output_dir"), &dependencies);
/// ```
pub fn generate(outdir: &Path, libs: &[&str]) -> Result<(), DynError> {
    generate_with_prefix(outdir, libs, None)
}

/// Transpile ROS2's message types to Rust's types with optional oxidros prefix.
///
/// # Arguments
/// * `outdir` - Output directory for generated code
/// * `libs` - ROS2 packages to generate bindings for  
/// * `oxidros_prefix` - Optional prefix for oxidros imports (e.g., Some("crate::") or None)
///
/// # Example
///
/// ```
/// use ros2_msg_gen;
/// use std::path::Path;
///
/// let dependencies = ["std_msgs", "std_srvs"];
/// // Use crate::oxidros for generated code included in a crate
/// ros2_msg_gen::generate_with_prefix(&Path::new("/tmp/output_dir"), &dependencies, Some("crate::"));
/// ```
pub fn generate_with_prefix(
    outdir: &Path,
    libs: &[&str],
    oxidros_prefix: Option<&str>,
) -> Result<(), DynError> {
    let ament_paths = std::env::var("AMENT_PREFIX_PATH")?;
    let mut ament_paths: Vec<_> = ament_paths
        .split(SEP)
        .filter(|&p| !p.is_empty())
        .map(|p| std::path::Path::new(p).join("share"))
        .collect();
    if cfg!(target_os = "windows") {
        let cmake_paths_env = std::env::var("CMAKE_PREFIX_PATH");
        if let Ok(cmake_paths) = cmake_paths_env {
            let cmake_paths = cmake_paths
                .split(SEP)
                .filter(|&p| !p.is_empty())
                .map(|p| std::path::Path::new(p).join("share"));
            ament_paths.extend(cmake_paths);
        }
    }
    let libs: BTreeSet<_> = libs.iter().map(|e| e.to_string()).collect();
    std::fs::create_dir_all(outdir)?;
    let mut generated_modules = BTreeSet::new();
    generate_modules(
        outdir,
        &ament_paths,
        &libs,
        &mut generated_modules,
        oxidros_prefix,
    )?;
    generate_root_mod(outdir, &generated_modules)?;
    Ok(())
}

fn generate_modules(
    outdir: &Path,
    ament_paths: &Vec<PathBuf>,
    modules: &BTreeSet<String>,
    generated_modules: &mut BTreeSet<String>,
    oxidros_prefix: Option<&str>,
) -> Result<(), DynError> {
    let modules: BTreeSet<_> = modules.iter().collect();

    let outdir = std::path::Path::new(outdir);
    std::fs::create_dir_all(outdir)?;

    'module: for module in modules.iter() {
        for path in ament_paths.iter() {
            let resource = path
                .join("ament_index")
                .join("resource_index")
                .join("packages")
                .join(module);
            if resource.exists() {
                let path = path.join(module);
                if path.exists() {
                    let mut gen = generator::Generator::new(oxidros_prefix);
                    gen.generate(outdir, &path, module)?;
                    generated_modules.insert(module.to_string());
                    // Generate dependencies.
                    generate_modules(
                        outdir,
                        ament_paths,
                        &gen.dependencies,
                        generated_modules,
                        oxidros_prefix,
                    )?;
                    continue 'module;
                } else {
                    eprintln!(
                        "{module} is not found in {}",
                        path.to_str().unwrap_or_default()
                    );
                }
            }
        }
    }

    Ok(())
}

fn generate_root_mod(outdir: &Path, modules: &BTreeSet<String>) -> Result<(), DynError> {
    let mod_file_path = outdir.join("mod.rs");
    let mut mod_file = File::create(mod_file_path)?;

    for module in modules.iter() {
        mod_file.write_fmt(format_args!("pub mod {module};\n"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_msgs() {
        generate(Path::new("/tmp/safe_drive_msg"), &["std_msgs"]).unwrap();
    }

    #[test]
    fn std_srvs() {
        generate(Path::new("/tmp/safe_drive_msg"), &["std_srvs"]).unwrap();
    }

    #[test]
    fn action_tutorials_interfaces() {
        generate(
            Path::new("/tmp/safe_drive_msg"),
            &["action_tutorials_interfaces"],
        )
        .unwrap();
    }
}
