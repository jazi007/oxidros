use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    let dependencies = ["std_msgs", "std_srvs"];
    ros2_msg_gen::generate(out_path, &dependencies).unwrap();
}
