# oxidros-zenoh

Native ROS2 implementation using Zenoh middleware.

## Overview

`oxidros-zenoh` provides a pure Rust ROS2 implementation that communicates over Zenoh,
compatible with `rmw_zenoh_cpp`. This allows ROS2 nodes built with this crate to
interoperate with standard ROS2 nodes using the Zenoh RMW.

## Features

- **No ROS2 installation required**: Works natively without ROS2 runtime
- **Compatible with rmw_zenoh_cpp**: Follows the [rmw_zenoh design](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md)
- **Pub/Sub**: Topic publishers and subscribers with QoS support
- **Services**: Client/Server request-response pattern
- **Parameters**: Full parameter server support
- **Graph Discovery**: Liveliness-based entity discovery

## Requirements

- A Zenoh router running (default: `localhost:7447`)
- Tokio runtime (Zenoh uses tokio internally)

## Quick Start

```rust
use oxidros_zenoh::{Context, Node};
use std_msgs::msg::String as StringMsg;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create context (connects to Zenoh router)
    let ctx = Context::new()?;
    
    // Create a node
    let node = ctx.create_node("my_node", None)?;
    
    // Create a publisher
    let publisher = node.create_publisher::<StringMsg>("chatter", None)?;
    
    // Publish a message
    let msg = StringMsg { data: "Hello from Rust!".into() };
    publisher.send(&msg)?;
    
    Ok(())
}
```

## Compatibility

This implementation follows the rmw_zenoh design specification:
- Key expression format: `<domain_id>/<fq_name>/<type_name>/<type_hash>`
- Liveliness tokens: `@ros2_lv/<domain_id>/<session_id>/...`
- CDR serialization (Little Endian)
- Attachment format for sequence numbers, timestamps, and GIDs

## Environment Variables

- `ROS_DOMAIN_ID`: Domain ID (default: 0)
- `ZENOH_SESSION_CONFIG_URI`: Path to Zenoh session config file
- `ZENOH_ROUTER_CONFIG_URI`: Path to Zenoh router config file

## License

See LICENSE file in the repository root.
