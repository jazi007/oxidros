/// Core types for ROS2 message parsing
use std::collections::HashMap;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::errors::{ParseError, ParseResult, invalid_resource_name, invalid_type};
use crate::msg::validation::{
    ARRAY_UPPER_BOUND_TOKEN, PACKAGE_NAME_MESSAGE_TYPE_SEPARATOR, PRIMITIVE_TYPES, PrimitiveValue,
    STRING_UPPER_BOUND_TOKEN, is_valid_constant_name, is_valid_field_name, is_valid_message_name,
    is_valid_package_name, parse_primitive_value_string,
};

/// Annotations for fields, constants, and messages
pub type Annotations = HashMap<String, AnnotationValue>;

/// Annotation values can be strings, booleans, or lists of strings
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(missing_docs)]
pub enum AnnotationValue {
    String(String),
    Bool(bool),
    StringList(Vec<String>),
}

/// Base type information (without array specifiers)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BaseType {
    /// Package name for non-primitive types (None for primitive types)
    pub pkg_name: Option<String>,
    /// Type name (e.g., "string", "int32", "Pose")
    pub type_name: String,
    /// String upper bound for string/wstring types
    pub string_upper_bound: Option<u32>,
}

impl BaseType {
    /// Create a new `BaseType` from a type string
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The type string contains invalid format for bounded strings
    /// - The bound value is invalid or out of range
    /// - The type string format is invalid or unsupported
    pub fn new(type_string: &str, context_package_name: Option<&str>) -> ParseResult<Self> {
        // Check for primitive types
        if PRIMITIVE_TYPES.contains(&type_string) {
            return Ok(BaseType {
                pkg_name: None,
                type_name: type_string.to_string(),
                string_upper_bound: None,
            });
        }

        // Check for bounded string types
        if type_string.starts_with("string") && type_string.contains(STRING_UPPER_BOUND_TOKEN) {
            return Self::parse_bounded_string(type_string, "string");
        }
        if type_string.starts_with("wstring") && type_string.contains(STRING_UPPER_BOUND_TOKEN) {
            return Self::parse_bounded_string(type_string, "wstring");
        }

        // Parse non-primitive type
        let parts: Vec<&str> = type_string
            .split(PACKAGE_NAME_MESSAGE_TYPE_SEPARATOR)
            .collect();

        let (pkg_name, type_name) = match parts.len() {
            1 => {
                // Local type reference
                match context_package_name {
                    Some(pkg) => (Some(pkg.to_string()), parts[0].to_string()),
                    None => {
                        return Err(invalid_type(
                            type_string,
                            "non-primitive type requires package name or context package",
                        ));
                    }
                }
            }
            2 => {
                // Fully qualified type
                (Some(parts[0].to_string()), parts[1].to_string())
            }
            _ => return Err(invalid_type(type_string, "invalid type format")),
        };

        // Validate package name if present
        if let Some(ref pkg) = pkg_name
            && !is_valid_package_name(pkg)
        {
            return Err(invalid_resource_name(pkg, "valid package name pattern"));
        }

        // Validate message name
        if !is_valid_message_name(&type_name) {
            return Err(invalid_resource_name(
                &type_name,
                "valid message name pattern",
            ));
        }

        Ok(BaseType {
            pkg_name,
            type_name,
            string_upper_bound: None,
        })
    }

    fn parse_bounded_string(type_string: &str, base_type: &str) -> ParseResult<Self> {
        let parts: Vec<&str> = type_string.split(STRING_UPPER_BOUND_TOKEN).collect();
        if parts.len() != 2 {
            return Err(invalid_type(type_string, "invalid bounded string format"));
        }

        let upper_bound_str = parts[1];
        let upper_bound = upper_bound_str.parse::<u32>().map_err(|_| {
            invalid_type(
                type_string,
                "string upper bound must be a valid positive integer",
            )
        })?;

        if upper_bound == 0 {
            return Err(invalid_type(type_string, "string upper bound must be > 0"));
        }

        Ok(BaseType {
            pkg_name: None,
            type_name: base_type.to_string(),
            string_upper_bound: Some(upper_bound),
        })
    }

    /// Check if this is a primitive type
    #[must_use]
    pub fn is_primitive_type(&self) -> bool {
        self.pkg_name.is_none()
    }
}

impl fmt::Display for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref pkg) = self.pkg_name {
            write!(f, "{}/{}", pkg, self.type_name)
        } else {
            write!(f, "{}", self.type_name)?;
            if let Some(bound) = self.string_upper_bound {
                write!(f, "{STRING_UPPER_BOUND_TOKEN}{bound}")?;
            }
            Ok(())
        }
    }
}

/// Type information including array specifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Type {
    /// Base type information
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub base_type: BaseType,
    /// Whether this is an array type
    pub is_array: bool,
    /// Array size (None for dynamic arrays)
    pub array_size: Option<u32>,
    /// Whether `array_size` is an upper bound
    pub is_upper_bound: bool,
}

impl Type {
    /// Create a new Type from a type string
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The type string contains invalid array syntax
    /// - Array size bounds are invalid or out of range  
    /// - The base type string is invalid or unsupported
    pub fn new(type_string: &str, context_package_name: Option<&str>) -> ParseResult<Self> {
        // Check for array brackets
        let is_array = type_string.ends_with(']');
        let mut array_size = None;
        let mut is_upper_bound = false;
        let base_type_string;

        if is_array {
            // Find the opening bracket
            let bracket_start = type_string
                .rfind('[')
                .ok_or_else(|| invalid_type(type_string, "ends with ']' but missing '['"))?;

            let array_spec = &type_string[bracket_start + 1..type_string.len() - 1];
            base_type_string = &type_string[..bracket_start];

            // Parse array size if specified
            if !array_spec.is_empty() {
                // Check for upper bound specifier
                if let Some(size_str) = array_spec.strip_prefix(ARRAY_UPPER_BOUND_TOKEN) {
                    is_upper_bound = true;
                    array_size = Some(size_str.parse::<u32>().map_err(|_| {
                        invalid_type(type_string, "array size must be a valid positive integer")
                    })?);
                } else {
                    array_size = Some(array_spec.parse::<u32>().map_err(|_| {
                        invalid_type(type_string, "array size must be a valid positive integer")
                    })?);
                }

                // Validate array size
                if let Some(size) = array_size
                    && size == 0
                {
                    return Err(invalid_type(type_string, "array size must be > 0"));
                }
            }
        } else {
            base_type_string = type_string;
        }

        let base_type = BaseType::new(base_type_string, context_package_name)?;

        Ok(Type {
            base_type,
            is_array,
            array_size,
            is_upper_bound,
        })
    }

    /// Check if this is a primitive type
    #[must_use]
    pub fn is_primitive_type(&self) -> bool {
        self.base_type.is_primitive_type()
    }

    /// Check if this is a dynamic array (array without fixed size)
    #[must_use]
    pub fn is_dynamic_array(&self) -> bool {
        self.is_array && self.array_size.is_none()
    }

    /// Check if this is a bounded array (array with upper bound)
    #[must_use]
    pub fn is_bounded_array(&self) -> bool {
        self.is_array && self.is_upper_bound
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base_type)?;
        if self.is_array {
            write!(f, "[")?;
            if self.is_upper_bound {
                write!(f, "{ARRAY_UPPER_BOUND_TOKEN}")?;
            }
            if let Some(size) = self.array_size {
                write!(f, "{size}")?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

/// Value that can be assigned to fields or constants
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Value {
    /// Single primitive value
    Primitive(PrimitiveValue),
    /// Array of primitive values
    Array(Vec<PrimitiveValue>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Primitive(v) => write!(f, "{v}"),
            Value::Array(values) => {
                write!(f, "[")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
        }
    }
}

/// Constant definition
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Constant {
    /// Primitive type of the constant
    pub type_name: String,
    /// Name of the constant
    pub name: String,
    /// Value of the constant
    pub value: PrimitiveValue,
    /// Annotations attached to this constant
    pub annotations: Annotations,
}

impl Constant {
    /// Create a new constant
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The primitive type is not a valid ROS2 primitive type
    /// - The constant name doesn't follow valid naming conventions
    /// - The value string cannot be parsed for the given primitive type
    pub fn new(primitive_type: &str, name: &str, value_string: &str) -> ParseResult<Self> {
        if !PRIMITIVE_TYPES.contains(&primitive_type) {
            return Err(invalid_type(
                primitive_type,
                "constant type must be primitive",
            ));
        }

        if !is_valid_constant_name(name) {
            return Err(invalid_resource_name(name, "valid constant name pattern"));
        }

        let value = parse_primitive_value_string(primitive_type, value_string)?;

        Ok(Constant {
            type_name: primitive_type.to_string(),
            name: name.to_string(),
            value,
            annotations: HashMap::new(),
        })
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}={}", self.type_name, self.name, self.value)
    }
}

/// Field definition
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Field {
    /// Type of the field
    pub field_type: Type,
    /// Name of the field
    pub name: String,
    /// Default value (if any)
    pub default_value: Option<Value>,
    /// Annotations attached to this field
    pub annotations: Annotations,
}

impl Field {
    /// Create a new field
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The field name doesn't follow valid naming conventions
    /// - The default value string cannot be parsed for the field's type
    pub fn new(
        field_type: Type,
        name: &str,
        default_value_string: Option<&str>,
    ) -> ParseResult<Self> {
        if !is_valid_field_name(name) {
            return Err(invalid_resource_name(name, "valid field name pattern"));
        }

        let default_value = if let Some(value_str) = default_value_string {
            Some(parse_value_string(&field_type, value_str)?)
        } else {
            None
        };

        Ok(Field {
            field_type,
            name: name.to_string(),
            default_value,
            annotations: HashMap::new(),
        })
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.field_type, self.name)?;
        if let Some(ref value) = self.default_value {
            write!(f, " {value}")?;
        }
        Ok(())
    }
}

/// Parse value string for a given type
fn parse_value_string(type_: &Type, value_string: &str) -> ParseResult<Value> {
    if type_.is_primitive_type() && !type_.is_array {
        // Single primitive value
        let value = parse_primitive_value_string(&type_.base_type.type_name, value_string)?;
        Ok(Value::Primitive(value))
    } else if type_.is_primitive_type() && type_.is_array {
        // Array of primitive values
        let trimmed = value_string.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return Err(ParseError::InvalidValue {
                value: value_string.to_string(),
                type_info: type_.to_string(),
                reason: "array value must start with '[' and end with ']'".to_string(),
            });
        }

        let elements_string = &trimmed[1..trimmed.len() - 1];
        let value_strings: Vec<&str> = if elements_string.is_empty() {
            Vec::new()
        } else {
            elements_string.split(',').collect()
        };

        // Validate array size constraints
        if let Some(array_size) = type_.array_size {
            if !type_.is_upper_bound && value_strings.len() != array_size as usize {
                return Err(ParseError::InvalidValue {
                    value: value_string.to_string(),
                    type_info: type_.to_string(),
                    reason: format!(
                        "array must have exactly {} elements, not {}",
                        array_size,
                        value_strings.len()
                    ),
                });
            }
            if type_.is_upper_bound && value_strings.len() > array_size as usize {
                return Err(ParseError::InvalidValue {
                    value: value_string.to_string(),
                    type_info: type_.to_string(),
                    reason: format!(
                        "array must have not more than {} elements, not {}",
                        array_size,
                        value_strings.len()
                    ),
                });
            }
        }

        // Parse individual elements
        let mut values = Vec::new();
        for element_str in value_strings {
            let element_str = element_str.trim();
            let value = parse_primitive_value_string(&type_.base_type.type_name, element_str)?;
            values.push(value);
        }

        Ok(Value::Array(values))
    } else {
        Err(ParseError::InvalidValue {
            value: value_string.to_string(),
            type_info: type_.to_string(),
            reason: "only primitive types and primitive arrays can have default values".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_type_creation() {
        // Primitive type
        let base_type = BaseType::new("int32", None).unwrap();
        assert!(base_type.is_primitive_type());
        assert_eq!(base_type.type_name, "int32");

        // Bounded string
        let base_type = BaseType::new("string<=10", None).unwrap();
        assert!(base_type.is_primitive_type());
        assert_eq!(base_type.string_upper_bound, Some(10));

        // Fully qualified type
        let base_type = BaseType::new("geometry_msgs/Pose", None).unwrap();
        assert!(!base_type.is_primitive_type());
        assert_eq!(base_type.pkg_name, Some("geometry_msgs".to_string()));
        assert_eq!(base_type.type_name, "Pose");
    }

    #[test]
    fn test_type_creation() {
        // Simple array
        let type_ = Type::new("int32[5]", None).unwrap();
        assert!(type_.is_array);
        assert_eq!(type_.array_size, Some(5));
        assert!(!type_.is_upper_bound);

        // Bounded array
        let type_ = Type::new("float64[<=10]", None).unwrap();
        assert!(type_.is_array);
        assert_eq!(type_.array_size, Some(10));
        assert!(type_.is_upper_bound);

        // Dynamic array
        let type_ = Type::new("string[]", None).unwrap();
        assert!(type_.is_array);
        assert!(type_.is_dynamic_array());
    }

    #[test]
    fn test_constant_creation() {
        let constant = Constant::new("int32", "MAX_VALUE", "100").unwrap();
        assert_eq!(constant.name, "MAX_VALUE");
        assert_eq!(constant.value, PrimitiveValue::Int32(100));
    }

    #[test]
    fn test_field_creation() {
        let type_ = Type::new("string", None).unwrap();
        let field = Field::new(type_, "name", Some("\"default\"")).unwrap();
        assert_eq!(field.name, "name");
        assert!(field.default_value.is_some());
    }

    #[test]
    fn test_base_type_display() {
        let bt = BaseType::new("int32", None).unwrap();
        assert_eq!(bt.to_string(), "int32");

        let bt = BaseType::new("geometry_msgs/Pose", None).unwrap();
        assert_eq!(bt.to_string(), "geometry_msgs/Pose");

        let bt = BaseType::new("string<=50", None).unwrap();
        assert_eq!(bt.to_string(), "string<=50");
    }

    #[test]
    fn test_type_display() {
        let t = Type::new("int32", None).unwrap();
        assert_eq!(t.to_string(), "int32");

        let t = Type::new("int32[10]", None).unwrap();
        assert_eq!(t.to_string(), "int32[10]");

        let t = Type::new("int32[]", None).unwrap();
        assert_eq!(t.to_string(), "int32[]");

        let t = Type::new("int32[<=100]", None).unwrap();
        assert_eq!(t.to_string(), "int32[<=100]");
    }

    #[test]
    fn test_annotation_value_variants() {
        let val = AnnotationValue::String("test".to_string());
        assert!(matches!(val, AnnotationValue::String(_)));

        let val = AnnotationValue::Bool(true);
        assert!(matches!(val, AnnotationValue::Bool(true)));

        let val = AnnotationValue::StringList(vec!["a".to_string(), "b".to_string()]);
        assert!(matches!(val, AnnotationValue::StringList(_)));
    }

    #[test]
    fn test_value_display() {
        let val = Value::Primitive(PrimitiveValue::Int32(42));
        assert_eq!(val.to_string(), "42");

        let val = Value::Array(vec![PrimitiveValue::Int32(1), PrimitiveValue::Int32(2)]);
        assert_eq!(val.to_string(), "[1, 2]");
    }

    #[test]
    fn test_type_is_methods() {
        let t = Type::new("int32[]", None).unwrap();
        assert!(t.is_dynamic_array());
        assert!(!t.is_bounded_array());

        let t = Type::new("int32[<=10]", None).unwrap();
        assert!(t.is_bounded_array());
        assert!(!t.is_dynamic_array());
    }
}
