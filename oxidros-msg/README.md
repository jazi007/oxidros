# oxidros-msg

Generated ROS2 message types for oxidros.

This crate provides Rust bindings for ROS2 messages, services, and actions, generated at build time using [ros2msg](../ros2msg) and [ros2-types-derive](../ros2-types-derive).

## Features

- `default` — Pure Rust message types (no FFI, no ROS2 installation required)
- `rcl` — Enables FFI code generation for ROS2 C libraries (requires a sourced ROS2 environment)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
oxidros-msg = { path = "../oxidros-msg" }

# With FFI support:
# oxidros-msg = { path = "../oxidros-msg", features = ["rcl"] }
```

## Structure

- `common_interfaces` — Standard ROS2 common interfaces (geometry_msgs, sensor_msgs, std_msgs, nav_msgs, etc.)
- `interfaces` — ROS2 core interfaces (rcl_interfaces, action_msgs, lifecycle_msgs, etc.)
- `ros2msg` — Additional ROS2 messages (unique_identifier_msgs, etc.)
- `primitives` — ROS2 primitive sequence types (BoolSeq, F64Seq, U8Seq, etc.)
- `strings` — ROS2 string types (RosString, RosWString, and their sequence variants)

## Code Generation

Message files are generated at compile time into `src/generated/` by the build script:

- **ROS2 sourced/installed**: Regenerates message files from installed ROS2 `.msg`/`.srv`/`.action` definitions.
- **No ROS2**: Uses pre-committed files in `src/generated/` — no generation needed.

This allows the crate to be built without a ROS2 installation.
