# oxidros-dynamic

Dynamic CDR decoding and encoding using ROS2 type descriptions.

## Overview

`oxidros-dynamic` provides runtime CDR serialization and deserialization without
compile-time type knowledge. Given raw CDR bytes and a `TypeDescriptionMsg`, it
produces a `serde_json::Value` tree (and vice versa).

This is useful for tools that need to inspect arbitrary ROS2 messages at runtime,
such as CLI utilities and bag file processors.

## Usage

```rust
use oxidros_dynamic::{decode_cdr, encode_cdr};
use ros2_types::types::TypeDescriptionMsg;

// Decode CDR bytes to JSON
let type_desc: TypeDescriptionMsg = /* obtained at runtime */;
let cdr_bytes: &[u8] = /* raw CDR payload including 4-byte header */;
let json = decode_cdr(cdr_bytes, &type_desc)?;
println!("{}", serde_json::to_string_pretty(&json)?);

// Encode JSON back to CDR
let cdr = encode_cdr(&json, &type_desc)?;
```
