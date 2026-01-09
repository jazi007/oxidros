fn main() {
    oxidros_build::ros2_env_var_changed();
    oxidros_build::msg::generate_msgs(&[]);
}
