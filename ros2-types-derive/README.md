# ros2-types-derive

Derive macros for ROS2 `TypeDescription` and `Ros2Msg` traits.

## Overview

This crate provides procedural macros that generate ROS2 type descriptions and
message bindings automatically from Rust struct definitions.

## Derive Macros

- **`TypeDescription`** — Generates type descriptions for ROS2 type hash computation
- **`Ros2Msg`** — Generates ROS2 message bindings (FFI with `rcl` feature, pure Rust otherwise)

## Helper Macros

- **`ros2_service!`** — Generates service wrapper types
- **`ros2_action!`** — Generates action wrapper types

## Attributes

### Container Attributes

- `#[ros2(package = "pkg_name")]` — Specify the ROS2 package name
- `#[ros2(interface_type = "msg|srv|action")]` — Specify the interface type (default: `"msg"`)

### Field Attributes

- `#[ros2(ros2_type = "byte")]` — Override field type (for byte, char, wstring)
- `#[ros2(capacity = 255)]` — Specify capacity for bounded strings/sequences
- `#[ros2(default = "0")]` — Specify default value

## Example

```rust
use ros2_types_derive::{TypeDescription, Ros2Msg};

#[derive(TypeDescription, Ros2Msg)]
#[ros2(package = "std_msgs", interface_type = "msg")]
#[repr(C)]
pub struct Header {
    pub stamp: Time,
    pub frame_id: String,
}
```
