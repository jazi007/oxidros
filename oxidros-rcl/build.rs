use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    oxidros_build::ros2_env_var_changed();

    // Declare expected cfg values for rustc check-cfg
    println!("cargo::rustc-check-cfg=cfg(ros_distro_humble)");
    println!("cargo::rustc-check-cfg=cfg(ros_distro_jazzy)");
    println!("cargo::rustc-check-cfg=cfg(ros_distro_kilted)");

    // Emit ros_distro_xxx cfg based on ROS_DISTRO env
    if let Some(distro) = oxidros_build::detect_distro() {
        oxidros_build::emit_distro_cfg(distro);
    }

    oxidros_build::generate_rcl_bindings(out_path);
    oxidros_build::link_rcl_ros2_libs();
}
