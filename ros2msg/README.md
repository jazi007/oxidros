# ros2msg - ROS2 Message Parser for Rust

A standalone Rust library for parsing ROS2 message, service, and action files. This crate provides comprehensive functionality to parse `.msg`, `.srv`, and `.action` files according to the ROS2 IDL specification, with support for MSG/SRV/Action to IDL conversion.

This crate can be used independently in any Rust project that needs to work with ROS2 interface definitions, whether you're building code generators, analysis tools, documentation systems, or full ROS2 applications.

## Features

- **Message parsing**: Parse `.msg` files with support for primitive types, arrays, and constants
- **Service parsing**: Parse `.srv` files with request/response separation  
- **Action parsing**: Parse `.action` files with goal/result/feedback sections
- **IDL conversion**: Convert MSG/SRV/Action files to IDL format (compatible with `rosidl_adapter`)
- **Serde support**: Optional serialization support with the `serde` feature

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ros2msg = "0.1"

# Enable serde support (optional)
ros2msg = { version = "0.1", features = ["serde"] }
```

## Quick Start

```rust
use ros2msg::*;

// Parse a message
let msg_content = r#"
# A simple Point message
float64 x  # X coordinate [m]
float64 y  # Y coordinate [m] 
float64 z  # Z coordinate [m]
"#;

let msg_spec = parse_message_string("geometry_msgs", "Point", msg_content)?;
println!("Parsed message: {}", msg_spec.msg_name);
println!("Fields: {}", msg_spec.fields.len());

// Parse a service
let srv_content = r#"
# Add two integers
int64 a
int64 b
---
int64 sum
"#;

let srv_spec = parse_service_string("example_interfaces", "AddTwoInts", srv_content)?;
println!("Request fields: {}", srv_spec.request.fields.len());
println!("Response fields: {}", srv_spec.response.fields.len());

// Parse an action
let action_content = r#"
# Fibonacci sequence action
int32 order
---
int32[] sequence
---
int32[] partial_sequence
"#;

let action_spec = parse_action_string("action_tutorials", "Fibonacci", action_content)?;
println!("Goal fields: {}", action_spec.goal.fields.len());
```

## Supported Types

### Primitive Types

- `bool` - Boolean values
- `byte`, `char`, `uint8` - 8-bit unsigned integers  
- `int8` - 8-bit signed integers
- `uint16`, `int16` - 16-bit integers
- `uint32`, `int32` - 32-bit integers
- `uint64`, `int64` - 64-bit integers
- `float32`, `float64` - Floating point numbers
- `string`, `wstring` - String types
- `duration`, `time` - Time-related types

### Array Types

```rust
// Dynamic arrays
int32[] numbers

// Fixed-size arrays  
int32[5] fixed_numbers

// Bounded arrays (upper limit)
int32[<=10] bounded_numbers

// String bounds
string<=50 limited_string
```

### Complex Types

```rust
// Message references
geometry_msgs/Point position

// Arrays of complex types
geometry_msgs/Point[] waypoints
sensor_msgs/LaserScan[<=5] scans
```

### Constants

```rust
// Primitive constants
int32 MAX_SIZE=100
float64 PI=3.14159
string DEFAULT_NAME="robot"
bool DEBUG_MODE=true
```