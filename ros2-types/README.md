# ros2-types

ROS2 type support: traits, type descriptions, and RIHS01 hash calculation.

This crate provides:
- Core traits for ROS2 messages, services, and actions (`TypeSupport`, `ServiceMsg`, `ActionMsg`, etc.)
- `TypeDescription` trait for generating type descriptions
- RIHS01 hash calculation that matches the rosidl implementation
- Derive macro support for automatic implementation

## Usage

```rust
use ros2_types::{TypeDescription, calculate_type_hash};

// Implement TypeDescription trait manually or use derive macro
#[derive(TypeDescription)]
struct MyMessage {
    field1: i32,
    field2: String,
}

// Get the type hash
let hash = MyMessage::compute_hash();
println!("Type hash: {}", hash); // RIHS01_<sha256_hash>
```

## RIHS01 Algorithm

RIHS01 (ROS Interface Hashing Standard, version 1) uses SHA256 to hash
a canonical JSON representation of the type description.
