//! Dynamic CDR decoding and encoding using ROS2 type descriptions.
//!
//! This crate provides runtime CDR deserialization without compile-time type knowledge.
//! Given raw CDR bytes and a [`TypeDescriptionMsg`], it produces a `serde_json::Value` tree.
//!
//! # Example
//!
//! ```ignore
//! use oxidros_dynamic::decode_cdr;
//! use ros2_types::types::TypeDescriptionMsg;
//!
//! let type_desc: TypeDescriptionMsg = /* obtained at runtime */;
//! let cdr_bytes: &[u8] = /* raw CDR payload including 4-byte header */;
//! let json = decode_cdr(cdr_bytes, &type_desc)?;
//! println!("{}", serde_json::to_string_pretty(&json)?);
//! ```

mod decoder;
mod encoder;
mod error;

pub use decoder::decode_cdr;
pub use encoder::encode_cdr;
pub use error::{DynamicError, Result};
