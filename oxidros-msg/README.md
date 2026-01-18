# oxidros-msg

Generated ROS2 message types for oxidros.

This crate contains generated Rust bindings for ROS2 messages, services, and actions across multiple ROS2 distributions (galactic, humble, iron, jazzy).

## Features

Select the ROS2 distribution you want to use:

- `jazzy` - ROS2 Jazzy messages
- `iron` - ROS2 Iron messages
- `humble` - ROS2 Humble messages
- `galactic` - ROS2 Galactic messages

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
oxidros-msg = { path = "../oxidros-msg", features = ["jazzy"] }
```

## Structure

Each distribution includes:
- `common_interfaces` - Standard ROS2 common interfaces (geometry_msgs, sensor_msgs, etc.)
- `interfaces` - ROS2 core interfaces (rcl_interfaces, action_msgs, etc.)
- `ros2msg` - Additional ROS2 messages
- `runtime_c` - C runtime bindings

## Code Generation

These files are automatically generated using [ros2msg_to_rs](https://github.com/tier4/ros2msg_to_rs) and bindgen.

See the documentation in each distro module for generation instructions.
