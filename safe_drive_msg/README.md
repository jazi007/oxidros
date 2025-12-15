# Message Transpiler for safe_drive

A transpiler from ROS2's message types to Rust's types.
This library is used by [cargo-ament-build](https://github.com/tier4/cargo-ament-build) internally.

```rust
use safe_drive_msg_v2;
use std::path::Path;

let dependencies = ["std_msgs", "std_srvs"];
safe_drive_msg_v2::depends(&Path::new("/tmp/output_dir"), &dependencies, safe_drive_msg_v2::SafeDrive::Version("0.2"));
```

## Limitation

This does not support C/C++ like preprocessor.

## Credits

This project is a fork of [safe_drive_msg](https://github.com/tier4/safe_drive_msg), originally developed by:
- Yuuki Takano (TIER IV, Inc.)

We are grateful for their work on this message transpiler for ROS2.
