# ros2-msg-gen

A transpiler from ROS2's message types to Rust's types.
This library is used by [cargo-ament-build](https://github.com/tier4/cargo-ament-build) internally.

```rust
use ros2_msg_gen;
use std::path::Path;

let dependencies = ["std_msgs", "std_srvs"];
ros2_msg_gen::depends(&Path::new("/tmp/output_dir"), &dependencies, ros2_msg_gen::SafeDrive::Version("0.2"));
```

## Limitation

This does not support C/C++ like preprocessor.

## Credits

This project is a fork of [safe_drive_msg](https://github.com/tier4/safe_drive_msg), originally developed by:
- Yuuki Takano (TIER IV, Inc.)

We are grateful for their work on this message transpiler for ROS2.
