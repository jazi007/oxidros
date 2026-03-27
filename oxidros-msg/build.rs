//! Build script for oxidros-msg
//!
//! Uses ros2msg to generate ROS2 message types with ros2-types-derive for FFI support.
//!
//! # Generation Strategy
//!
//! All generated output goes into `OUT_DIR/generated/` (never into the source tree).
//! This prevents race conditions when multiple concurrent cargo processes build the
//! same crate from the crates.io registry cache.
//!
//! - **ROS2 Sourced/Installed**: Generates fresh message files into `OUT_DIR/generated/`
//! - **No ROS2**: Copies pre-committed `src/generated/` files into `OUT_DIR/generated/`
//!
//! To update the pre-committed files, set `OXIDROS_REGENERATE_SRC=1` with ROS2 sourced.

use std::env;
use std::path::{Path, PathBuf};

use oxidros_build::msg::{Config, RosAvailability, detect_ros_availability, get_base_generator};

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst)
        .unwrap_or_else(|e| panic!("Failed to create directory {}: {}", dst.display(), e));
    for entry in std::fs::read_dir(src)
        .unwrap_or_else(|e| panic!("Failed to read directory {}: {}", src.display(), e))
    {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path);
        } else {
            std::fs::copy(&src_path, &dst_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to copy {} -> {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            });
        }
    }
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_generated = PathBuf::from(&manifest_dir).join("src").join("generated");

    // Generated output always goes into OUT_DIR (isolated per crate build)
    let out_generated = out_path.join("generated");

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

    // Check ROS2 availability to decide whether to generate
    let config = Config::builder().build();
    let availability = detect_ros_availability(&config);

    // Optionally also update the committed src/generated/ tree (developer workflow)
    let regenerate_src = env::var("OXIDROS_REGENERATE_SRC").is_ok();

    // Try generation if ROS2 is detected, then verify output.
    // Fall back to pre-committed files if generation produced nothing
    // (e.g. a directory matches common ROS2 paths but has no message packages).
    let mut generated = false;

    if matches!(
        &availability,
        RosAvailability::Sourced { .. } | RosAvailability::CommonInstall { .. }
    ) {
        println!("cargo:rerun-if-env-changed=OXIDROS_REGENERATE_SRC");
        println!("cargo:info=ROS2 detected, attempting message generation into OUT_DIR");

        std::fs::create_dir_all(&out_generated).unwrap();

        let generate_group = |subdir: &str, packages: &[&str]| -> bool {
            let out_subdir = out_generated.join(subdir);
            std::fs::create_dir_all(&out_subdir).unwrap();
            let config = Config::builder()
                .packages(packages)
                .uuid_path("crate::ros2msg")
                .primitive_path("crate")
                .build();
            if let Some(generator) = get_base_generator(&config) {
                generator
                    .output_dir(&out_subdir)
                    .generate()
                    .unwrap_or_else(|e| panic!("Failed to generate {}: {}", subdir, e));

                if regenerate_src {
                    let src_subdir = src_generated.join(subdir);
                    if src_subdir.exists() {
                        std::fs::remove_dir_all(&src_subdir).ok();
                    }
                    copy_dir_recursive(&out_subdir, &src_subdir);
                }
                true
            } else {
                false
            }
        };

        let ok_common = generate_group("common_interfaces", &common_interfaces_deps);
        let ok_ifaces = generate_group("interfaces", &interface_deps);
        let ok_ros2msg = generate_group("ros2msg", &ros2msg_deps);

        generated = ok_common && ok_ifaces && ok_ros2msg;

        if !generated {
            println!(
                "cargo:warning=ROS2 path detected but message packages not found, \
                 falling back to pre-generated files"
            );
            // Clean up partial output
            if out_generated.exists() {
                std::fs::remove_dir_all(&out_generated).ok();
            }
        }
    }

    if !generated {
        if !matches!(
            &availability,
            RosAvailability::Sourced { .. } | RosAvailability::CommonInstall { .. }
        ) {
            println!("cargo:warning=No ROS2 installation detected");
        }
        println!("cargo:warning=Copying pre-generated message files to OUT_DIR");

        // Verify that pre-generated files exist
        if !src_generated
            .join("common_interfaces")
            .join("mod.rs")
            .exists()
        {
            panic!(
                "Pre-generated message files not found in {}. \
                 Either install ROS2 and regenerate, or ensure the generated files are committed.",
                src_generated.display()
            );
        }

        // Copy pre-committed files into OUT_DIR so lib.rs can include from there
        copy_dir_recursive(&src_generated, &out_generated);
    }

    // Generate runtime_c.rs using bindgen (only when ROS2 is sourced for rcl feature)
    if availability.is_sourced() {
        oxidros_build::generate_runtime_c(out_path);

        // Link ROS2 libraries (required for tests and standalone use)
        oxidros_build::link_rcl_ros2_libs();
        oxidros_build::link_msg_ros2_libs();
    }
}
