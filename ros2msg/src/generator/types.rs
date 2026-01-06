//! Type mapping from ROS2 types to Rust types

use super::callbacks::ParseCallbacks;
use crate::{BaseType, Type};

/// Maps ROS2 types to Rust types
pub struct TypeMapper {
    /// Prefix for C types
    ctypes_prefix: String,
}

impl TypeMapper {
    /// Create a new type mapper with default C types prefix
    #[must_use]
    pub fn new() -> Self {
        Self {
            ctypes_prefix: "std::os::raw".to_string(),
        }
    }

    /// Create a new type mapper with custom C types prefix
    pub fn with_ctypes_prefix(prefix: impl AsRef<str>) -> Self {
        Self {
            ctypes_prefix: prefix.as_ref().to_string(),
        }
    }

    /// Get the ctypes prefix
    #[must_use]
    pub fn get_ctypes_prefix(&self) -> &str {
        &self.ctypes_prefix
    }

    /// Map a ROS2 Type to Rust type string
    #[must_use]
    pub fn map_type(&self, ros_type: &Type) -> String {
        let base_rust_type = self.map_base_type(&ros_type.base_type);

        if ros_type.is_array {
            if let Some(size) = ros_type.array_size {
                if ros_type.is_upper_bound {
                    // Bounded sequence (dynamic array with max size)
                    format!("Vec<{base_rust_type}> /* max_size: {size} */")
                } else {
                    // Fixed-size array
                    format!("[{base_rust_type}; {size}]")
                }
            } else {
                // Unbounded sequence (dynamic array)
                format!("Vec<{base_rust_type}>")
            }
        } else {
            base_rust_type
        }
    }

    /// Check if a type needs the `serde_big_array` attribute for large arrays
    #[must_use]
    pub fn needs_big_array_attr(&self, ros_type: &Type) -> bool {
        ros_type.is_array
            && !ros_type.is_upper_bound
            && ros_type.array_size.is_some_and(|size| size > 32)
    }

    /// Map a base type to Rust primitive type
    #[must_use]
    pub fn map_base_type_in_context(&self, base_type: &BaseType) -> String {
        // Check if it's a nested type (has package)
        if let Some(pkg) = &base_type.pkg_name {
            if pkg.is_empty() {
                return base_type.type_name.clone();
            }

            // Return the bare type name
            // The caller (codegen) is responsible for adding the full path or importing it
            return base_type.type_name.clone();
        }

        // Map primitive types
        let type_name = base_type.type_name.as_str();
        match type_name {
            "bool" => "bool".to_string(),
            "byte" | "uint8" => "u8".to_string(),
            "char" => format!("{}::c_char", self.ctypes_prefix),
            "float32" => "f32".to_string(),
            "float64" => "f64".to_string(),
            "int8" => "i8".to_string(),
            "int16" => "i16".to_string(),
            "uint16" => "u16".to_string(),
            "int32" => "i32".to_string(),
            "uint32" => "u32".to_string(),
            "int64" => "i64".to_string(),
            "uint64" => "u64".to_string(),
            "string" => {
                // Use fully qualified path to avoid conflicts with generated String types
                if let Some(bound) = base_type.string_upper_bound {
                    format!("::std::string::String /* max_size: {bound} */")
                } else {
                    "::std::string::String".to_string()
                }
            }
            "wstring" => {
                // Use fully qualified path to avoid conflicts with generated String types
                if let Some(bound) = base_type.string_upper_bound {
                    format!("::std::string::String /* max_size: {bound}, wstring */")
                } else {
                    "::std::string::String /* wstring */".to_string()
                }
            }
            // If not a primitive, it's a custom type
            _ => base_type.type_name.clone(),
        }
    }

    /// Map a base type to Rust primitive type (legacy, no package context)
    #[must_use]
    pub fn map_base_type(&self, base_type: &BaseType) -> String {
        self.map_base_type_in_context(base_type)
    }

    /// Map a ROS2 Type to Rust type string with package context
    #[must_use]
    pub fn map_type_in_context(&self, ros_type: &Type) -> String {
        self.map_type_in_context_with_callbacks(ros_type, None)
    }

    /// Map a ROS2 Type to Rust type string with package context and optional callbacks
    #[must_use]
    pub fn map_type_in_context_with_callbacks(
        &self,
        ros_type: &Type,
        callbacks: Option<&dyn ParseCallbacks>,
    ) -> String {
        let base_rust_type = self.map_base_type_with_callbacks(&ros_type.base_type, callbacks);

        if ros_type.is_array {
            if let Some(size) = ros_type.array_size {
                if ros_type.is_upper_bound {
                    // Bounded sequence (dynamic array with max size)
                    // Check for callback override
                    if let Some(cb) = callbacks
                        && let Some(custom_type) = cb.sequence_type(&base_rust_type, Some(size))
                    {
                        return custom_type;
                    }
                    format!("Vec<{base_rust_type}> /* max_size: {size} */")
                } else {
                    // Fixed-size array (not a sequence, no callback)
                    format!("[{base_rust_type}; {size}]")
                }
            } else {
                // Unbounded sequence (dynamic array)
                // Check for callback override
                if let Some(cb) = callbacks
                    && let Some(custom_type) = cb.sequence_type(&base_rust_type, None)
                {
                    return custom_type;
                }
                format!("Vec<{base_rust_type}>")
            }
        } else {
            base_rust_type
        }
    }

    /// Map a base type to Rust primitive type with optional callbacks
    #[must_use]
    pub fn map_base_type_with_callbacks(
        &self,
        base_type: &BaseType,
        callbacks: Option<&dyn ParseCallbacks>,
    ) -> String {
        // Check if it's a nested type (has package)
        if let Some(pkg) = &base_type.pkg_name {
            if pkg.is_empty() {
                return base_type.type_name.clone();
            }

            // Return the bare type name
            // The caller (codegen) is responsible for adding the full path or importing it
            return base_type.type_name.clone();
        }

        // Map primitive types
        let type_name = base_type.type_name.as_str();
        match type_name {
            "bool" => "bool".to_string(),
            "byte" | "uint8" => "u8".to_string(),
            "char" => format!("{}::c_char", self.ctypes_prefix),
            "float32" => "f32".to_string(),
            "float64" => "f64".to_string(),
            "int8" => "i8".to_string(),
            "int16" => "i16".to_string(),
            "uint16" => "u16".to_string(),
            "int32" => "i32".to_string(),
            "uint32" => "u32".to_string(),
            "int64" => "i64".to_string(),
            "uint64" => "u64".to_string(),
            "string" => {
                // Check for callback override
                if let Some(cb) = callbacks
                    && let Some(custom_type) = cb.string_type(base_type.string_upper_bound)
                {
                    return custom_type;
                }
                // Use fully qualified path to avoid conflicts with generated String types
                if let Some(bound) = base_type.string_upper_bound {
                    format!("::std::string::String /* max_size: {bound} */")
                } else {
                    "::std::string::String".to_string()
                }
            }
            "wstring" => {
                // Check for callback override
                if let Some(cb) = callbacks
                    && let Some(custom_type) = cb.wstring_type(base_type.string_upper_bound)
                {
                    return custom_type;
                }
                // Use fully qualified path to avoid conflicts with generated String types
                if let Some(bound) = base_type.string_upper_bound {
                    format!("::std::string::String /* max_size: {bound}, wstring */")
                } else {
                    "::std::string::String /* wstring */".to_string()
                }
            }
            // If not a primitive, it's a custom type
            _ => base_type.type_name.clone(),
        }
    }
}

impl Default for TypeMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_primitive_types() {
        let mapper = TypeMapper::new();

        let bool_type = BaseType {
            pkg_name: None,
            type_name: "bool".to_string(),
            string_upper_bound: None,
        };
        assert_eq!(mapper.map_base_type(&bool_type), "bool");

        let int32_type = BaseType {
            pkg_name: None,
            type_name: "int32".to_string(),
            string_upper_bound: None,
        };
        assert_eq!(mapper.map_base_type(&int32_type), "i32");

        let uint64_type = BaseType {
            pkg_name: None,
            type_name: "uint64".to_string(),
            string_upper_bound: None,
        };
        assert_eq!(mapper.map_base_type(&uint64_type), "u64");
    }

    #[test]
    fn test_map_array_type() {
        let mapper = TypeMapper::new();
        let array_type = Type {
            base_type: BaseType {
                pkg_name: None,
                type_name: "int32".to_string(),
                string_upper_bound: None,
            },
            is_array: true,
            array_size: Some(10),
            is_upper_bound: false,
        };

        assert_eq!(mapper.map_type(&array_type), "[i32; 10]");
    }

    #[test]
    fn test_map_sequence_types() {
        let mapper = TypeMapper::new();

        // Unbounded sequence
        let unbounded = Type {
            base_type: BaseType {
                pkg_name: None,
                type_name: "float64".to_string(),
                string_upper_bound: None,
            },
            is_array: true,
            array_size: None,
            is_upper_bound: false,
        };
        assert_eq!(mapper.map_type(&unbounded), "Vec<f64>");

        // Bounded sequence
        let bounded = Type {
            base_type: BaseType {
                pkg_name: None,
                type_name: "uint8".to_string(),
                string_upper_bound: None,
            },
            is_array: true,
            array_size: Some(100),
            is_upper_bound: true,
        };
        let result = mapper.map_type(&bounded);
        assert!(result.contains("Vec<u8>"));
        assert!(result.contains("100"));
    }

    #[test]
    fn test_map_nested_type() {
        let mapper = TypeMapper::new();

        // Nested type with package - should include msg submodule
        let nested = Type {
            base_type: BaseType {
                pkg_name: Some("std_msgs".to_string()),
                type_name: "Header".to_string(),
                string_upper_bound: None,
            },
            is_array: false,
            array_size: None,
            is_upper_bound: false,
        };
        assert_eq!(mapper.map_type(&nested), "Header");

        // Nested type without package
        let no_namespace = Type {
            base_type: BaseType {
                pkg_name: Some(String::new()),
                type_name: "CustomType".to_string(),
                string_upper_bound: None,
            },
            is_array: false,
            array_size: None,
            is_upper_bound: false,
        };
        assert_eq!(mapper.map_type(&no_namespace), "CustomType");
    }

    #[test]
    fn test_bounded_strings() {
        let mapper = TypeMapper::new();

        let bounded_string = BaseType {
            pkg_name: None,
            type_name: "string".to_string(),
            string_upper_bound: Some(256),
        };
        let result = mapper.map_base_type(&bounded_string);
        assert!(result.contains("String"));
        assert!(result.contains("256"));
    }
}
