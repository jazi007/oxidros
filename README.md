# Oxidros

**Oxidros** (from *oxide* + *ROS*) - Rust bindings and native implementations for ROS2.

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

## Overview

Oxidros provides two ways to build ROS2 applications in Rust:

- **RCL Backend** (`oxidros-rcl`): FFI bindings to the official ROS2 C library.
  Requires a ROS2 installation (Humble, Jazzy, or Kilted).

- **Zenoh Backend** (`oxidros-zenoh`): Pure Rust implementation using [Zenoh](https://zenoh.io/) middleware.
  Compatible with `rmw_zenoh_cpp`. No ROS2 installation required at runtime.

Both backends share a unified API through the `oxidros` crate, making it easy to switch between them.

## Features

- **Unified API**: Write code once, run with either backend
- **Standard Rust tooling**: Works with `cargo` - no custom build tools required
- **Build-time message generation**: Message types generated at compile time via `build.rs`
- **Async/await support**: First-class async support with tokio
- **Selector-based callbacks**: Traditional callback-based event handling
- **Tracing integration**: Modern logging via the `tracing` ecosystem

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
# For RCL backend (requires ROS2 installation)
oxidros = { version = "0.1", features = ["jazzy"] }

# OR for Zenoh backend (pure Rust)
oxidros = { version = "0.1", features = ["zenoh"] }

tokio = { version = "1", features = ["full"] }
```

### Async Publisher/Subscriber

```rust
use oxidros::prelude::*;
use oxidros::msg::common_interfaces::std_msgs::msg::String;

#[tokio::main]
async fn main() -> oxidros::error::Result<()> {
    let ctx = Context::new()?;
    let node = ctx.new_node("my_node", None)?;
    
    // Publisher
    let publisher = node.create_publisher::<String>("chatter", None)?;
    
    // Subscriber
    let mut subscriber = node.create_subscriber::<String>("chatter", None)?;
    
    // Async receive
    let msg = subscriber.recv().await?;
    println!("Received: {}", msg.sample.data.get_string());
    
    Ok(())
}
```

### Async Service Client/Server

```rust
use oxidros::prelude::*;
use oxidros::msg::common_interfaces::example_interfaces::srv::AddTwoInts;

#[tokio::main]
async fn main() -> oxidros::error::Result<()> {
    let ctx = Context::new()?;
    let node = ctx.new_node("my_node", None)?;
    
    // Client
    let mut client = node.create_client::<AddTwoInts>("add_two_ints", None)?;
    
    // Wait for service
    while !client.is_service_available() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    // Call service
    let request = AddTwoInts_Request { a: 1, b: 2 };
    let response = client.call(&request).await?;
    println!("Sum: {}", response.sample.sum);
    
    Ok(())
}
```

### Selector-based (Callback) Pattern

```rust
use oxidros::prelude::*;
use oxidros::msg::common_interfaces::std_msgs::msg::String;
use std::time::Duration;

fn main() -> oxidros::error::Result<()> {
    let ctx = Context::new()?;
    let node = ctx.new_node("my_node", None)?;
    
    let mut selector = ctx.new_selector()?;
    
    // Add subscriber with callback
    let subscriber = node.create_subscriber::<String>("chatter", None)?;
    selector.add_subscriber(subscriber, Box::new(|msg| {
        println!("Received: {}", msg.sample.data.get_string());
    }));
    
    // Add timer
    selector.add_wall_timer("timer", Duration::from_secs(1), Box::new(|| {
        println!("Timer fired!");
    }));
    
    // Event loop
    loop {
        selector.wait()?;
    }
}
```

## Crate Structure

| Crate | Description |
|-------|-------------|
| `oxidros` | Unified API crate - use this in your applications |
| `oxidros-rcl` | RCL backend (FFI bindings) |
| `oxidros-zenoh` | Zenoh backend (pure Rust) |
| `oxidros-core` | Shared types and traits |
| `oxidros-msg` | ROS2 message type generation |
| `ros2-types` | CDR serialization and type traits |
| `ros2args` | ROS2 argument parsing |

## Examples

See the `examples/simple` directory for complete working examples (RCL backend):

- `publisher.rs` - Simple publisher loop
- `subscriber.rs` - Async subscriber
- `async_client.rs` - Async service client
- `async_server.rs` - Async service server
- `client.rs` - Service client with tokio
- `server.rs` - Selector-based service server
- `selector_pubsub.rs` - Selector-based pub/sub
- `selector_service.rs` - Selector-based service
- `parameters.rs` - Parameter server usage

Run examples:

```bash
# Source ROS2 (required for examples)
source /opt/ros/jazzy/setup.bash

# Run publisher
cargo run -p simple --bin publisher --features jazzy

# Run subscriber (in another terminal)
cargo run -p simple --bin subscriber --features jazzy
```

## Documentation

- [API Reference](docs/API_REFERENCE.md) - Auto-generated API comparison
- [API Alignment Plan](docs/API_ALIGNMENT_PLAN.md) - Backend API alignment status

Generate API docs:

```bash
just api-docs
```

## License

Apache 2.0 - see [LICENSE](LICENSE)
