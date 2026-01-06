#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(deref_nullptr)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(unused_imports)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_c_str_literals)]
#![allow(clippy::useless_conversion)]

//! Generated ROS2 message types for oxidros.
//!
//! This crate provides Rust bindings for ROS2 messages, services, and actions.
//! Select the appropriate ROS2 distribution using features: `jazzy`, `iron`, `humble`, or `galactic`.
//!
//! Messages are generated at compile time using ros2msg and ros2-types-derive for FFI.

// Re-export rcl types for generated code (only available with rcl feature)
#[cfg(feature = "rcl")]
pub mod rcl {
    // Re-export C types from runtime_c
    pub use crate::runtime_c::*;
}

pub mod primitives;
pub mod strings;

// Include runtime C bindings first (provides rosidl_runtime_c types)
#[cfg(feature = "rcl")]
mod runtime_c {
    include!(concat!(env!("OUT_DIR"), "/runtime_c.rs"));
}

// Re-export runtime_c types
#[cfg(feature = "rcl")]
pub use runtime_c::*;

// Re-export msg module utilities for generated code
pub mod msg {
    pub use crate::primitives::{
        BoolSeq, F32Seq, F64Seq, I8Seq, I16Seq, I32Seq, I64Seq, U8Seq, U16Seq, U32Seq, U64Seq,
    };
    pub use crate::strings::{RosString, RosStringSeq, RosWString, RosWStringSeq};
    pub use oxidros_core::TypeSupport;
}

// Re-export builtin_interfaces types
pub mod builtin_interfaces {
    pub use oxidros_core::{UnsafeDuration, UnsafeTime};
}

// Re-export ros2-types traits and macros for generated code
pub use ros2_types::{
    Ros2Msg, SequenceRaw, ServiceMsg, TryClone, TypeSupport, ros2_action, ros2_service,
};

// Include generated message modules
pub mod common_interfaces {
    //! Common ROS2 interface messages (geometry_msgs, sensor_msgs, etc.)
    include!(concat!(env!("OUT_DIR"), "/common_interfaces/mod.rs"));
}

pub mod interfaces {
    //! ROS2 core interfaces (rcl_interfaces, action_msgs, etc.)
    include!(concat!(env!("OUT_DIR"), "/interfaces/mod.rs"));
}

pub mod ros2msg {
    //! Additional ROS2 messages (unique_identifier_msgs, etc.)
    include!(concat!(env!("OUT_DIR"), "/ros2msg/mod.rs"));
}

// Re-export commonly used items
pub use ros2msg::*;

// Re-export oxidros_core module so generated code can use crate::oxidros_core::TypeSupport
pub use oxidros_core;

// Re-export traits from oxidros-core at the top level for convenience
pub use oxidros_core::{
    ActionGoal, ActionMsg, ActionResult, GetUUID, GoalResponse, ResultResponse,
};

// Re-export UnsafeTime and UnsafeDuration from oxidros-core
pub use oxidros_core::{UnsafeDuration, UnsafeTime};

use crate::interfaces::rcl_interfaces::msg::ParameterValue;
use crate::msg::{BoolSeq, F64Seq, I64Seq, RosString, RosStringSeq, U8Seq};
use oxidros_core::Value;

impl From<&oxidros_core::parameter::IntegerRange>
    for interfaces::rcl_interfaces::msg::IntegerRange
{
    fn from(range: &oxidros_core::parameter::IntegerRange) -> Self {
        interfaces::rcl_interfaces::msg::IntegerRange {
            from_value: range.min,
            to_value: range.max,
            step: range.step as u64,
        }
    }
}

impl From<&oxidros_core::parameter::FloatingPointRange>
    for interfaces::rcl_interfaces::msg::FloatingPointRange
{
    fn from(range: &oxidros_core::parameter::FloatingPointRange) -> Self {
        interfaces::rcl_interfaces::msg::FloatingPointRange {
            from_value: range.min,
            to_value: range.max,
            step: range.step,
        }
    }
}

impl From<&ParameterValue> for Value {
    fn from(var: &ParameterValue) -> Self {
        match var.r#type {
            1 => Value::Bool(var.bool_value),
            2 => Value::I64(var.integer_value),
            3 => Value::F64(var.double_value),
            4 => Value::String(var.string_value.to_string()),
            5 => {
                let mut v = Vec::new();
                var.byte_array_value.iter().for_each(|x| v.push(*x));
                Value::VecU8(v)
            }
            6 => {
                let mut v = Vec::new();
                var.bool_array_value.iter().for_each(|x| v.push(*x));
                Value::VecBool(v)
            }
            7 => {
                let mut v = Vec::new();
                var.integer_array_value.iter().for_each(|x| v.push(*x));
                Value::VecI64(v)
            }
            8 => {
                let mut v = Vec::new();
                var.double_array_value.iter().for_each(|x| v.push(*x));
                Value::VecF64(v)
            }
            9 => {
                let mut v = Vec::new();
                var.string_array_value
                    .iter()
                    .for_each(|x| v.push(x.to_string()));
                Value::VecString(v)
            }
            _ => Value::NotSet,
        }
    }
}

impl From<&Value> for ParameterValue {
    fn from(var: &Value) -> Self {
        let mut result = ParameterValue::new().unwrap();
        match var {
            Value::NotSet => result.r#type = 0,
            Value::Bool(val) => {
                result.r#type = 1;
                result.bool_value = *val;
            }
            Value::I64(val) => {
                result.r#type = 2;
                result.integer_value = *val;
            }
            Value::F64(val) => {
                result.r#type = 3;
                result.double_value = *val;
            }
            Value::String(val) => {
                result.r#type = 4;
                result.string_value = RosString::new(val).unwrap_or_else(|| {
                    log::error!("{}:{}: failed allocation", file!(), line!());
                    RosString::null()
                });
            }
            Value::VecU8(val) => {
                result.r#type = 5;
                result.byte_array_value = U8Seq::new(val.len()).unwrap_or_else(|| {
                    log::error!("{}:{}: failed allocation", file!(), line!());
                    U8Seq::null()
                });
                result
                    .byte_array_value
                    .iter_mut()
                    .zip(val.iter())
                    .for_each(|(dst, src)| *dst = *src);
            }
            Value::VecBool(val) => {
                result.r#type = 6;
                result.bool_array_value = BoolSeq::new(val.len()).unwrap_or_else(|| {
                    log::error!("{}:{}: failed allocation", file!(), line!());
                    BoolSeq::null()
                });
                result
                    .bool_array_value
                    .iter_mut()
                    .zip(val.iter())
                    .for_each(|(dst, src)| *dst = *src);
            }
            Value::VecI64(val) => {
                result.r#type = 7;
                result.integer_array_value = I64Seq::new(val.len()).unwrap_or_else(|| {
                    log::error!("{}:{}: failed allocation", file!(), line!());
                    I64Seq::null()
                });
                result
                    .integer_array_value
                    .iter_mut()
                    .zip(val.iter())
                    .for_each(|(dst, src)| *dst = *src);
            }
            Value::VecF64(val) => {
                result.r#type = 8;
                result.double_array_value = F64Seq::new(val.len()).unwrap_or_else(|| {
                    log::error!("{}:{}: failed allocation", file!(), line!());
                    F64Seq::null()
                });
                result
                    .double_array_value
                    .iter_mut()
                    .zip(val.iter())
                    .for_each(|(dst, src)| *dst = *src);
            }
            Value::VecString(val) => {
                result.r#type = 9;
                result.string_array_value = RosStringSeq::new(val.len()).unwrap_or_else(|| {
                    log::error!("{}:{}: failed allocation", file!(), line!());
                    RosStringSeq::null()
                });
                result
                    .string_array_value
                    .iter_mut()
                    .zip(val.iter())
                    .for_each(|(dst, src)| {
                        dst.assign(src);
                    });
            }
        }
        result
    }
}
