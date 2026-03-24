# oxidros-build

Build utilities for generating Rust types from ROS2 message definitions.

## Overview

This crate provides `build.rs` helpers for:

- **Custom Message Generation** — Generate Rust types from `.msg`, `.srv`, `.action`, or `.idl` files
- **No ROS2 installation required** — Just point to a directory containing message definitions
- **RCL Bindings Generation** — (Advanced) Generate FFI bindings for the ROS2 C client library

## Quick Start

Add `oxidros-build` to your `build-dependencies` in `Cargo.toml`:

```toml
[build-dependencies]
oxidros-build = "0.5"
```

Create a `build.rs`:

```rust
fn main() {
    oxidros_build::ros2_env_var_changed();

    let config = oxidros_build::msg::Config::builder()
        .packages(&["my_custom_msgs"])
        .build();

    oxidros_build::msg::generate_msgs_with_config(&config);
}
```

Then in your `lib.rs`:

```rust
include!(concat!(env!("OUT_DIR"), "/generated/mod.rs"));
```

## Without a ROS2 Installation

You don't need a full ROS2 installation. Clone the message definition
repositories and point the generator to them:

```rust
fn main() {
    oxidros_build::ros2_env_var_changed();

    let config = oxidros_build::msg::Config::builder()
        .packages(&["my_custom_msgs", "std_msgs"])
        .extra_search_path("/path/to/cloned/common_interfaces")
        .extra_search_path("/path/to/my/custom_msgs")
        .build();

    oxidros_build::msg::generate_msgs_with_config(&config);
}
```

The directory layout should follow the standard ROS2 convention:

```
my_custom_msgs/
├── msg/
│   ├── MyMessage.msg
│   └── AnotherMessage.msg
└── srv/
    └── MyService.srv
```

## Package Discovery

The generator searches for message packages in this order:

1. **`AMENT_PREFIX_PATH`** — If set (sourced ROS2 environment)
2. **Common paths** — `/opt/ros/jazzy`, `/opt/ros/humble` (Linux); `C:\dev\ros2_*` (Windows)
3. **Extra paths** — User-provided via `extra_search_path()`

For Zenoh-based or non-RCL projects, options 1 and 2 are irrelevant — just
use `extra_search_path()` to point to your message definitions directly.

## Advanced — RCL FFI Bindings

For crates that need direct FFI access to the ROS2 C client library
(requires a fully sourced ROS2 environment):

```rust
use std::path::PathBuf;

fn main() {
    oxidros_build::ros2_env_var_changed();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    oxidros_build::generate_rcl_bindings(&out_dir);
    oxidros_build::link_rcl_ros2_libs();
}
```
