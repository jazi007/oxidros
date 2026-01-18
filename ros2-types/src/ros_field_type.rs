//! RosFieldType trait for type-safe field type mapping
//!
//! This trait allows types to describe how they should be represented
//! as ROS2 field types. This eliminates the need for complex type path
//! analysis in the derive macro.

use crate::types::{
    FIELD_TYPE_BOOLEAN, FIELD_TYPE_DOUBLE, FIELD_TYPE_FLOAT, FIELD_TYPE_INT8, FIELD_TYPE_INT16,
    FIELD_TYPE_INT32, FIELD_TYPE_INT64, FIELD_TYPE_NESTED_TYPE, FIELD_TYPE_STRING,
    FIELD_TYPE_UINT8, FIELD_TYPE_UINT16, FIELD_TYPE_UINT32, FIELD_TYPE_UINT64, FieldType,
    IndividualTypeDescription,
};

/// Trait for types that can be used as fields in ROS2 messages.
///
/// Each type knows how to describe itself as a ROS2 field type.
/// This provides a clean, type-safe way to map Rust types to ROS2 field types
/// without complex compile-time type path analysis.
pub trait RosFieldType {
    /// Returns the FieldType for this type when used as a field.
    fn ros_field_type() -> FieldType;

    /// Returns referenced type descriptions (empty for primitives, populated for nested types).
    fn referenced_types() -> Vec<IndividualTypeDescription> {
        vec![]
    }
}

// ============================================================================
// Primitive type implementations
// ============================================================================

impl RosFieldType for bool {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_BOOLEAN)
    }
}

impl RosFieldType for i8 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_INT8)
    }
}

impl RosFieldType for u8 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_UINT8)
    }
}

impl RosFieldType for i16 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_INT16)
    }
}

impl RosFieldType for u16 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_UINT16)
    }
}

impl RosFieldType for i32 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_INT32)
    }
}

impl RosFieldType for u32 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_UINT32)
    }
}

impl RosFieldType for i64 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_INT64)
    }
}

impl RosFieldType for u64 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_UINT64)
    }
}

impl RosFieldType for f32 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_FLOAT)
    }
}

impl RosFieldType for f64 {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_DOUBLE)
    }
}

impl RosFieldType for String {
    fn ros_field_type() -> FieldType {
        FieldType::primitive(FIELD_TYPE_STRING)
    }
}

// ============================================================================
// Vec<T> implementation - unbounded sequences
// ============================================================================

impl<T: RosFieldType> RosFieldType for Vec<T> {
    fn ros_field_type() -> FieldType {
        let inner = T::ros_field_type();
        if inner.type_id == FIELD_TYPE_NESTED_TYPE {
            FieldType::nested_sequence(&inner.nested_type_name)
        } else {
            FieldType::sequence(inner.type_id)
        }
    }

    fn referenced_types() -> Vec<IndividualTypeDescription> {
        T::referenced_types()
    }
}

// ============================================================================
// [T; N] implementation - fixed-size arrays
// ============================================================================

impl<T: RosFieldType, const N: usize> RosFieldType for [T; N] {
    fn ros_field_type() -> FieldType {
        let inner = T::ros_field_type();
        if inner.type_id == FIELD_TYPE_NESTED_TYPE {
            FieldType::nested_array(&inner.nested_type_name, N as u64)
        } else {
            FieldType::array(inner.type_id, N as u64)
        }
    }

    fn referenced_types() -> Vec<IndividualTypeDescription> {
        T::referenced_types()
    }
}
