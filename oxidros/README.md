# oxidros

A safe, idiomatic Rust client library for ROS 2.

## Overview

`oxidros` is the main entry-point crate for the Oxidros ecosystem, providing a unified API
with multiple backend implementations:

- **RCL Backend** (`rcl` feature): FFI bindings to the official ROS2 C library.
  Requires a ROS2 installation (Humble, Jazzy, or Kilted).

- **Zenoh Backend** (`zenoh` feature): Pure Rust implementation using Zenoh middleware.
  Compatible with `rmw_zenoh_cpp`. No ROS2 installation required at runtime.

## Feature Flags

Choose exactly one backend by enabling one of these features:

| Feature | Backend | Requirements |
|---------|---------|--------------|
| `zenoh` | Zenoh   | None (pure Rust) |
| `rcl`   | RCL     | ROS2 installation |

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
# For Zenoh backend (no ROS2 needed):
oxidros = { version = "0.5", features = ["zenoh"] }

# For RCL backend (requires ROS2 installation):
oxidros = { version = "0.5", features = ["rcl"] }
tokio = { version = "1", features = ["full"] }
```

### Example

```rust
use oxidros::prelude::*;
use oxidros::msg::common_interfaces::std_msgs;

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let ctx = Context::new()?;
    let node = ctx.create_node("my_node", None)?;

    let publisher = node.create_publisher::<std_msgs::msg::String>("chatter", None)?;

    let msg = std_msgs::msg::String { data: "Hello from Rust!".into() };
    publisher.send(&msg)?;

    Ok(())
}
```
