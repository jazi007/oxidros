use oxidros_build::msg::Config;

fn main() {
    oxidros_build::ros2_env_var_changed();
    let config = Config::builder()
        .extra_search_path("~/github/ros2-msg/ros2-windows/share/")
        .build();
    oxidros_build::msg::generate_msgs_with_config(&config);
}
