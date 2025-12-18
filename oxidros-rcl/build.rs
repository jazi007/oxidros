use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    oxidros_build::ros2_env_var_changed();
    oxidros_build::generate_rcl_bindings(out_path);
    oxidros_build::link_rcl_ros2_libs();
}
