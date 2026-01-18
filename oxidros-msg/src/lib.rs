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
        BoolSeq, ByteSeq, F32Seq, F64Seq, I8Seq, I16Seq, I32Seq, I64Seq, U8Seq, U16Seq, U32Seq,
        U64Seq,
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
use crate::msg::{BoolSeq, ByteSeq, F64Seq, I64Seq, RosString, RosStringSeq};
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
                result.byte_array_value = ByteSeq::new(val.len()).unwrap_or_else(|| {
                    log::error!("{}:{}: failed allocation", file!(), line!());
                    ByteSeq::null()
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

#[cfg(not(feature = "rcl"))]
#[cfg(test)]
mod tests {
    use oxidros_core::ServiceTypeDescription;

    /// Expected type hashes from ROS2 Jazzy (validated against ros2 CLI)
    /// These hashes must match exactly for interoperability with ros2 param commands
    const EXPECTED_HASHES: &[(&str, &str)] = &[
        (
            "rcl_interfaces/srv/ListParameters",
            "RIHS01_3e6062bfbb27bfb8730d4cef2558221f51a11646d78e7bb30a1e83afac3aad9d",
        ),
        (
            "rcl_interfaces/srv/GetParameters",
            "RIHS01_bf9803d5c74cf989a5de3e0c2e99444599a627c7ff75f97b8c05b01003675cbc",
        ),
        (
            "rcl_interfaces/srv/SetParameters",
            "RIHS01_56eed9a67e169f9cb6c1f987bc88f868c14a8fc9f743a263bc734c154015d7e0",
        ),
        (
            "rcl_interfaces/srv/SetParametersAtomically",
            "RIHS01_0e192ef259c07fc3c07a13191d27002222e65e00ccec653ca05e856f79285fcd",
        ),
        (
            "rcl_interfaces/srv/DescribeParameters",
            "RIHS01_845b484d71eb0673dae682f2e3ba3c4851a65a3dcfb97bddd82c5b57e91e4cff",
        ),
        (
            "rcl_interfaces/srv/GetParameterTypes",
            "RIHS01_da199c878688b3e530bdfe3ca8f74cb9fa0c303101e980a9e8f260e25e1c80ca",
        ),
    ];

    #[test]
    fn test_list_parameters_type_hash() {
        use super::interfaces::rcl_interfaces::srv::ListParameters;
        let hash = ListParameters::compute_hash().expect("failed to compute hash");
        assert_eq!(
            hash, EXPECTED_HASHES[0].1,
            "ListParameters type hash mismatch - interop with ros2 param will fail"
        );
    }

    #[test]
    fn test_get_parameters_type_hash() {
        use super::interfaces::rcl_interfaces::srv::GetParameters;
        let td = GetParameters::type_description();
        println!("GetParameters type description: {:?}", td);
        let hash = GetParameters::compute_hash().expect("failed to compute hash");
        assert_eq!(
            hash, EXPECTED_HASHES[1].1,
            "GetParameters type hash mismatch - interop with ros2 param will fail"
        );
    }

    #[test]
    fn test_set_parameters_type_hash() {
        use super::interfaces::rcl_interfaces::srv::SetParameters;
        let hash = SetParameters::compute_hash().expect("failed to compute hash");
        assert_eq!(
            hash, EXPECTED_HASHES[2].1,
            "SetParameters type hash mismatch - interop with ros2 param will fail"
        );
    }

    #[test]
    fn test_set_parameters_atomically_type_hash() {
        use super::interfaces::rcl_interfaces::srv::SetParametersAtomically;
        let hash = SetParametersAtomically::compute_hash().expect("failed to compute hash");
        assert_eq!(
            hash, EXPECTED_HASHES[3].1,
            "SetParametersAtomically type hash mismatch - interop with ros2 param will fail"
        );
    }

    #[test]
    fn test_describe_parameters_type_hash() {
        use super::interfaces::rcl_interfaces::srv::DescribeParameters;
        let hash = DescribeParameters::compute_hash().expect("failed to compute hash");
        assert_eq!(
            hash, EXPECTED_HASHES[4].1,
            "DescribeParameters type hash mismatch - interop with ros2 param will fail"
        );
    }

    #[test]
    fn test_get_parameter_types_type_hash() {
        use super::interfaces::rcl_interfaces::srv::GetParameterTypes;
        let hash = GetParameterTypes::compute_hash().expect("failed to compute hash");
        assert_eq!(
            hash, EXPECTED_HASHES[5].1,
            "GetParameterTypes type hash mismatch - interop with ros2 param will fail"
        );
    }

    #[test]
    fn test_all_parameter_service_hashes() {
        // Test all parameter service hashes in one place for easy validation
        use super::interfaces::rcl_interfaces::srv::{
            DescribeParameters, GetParameterTypes, GetParameters, ListParameters, SetParameters,
            SetParametersAtomically,
        };

        let services: Vec<(&str, String)> = vec![
            (
                "ListParameters",
                ListParameters::compute_hash().expect("hash"),
            ),
            (
                "GetParameters",
                GetParameters::compute_hash().expect("hash"),
            ),
            (
                "SetParameters",
                SetParameters::compute_hash().expect("hash"),
            ),
            (
                "SetParametersAtomically",
                SetParametersAtomically::compute_hash().expect("hash"),
            ),
            (
                "DescribeParameters",
                DescribeParameters::compute_hash().expect("hash"),
            ),
            (
                "GetParameterTypes",
                GetParameterTypes::compute_hash().expect("hash"),
            ),
        ];

        let mut all_match = true;
        for (i, (name, hash)) in services.iter().enumerate() {
            let expected = EXPECTED_HASHES[i].1;
            if hash != expected {
                eprintln!("MISMATCH: {} - got {} expected {}", name, hash, expected);
                all_match = false;
            }
        }

        assert!(
            all_match,
            "One or more parameter service type hashes do not match ROS2 expectations"
        );
    }
}
