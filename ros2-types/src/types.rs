//! Type description data structures
//!
//! These structures match the ROS2 type_description_interfaces

use serde::{Deserialize, Serialize};

/// Complete type description message including referenced types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDescriptionMsg {
    /// The main type being described
    pub type_description: IndividualTypeDescription,
    /// All types referenced by the main type
    pub referenced_type_descriptions: Vec<IndividualTypeDescription>,
}

/// Description of a single type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualTypeDescription {
    /// Fully qualified type name (e.g., "std_msgs/msg/Header")
    pub type_name: String,
    /// Fields in this type
    pub fields: Vec<Field>,
}

/// Description of a field in a type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Field name
    pub name: String,
    /// Field type information
    #[serde(rename = "type")]
    pub field_type: FieldType,
    /// Default value (empty string if none)
    pub default_value: String,
}

/// Type information for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldType {
    /// Type ID from FieldType.msg constants
    pub type_id: u8,
    /// Array/sequence capacity (0 if not applicable)
    pub capacity: u64,
    /// String capacity (0 if not applicable)
    pub string_capacity: u64,
    /// Nested type name (empty if not a nested type)
    pub nested_type_name: String,
}

// Field type constants matching type_description_interfaces/msg/FieldType.msg
pub const FIELD_TYPE_NOT_SET: u8 = 0;
pub const FIELD_TYPE_NESTED_TYPE: u8 = 1;
pub const FIELD_TYPE_INT8: u8 = 2;
pub const FIELD_TYPE_UINT8: u8 = 3;
pub const FIELD_TYPE_INT16: u8 = 4;
pub const FIELD_TYPE_UINT16: u8 = 5;
pub const FIELD_TYPE_INT32: u8 = 6;
pub const FIELD_TYPE_UINT32: u8 = 7;
pub const FIELD_TYPE_INT64: u8 = 8;
pub const FIELD_TYPE_UINT64: u8 = 9;
pub const FIELD_TYPE_FLOAT: u8 = 10;
pub const FIELD_TYPE_DOUBLE: u8 = 11;
pub const FIELD_TYPE_LONG_DOUBLE: u8 = 12;
pub const FIELD_TYPE_CHAR: u8 = 13;
pub const FIELD_TYPE_WCHAR: u8 = 14;
pub const FIELD_TYPE_BOOLEAN: u8 = 15;
pub const FIELD_TYPE_BYTE: u8 = 16;
pub const FIELD_TYPE_STRING: u8 = 17;
pub const FIELD_TYPE_WSTRING: u8 = 18;
pub const FIELD_TYPE_FIXED_STRING: u8 = 19;
pub const FIELD_TYPE_FIXED_WSTRING: u8 = 20;
pub const FIELD_TYPE_BOUNDED_STRING: u8 = 21;
pub const FIELD_TYPE_BOUNDED_WSTRING: u8 = 22;

// Fixed-size arrays (49-96)
pub const FIELD_TYPE_NESTED_TYPE_ARRAY: u8 = 49;
pub const FIELD_TYPE_INT8_ARRAY: u8 = 50;
pub const FIELD_TYPE_UINT8_ARRAY: u8 = 51;
pub const FIELD_TYPE_INT16_ARRAY: u8 = 52;
pub const FIELD_TYPE_UINT16_ARRAY: u8 = 53;
pub const FIELD_TYPE_INT32_ARRAY: u8 = 54;
pub const FIELD_TYPE_UINT32_ARRAY: u8 = 55;
pub const FIELD_TYPE_INT64_ARRAY: u8 = 56;
pub const FIELD_TYPE_UINT64_ARRAY: u8 = 57;
pub const FIELD_TYPE_FLOAT_ARRAY: u8 = 58;
pub const FIELD_TYPE_DOUBLE_ARRAY: u8 = 59;
pub const FIELD_TYPE_LONG_DOUBLE_ARRAY: u8 = 60;
pub const FIELD_TYPE_CHAR_ARRAY: u8 = 61;
pub const FIELD_TYPE_WCHAR_ARRAY: u8 = 62;
pub const FIELD_TYPE_BOOLEAN_ARRAY: u8 = 63;
pub const FIELD_TYPE_BYTE_ARRAY: u8 = 64;
pub const FIELD_TYPE_STRING_ARRAY: u8 = 65;
pub const FIELD_TYPE_WSTRING_ARRAY: u8 = 66;

// Bounded sequences (97-144)
pub const FIELD_TYPE_NESTED_TYPE_BOUNDED_SEQUENCE: u8 = 97;
pub const FIELD_TYPE_INT8_BOUNDED_SEQUENCE: u8 = 98;
pub const FIELD_TYPE_UINT8_BOUNDED_SEQUENCE: u8 = 99;
pub const FIELD_TYPE_INT16_BOUNDED_SEQUENCE: u8 = 100;
pub const FIELD_TYPE_UINT16_BOUNDED_SEQUENCE: u8 = 101;
pub const FIELD_TYPE_INT32_BOUNDED_SEQUENCE: u8 = 102;
pub const FIELD_TYPE_UINT32_BOUNDED_SEQUENCE: u8 = 103;
pub const FIELD_TYPE_INT64_BOUNDED_SEQUENCE: u8 = 104;
pub const FIELD_TYPE_UINT64_BOUNDED_SEQUENCE: u8 = 105;
pub const FIELD_TYPE_FLOAT_BOUNDED_SEQUENCE: u8 = 106;
pub const FIELD_TYPE_DOUBLE_BOUNDED_SEQUENCE: u8 = 107;
pub const FIELD_TYPE_LONG_DOUBLE_BOUNDED_SEQUENCE: u8 = 108;
pub const FIELD_TYPE_CHAR_BOUNDED_SEQUENCE: u8 = 109;
pub const FIELD_TYPE_WCHAR_BOUNDED_SEQUENCE: u8 = 110;
pub const FIELD_TYPE_BOOLEAN_BOUNDED_SEQUENCE: u8 = 111;
pub const FIELD_TYPE_BYTE_BOUNDED_SEQUENCE: u8 = 112;
pub const FIELD_TYPE_STRING_BOUNDED_SEQUENCE: u8 = 113;
pub const FIELD_TYPE_WSTRING_BOUNDED_SEQUENCE: u8 = 114;

// Unbounded sequences (145-192)
pub const FIELD_TYPE_NESTED_TYPE_UNBOUNDED_SEQUENCE: u8 = 145;
pub const FIELD_TYPE_INT8_UNBOUNDED_SEQUENCE: u8 = 146;
pub const FIELD_TYPE_UINT8_UNBOUNDED_SEQUENCE: u8 = 147;
pub const FIELD_TYPE_INT16_UNBOUNDED_SEQUENCE: u8 = 148;
pub const FIELD_TYPE_UINT16_UNBOUNDED_SEQUENCE: u8 = 149;
pub const FIELD_TYPE_INT32_UNBOUNDED_SEQUENCE: u8 = 150;
pub const FIELD_TYPE_UINT32_UNBOUNDED_SEQUENCE: u8 = 151;
pub const FIELD_TYPE_INT64_UNBOUNDED_SEQUENCE: u8 = 152;
pub const FIELD_TYPE_UINT64_UNBOUNDED_SEQUENCE: u8 = 153;
pub const FIELD_TYPE_FLOAT_UNBOUNDED_SEQUENCE: u8 = 154;
pub const FIELD_TYPE_DOUBLE_UNBOUNDED_SEQUENCE: u8 = 155;
pub const FIELD_TYPE_LONG_DOUBLE_UNBOUNDED_SEQUENCE: u8 = 156;
pub const FIELD_TYPE_CHAR_UNBOUNDED_SEQUENCE: u8 = 157;
pub const FIELD_TYPE_WCHAR_UNBOUNDED_SEQUENCE: u8 = 158;
pub const FIELD_TYPE_BOOLEAN_UNBOUNDED_SEQUENCE: u8 = 159;
pub const FIELD_TYPE_BYTE_UNBOUNDED_SEQUENCE: u8 = 160;
pub const FIELD_TYPE_STRING_UNBOUNDED_SEQUENCE: u8 = 161;
pub const FIELD_TYPE_WSTRING_UNBOUNDED_SEQUENCE: u8 = 162;

impl FieldType {
    /// Create a primitive field type
    pub fn primitive(type_id: u8) -> Self {
        Self {
            type_id,
            capacity: 0,
            string_capacity: 0,
            nested_type_name: String::new(),
        }
    }

    /// Create a nested type field
    pub fn nested(type_name: impl Into<String>) -> Self {
        Self {
            type_id: FIELD_TYPE_NESTED_TYPE,
            capacity: 0,
            string_capacity: 0,
            nested_type_name: type_name.into(),
        }
    }

    /// Create an unbounded sequence of nested types (Vec<NestedType>)
    pub fn nested_sequence(type_name: impl Into<String>) -> Self {
        Self {
            type_id: FIELD_TYPE_NESTED_TYPE_UNBOUNDED_SEQUENCE,
            capacity: 0,
            string_capacity: 0,
            nested_type_name: type_name.into(),
        }
    }

    /// Create a fixed-size array field type (e.g., [T; N])
    /// For nested types: [NestedType; N]
    /// For primitives: [primitive; N]
    pub fn array(base_type_id: u8, capacity: u64) -> Self {
        let array_type_id = match base_type_id {
            FIELD_TYPE_INT8 => FIELD_TYPE_INT8_ARRAY,
            FIELD_TYPE_UINT8 => FIELD_TYPE_UINT8_ARRAY,
            FIELD_TYPE_INT16 => FIELD_TYPE_INT16_ARRAY,
            FIELD_TYPE_UINT16 => FIELD_TYPE_UINT16_ARRAY,
            FIELD_TYPE_INT32 => FIELD_TYPE_INT32_ARRAY,
            FIELD_TYPE_UINT32 => FIELD_TYPE_UINT32_ARRAY,
            FIELD_TYPE_INT64 => FIELD_TYPE_INT64_ARRAY,
            FIELD_TYPE_UINT64 => FIELD_TYPE_UINT64_ARRAY,
            FIELD_TYPE_FLOAT => FIELD_TYPE_FLOAT_ARRAY,
            FIELD_TYPE_DOUBLE => FIELD_TYPE_DOUBLE_ARRAY,
            FIELD_TYPE_LONG_DOUBLE => FIELD_TYPE_LONG_DOUBLE_ARRAY,
            FIELD_TYPE_CHAR => FIELD_TYPE_CHAR_ARRAY,
            FIELD_TYPE_WCHAR => FIELD_TYPE_WCHAR_ARRAY,
            FIELD_TYPE_BOOLEAN => FIELD_TYPE_BOOLEAN_ARRAY,
            FIELD_TYPE_BYTE => FIELD_TYPE_BYTE_ARRAY,
            FIELD_TYPE_STRING => FIELD_TYPE_STRING_ARRAY,
            FIELD_TYPE_WSTRING => FIELD_TYPE_WSTRING_ARRAY,
            _ => base_type_id, // Fallback
        };
        Self {
            type_id: array_type_id,
            capacity,
            string_capacity: 0,
            nested_type_name: String::new(),
        }
    }

    /// Create a fixed-size array of nested types
    pub fn nested_array(type_name: impl Into<String>, capacity: u64) -> Self {
        Self {
            type_id: FIELD_TYPE_NESTED_TYPE_ARRAY,
            capacity,
            string_capacity: 0,
            nested_type_name: type_name.into(),
        }
    }

    /// Create an unbounded sequence (Vec) of primitives
    pub fn sequence(base_type_id: u8) -> Self {
        let sequence_type_id = match base_type_id {
            FIELD_TYPE_INT8 => FIELD_TYPE_INT8_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_UINT8 => FIELD_TYPE_UINT8_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_INT16 => FIELD_TYPE_INT16_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_UINT16 => FIELD_TYPE_UINT16_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_INT32 => FIELD_TYPE_INT32_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_UINT32 => FIELD_TYPE_UINT32_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_INT64 => FIELD_TYPE_INT64_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_UINT64 => FIELD_TYPE_UINT64_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_FLOAT => FIELD_TYPE_FLOAT_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_DOUBLE => FIELD_TYPE_DOUBLE_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_LONG_DOUBLE => FIELD_TYPE_LONG_DOUBLE_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_CHAR => FIELD_TYPE_CHAR_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_WCHAR => FIELD_TYPE_WCHAR_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_BOOLEAN => FIELD_TYPE_BOOLEAN_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_BYTE => FIELD_TYPE_BYTE_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_STRING => FIELD_TYPE_STRING_UNBOUNDED_SEQUENCE,
            FIELD_TYPE_WSTRING => FIELD_TYPE_WSTRING_UNBOUNDED_SEQUENCE,
            _ => base_type_id, // Fallback
        };
        Self {
            type_id: sequence_type_id,
            capacity: 0,
            string_capacity: 0,
            nested_type_name: String::new(),
        }
    }

    /// Create a string field type with capacity
    pub fn string_with_capacity(type_id: u8, string_capacity: u64) -> Self {
        Self {
            type_id,
            capacity: 0,
            string_capacity,
            nested_type_name: String::new(),
        }
    }

    /// Create a bounded string (string with maximum size)
    pub fn bounded_string(string_capacity: u64) -> Self {
        Self {
            type_id: FIELD_TYPE_BOUNDED_STRING,
            capacity: 0,
            string_capacity,
            nested_type_name: String::new(),
        }
    }

    /// Create a bounded wstring (wide string with maximum size)
    pub fn bounded_wstring(string_capacity: u64) -> Self {
        Self {
            type_id: FIELD_TYPE_BOUNDED_WSTRING,
            capacity: 0,
            string_capacity,
            nested_type_name: String::new(),
        }
    }

    /// Create a bounded sequence of primitives
    pub fn bounded_sequence(base_type_id: u8, capacity: u64) -> Self {
        let sequence_type_id = match base_type_id {
            FIELD_TYPE_INT8 => FIELD_TYPE_INT8_BOUNDED_SEQUENCE,
            FIELD_TYPE_UINT8 => FIELD_TYPE_UINT8_BOUNDED_SEQUENCE,
            FIELD_TYPE_INT16 => FIELD_TYPE_INT16_BOUNDED_SEQUENCE,
            FIELD_TYPE_UINT16 => FIELD_TYPE_UINT16_BOUNDED_SEQUENCE,
            FIELD_TYPE_INT32 => FIELD_TYPE_INT32_BOUNDED_SEQUENCE,
            FIELD_TYPE_UINT32 => FIELD_TYPE_UINT32_BOUNDED_SEQUENCE,
            FIELD_TYPE_INT64 => FIELD_TYPE_INT64_BOUNDED_SEQUENCE,
            FIELD_TYPE_UINT64 => FIELD_TYPE_UINT64_BOUNDED_SEQUENCE,
            FIELD_TYPE_FLOAT => FIELD_TYPE_FLOAT_BOUNDED_SEQUENCE,
            FIELD_TYPE_DOUBLE => FIELD_TYPE_DOUBLE_BOUNDED_SEQUENCE,
            FIELD_TYPE_LONG_DOUBLE => FIELD_TYPE_LONG_DOUBLE_BOUNDED_SEQUENCE,
            FIELD_TYPE_CHAR => FIELD_TYPE_CHAR_BOUNDED_SEQUENCE,
            FIELD_TYPE_WCHAR => FIELD_TYPE_WCHAR_BOUNDED_SEQUENCE,
            FIELD_TYPE_BOOLEAN => FIELD_TYPE_BOOLEAN_BOUNDED_SEQUENCE,
            FIELD_TYPE_BYTE => FIELD_TYPE_BYTE_BOUNDED_SEQUENCE,
            FIELD_TYPE_STRING => FIELD_TYPE_STRING_BOUNDED_SEQUENCE,
            FIELD_TYPE_WSTRING => FIELD_TYPE_WSTRING_BOUNDED_SEQUENCE,
            _ => base_type_id, // Fallback
        };
        Self {
            type_id: sequence_type_id,
            capacity,
            string_capacity: 0,
            nested_type_name: String::new(),
        }
    }

    /// Create a bounded sequence with string_capacity for bounded string elements
    /// Used for sequence<string<M>, N> where M is string_capacity and N is capacity
    pub fn bounded_sequence_with_string_capacity(
        base_type_id: u8,
        capacity: u64,
        string_capacity: u64,
    ) -> Self {
        let mut ft = Self::bounded_sequence(base_type_id, capacity);
        ft.string_capacity = string_capacity;
        ft
    }

    /// Create an unbounded sequence with string_capacity for bounded string elements
    /// Used for sequence<string<M>> where M is string_capacity
    pub fn sequence_with_string_capacity(base_type_id: u8, string_capacity: u64) -> Self {
        let mut ft = Self::sequence(base_type_id);
        ft.string_capacity = string_capacity;
        ft
    }

    /// Create a bounded sequence of nested types
    pub fn nested_bounded_sequence(type_name: impl Into<String>, capacity: u64) -> Self {
        Self {
            type_id: FIELD_TYPE_NESTED_TYPE_BOUNDED_SEQUENCE,
            capacity,
            string_capacity: 0,
            nested_type_name: type_name.into(),
        }
    }
}

impl Field {
    /// Create a new field
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            default_value: String::new(),
        }
    }

    /// Create a new field with a default value
    pub fn with_default(
        name: impl Into<String>,
        field_type: FieldType,
        default_value: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            field_type,
            default_value: default_value.into(),
        }
    }
}

impl IndividualTypeDescription {
    /// Create a new type description
    pub fn new(type_name: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            type_name: type_name.into(),
            fields,
        }
    }
}

impl TypeDescriptionMsg {
    /// Create a new type description message
    pub fn new(
        type_description: IndividualTypeDescription,
        referenced_type_descriptions: Vec<IndividualTypeDescription>,
    ) -> Self {
        Self {
            type_description,
            referenced_type_descriptions,
        }
    }
}
