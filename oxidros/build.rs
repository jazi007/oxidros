fn main() {
    oxidros_build::ros2_env_var_changed();
    println!("cargo:rerun-if-env-changed=SAFE_DRIVE_TEST");
    if std::env::var_os("SAFE_DRIVE_TEST").is_some() {
        println!("cargo:rustc-link-lib=example_msg__rosidl_typesupport_c");
        println!("cargo:rustc-link-lib=example_msg__rosidl_generator_c");
        println!("cargo:rustc-link-search=oxidros/supplements/ros2/install/example_msg/lib");
    }
}
