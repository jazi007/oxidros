# oxidros-build

Build utilities for oxidros ROS2 message generation and FFI bindings.

## Overview

This crate provides `build.rs` helpers for:

- **RCL Bindings Generation** — Generate Rust FFI bindings for the ROS2 C client library
- **Message Bindings Generation** — Generate Rust FFI bindings for ROS2 message types
- **Library Linking** — Set up cargo link directives for ROS2 shared libraries
- **Environment Detection** — Handle ROS2 environment variables (`AMENT_PREFIX_PATH`, `ROS_DISTRO`, etc.)
- **Distro Detection** — Automatically detect the ROS2 distribution from the environment

## Requirements

- A sourced ROS2 installation (e.g., `source /opt/ros/jazzy/setup.bash`)
- `AMENT_PREFIX_PATH` and `ROS_DISTRO` environment variables must be set

## Usage

In your `build.rs`:

```rust
use oxidros_build::{ros2_env_var_changed, generate_rcl_bindings, link_rcl_ros2_libs};
use std::path::PathBuf;

fn main() {
    // Signal cargo to rebuild if ROS2 environment changes
    ros2_env_var_changed();

    // Generate RCL bindings
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    generate_rcl_bindings(&out_dir);

    // Link ROS2 libraries
    link_rcl_ros2_libs();
}
```
