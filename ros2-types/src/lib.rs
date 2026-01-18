//! ROS2 type support library
//!
//! This crate provides core traits and utilities for ROS2 message types,
//! including RIHS01 (ROS Interface Hashing Standard v1) type hash calculation.
//!
//! # Features
//!
//! - `derive`: Enable derive macros for `TypeDescription` and `Ros2Msg`
//! - `native`: Enable native CDR serialization (for Zenoh, iceoryx2, etc.)
//!
//! # Traits
//!
//! This crate provides several traits for ROS2 message types:
//!
//! - `TypeSupport`: For types with type support information and CDR serialization
//! - `TryClone`: For types that can fail cloning (FFI types)
//! - `ServiceMsg`: For ROS2 service types (Request/Response pairs)
//! - `ActionMsg`: For ROS2 action types (Goal/Result/Feedback)
//! - `ActionGoal`, `ActionResult`: For action service types
//! - `GetUUID`, `GoalResponse`, `ResultResponse`: Helper traits for actions
//!
//! # Native CDR Serialization
//!
//! When the `native` feature is enabled, message types can be serialized/deserialized
//! using CDR encoding (compatible with DDS/ROS2):
//!
//! ```ignore
//! use ros2_types::TypeSupport;
//!
//! let msg = std_msgs::msg::String { data: "hello".into() };
//! let bytes = msg.to_bytes()?;
//! let decoded = std_msgs::msg::String::from_bytes(&bytes)?;
//! ```
//!
//! **Note**: When using `native` feature, message structs must derive
//! `serde::Serialize` and `serde::Deserialize`.

pub mod cdr;
mod error;
mod hash;
mod ros_field_type;
mod traits;

mod type_description;
pub mod types;

pub use cdr::CdrSerde;
pub use error::{Error, Result};
pub use hash::{calculate_type_hash, parse_rihs_string};
pub use ros_field_type::RosFieldType;
pub use traits::{
    ActionGoal, ActionMsg, ActionResult, GetUUID, GoalResponse, ResultResponse, SequenceRaw,
    ServiceMsg, TryClone, TypeSupport, UnsafeDuration, UnsafeTime,
};
pub use type_description::{
    ActionTypeDescription, MessageTypeName, ServiceTypeDescription, TypeDescription,
};
pub use types::{
    FIELD_TYPE_BOOLEAN, FIELD_TYPE_BOUNDED_STRING, FIELD_TYPE_BOUNDED_WSTRING, FIELD_TYPE_BYTE,
    FIELD_TYPE_CHAR, FIELD_TYPE_DOUBLE, FIELD_TYPE_FIXED_STRING, FIELD_TYPE_FIXED_WSTRING,
    FIELD_TYPE_FLOAT, FIELD_TYPE_INT8, FIELD_TYPE_INT16, FIELD_TYPE_INT32, FIELD_TYPE_INT64,
    FIELD_TYPE_LONG_DOUBLE, FIELD_TYPE_NESTED_TYPE, FIELD_TYPE_NOT_SET, FIELD_TYPE_STRING,
    FIELD_TYPE_UINT8, FIELD_TYPE_UINT16, FIELD_TYPE_UINT32, FIELD_TYPE_UINT64, FIELD_TYPE_WCHAR,
    FIELD_TYPE_WSTRING,
};

// Note: Field, FieldType, IndividualTypeDescription, TypeDescriptionMsg are NOT re-exported
// at the crate root to avoid conflicts with generated type_description_interfaces messages.
// Access them via ros2_types::types::{Field, FieldType, ...} if needed.

#[cfg(feature = "derive")]
pub use ros2_types_derive::{
    ActionTypeDescription, Ros2Msg, ServiceTypeDescription, TypeDescription, ros2_action,
    ros2_service,
};

// Re-export cdr-encoding dependencies for generated code
pub use serde;
pub use serde_big_array::BigArray;
