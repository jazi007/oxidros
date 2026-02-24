# oxidros-wrapper

Ergonomic ROS2 API implementing `oxidros-core` traits for the RCL backend.

This crate provides newtype wrappers around `oxidros-rcl` types and implements the core API traits, making it easy to work with ROS2 using async/await patterns.

## Features

- **Newtype wrappers**: Wraps `oxidros-rcl` types to implement `oxidros-core` traits (orphan rule workaround)
- **Async streams**: Convert subscribers to `Stream` via `into_stream()` for use with `tokio::select!`
- **Service handling**: Async service client/server with request/response patterns
- **Action support**: Full action client/server implementation
- **Parameter server**: ROS2 parameter server support

## Requirements

- ROS2 installation (Humble, Jazzy, or Kilted)
- ROS2 environment sourced (`source /opt/ros/<distro>/setup.bash`)

## Example

```rust
use oxidros_wrapper::prelude::*;
use oxidros_wrapper::msg::common_interfaces::std_msgs;
use tokio::signal::ctrl_c;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Context::new()?;
    let node = ctx.create_node("my_node", None)?;

    // Create publisher using core trait
    let publisher = node.create_publisher::<std_msgs::msg::String>("topic", None)?;

    // Create subscriber and convert to stream
    let mut subscriber = node
        .create_subscriber::<std_msgs::msg::String>("topic", None)?
        .into_stream();

    loop {
        tokio::select! {
            Some(Ok(msg)) = subscriber.next() => {
                println!("Received: {:?}", msg.sample.data.get_string());
            }
            _ = ctrl_c() => break,
        }
    }
    Ok(())
}
```

## Architecture

`oxidros-wrapper` sits between application code and `oxidros-rcl`:

```
Application
    │
    ▼
oxidros-wrapper  ─── implements ───►  oxidros-core traits
    │
    ▼
oxidros-rcl  ─── FFI bindings ───►  RCL (ROS Client Library)
```

For a pure-Rust alternative without RCL dependencies, see `oxidros-zenoh`.
