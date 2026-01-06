//! IDL AST Types
//!
//! This module contains Rust equivalents of the Python IDL definition types,
//! providing a complete Abstract Syntax Tree for ROS2 IDL files.

use std::path::PathBuf;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::values::IdlValue;

// ============================================================================
// Basic Type Constants (from definition.py)
// ============================================================================

/// Signed non-explicit integer types (short, long, long long)
pub const SIGNED_NONEXPLICIT_INTEGER_TYPES: &[&str] = &["short", "long", "long long"];

/// Unsigned non-explicit integer types (unsigned short, etc.)
pub const UNSIGNED_NONEXPLICIT_INTEGER_TYPES: &[&str] =
    &["unsigned short", "unsigned long", "unsigned long long"];

/// Floating point types
pub const FLOATING_POINT_TYPES: &[&str] = &["float", "double", "long double"];

/// Character types
pub const CHARACTER_TYPES: &[&str] = &["char", "wchar"];

/// Boolean type
pub const BOOLEAN_TYPE: &str = "boolean";

/// Octet type  
pub const OCTET_TYPE: &str = "octet";

/// Signed explicit integer types (int8, int16, etc.)
pub const SIGNED_EXPLICIT_INTEGER_TYPES: &[&str] = &["int8", "int16", "int32", "int64"];

/// Unsigned explicit integer types (uint8, uint16, etc.)
pub const UNSIGNED_EXPLICIT_INTEGER_TYPES: &[&str] = &["uint8", "uint16", "uint32", "uint64"];

/// Service message suffixes
/// Suffix for service request message types
pub const SERVICE_REQUEST_MESSAGE_SUFFIX: &str = "_Request";
/// Suffix for service response message types
pub const SERVICE_RESPONSE_MESSAGE_SUFFIX: &str = "_Response";
/// Suffix for service event message types
pub const SERVICE_EVENT_MESSAGE_SUFFIX: &str = "_Event";

/// Action message suffixes
/// Suffix for action goal types
pub const ACTION_GOAL_SUFFIX: &str = "_Goal";
/// Suffix for action result types
pub const ACTION_RESULT_SUFFIX: &str = "_Result";
/// Suffix for action feedback types
pub const ACTION_FEEDBACK_SUFFIX: &str = "_Feedback";
/// Suffix for action goal service types
pub const ACTION_GOAL_SERVICE_SUFFIX: &str = "_SendGoal";
/// Suffix for action result service types
pub const ACTION_RESULT_SERVICE_SUFFIX: &str = "_GetResult";
/// Suffix for action feedback message types
pub const ACTION_FEEDBACK_MESSAGE_SUFFIX: &str = "_FeedbackMessage";

/// Empty structure required member name
pub const EMPTY_STRUCTURE_REQUIRED_MEMBER_NAME: &str = "structure_needs_at_least_one_member";

// ============================================================================
// Core Type System
// ============================================================================

/// Core IDL type system using enums instead of trait objects
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum IdlType {
    /// Basic primitive type (int32, float64, etc.)
    Basic(BasicType),
    /// Named user-defined type
    Named(NamedType),
    /// Namespaced type with scope
    Namespaced(NamespacedType),
    /// String type (bounded or unbounded)
    String(AbstractString),
    /// Wide string type (bounded or unbounded)
    WString(AbstractWString),
    /// Array type with static size
    Array(Array),
    /// Bounded sequence type
    BoundedSequence(BoundedSequence),
    /// Unbounded sequence type
    UnboundedSequence(UnboundedSequence),
}

impl IdlType {
    /// Check if this type has a maximum size
    #[must_use]
    pub fn has_maximum_size(&self) -> bool {
        match self {
            IdlType::String(s) => s.has_maximum_size(),
            IdlType::WString(s) => s.has_maximum_size(),
            IdlType::UnboundedSequence(_) => false,
            IdlType::Basic(_)
            | IdlType::Named(_)
            | IdlType::Namespaced(_)
            | IdlType::Array(_)
            | IdlType::BoundedSequence(_) => true,
        }
    }

    /// Check if this is a primitive type
    #[must_use]
    pub fn is_primitive(&self) -> bool {
        matches!(self, IdlType::Basic(_))
    }
}

// ============================================================================
// Basic Types
// ============================================================================

/// IDL basic/primitive type kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum BasicTypeKind {
    // Signed explicit integers
    /// 8-bit signed integer
    #[cfg_attr(feature = "serde", serde(rename = "int8"))]
    Int8,
    /// 16-bit signed integer
    #[cfg_attr(feature = "serde", serde(rename = "int16"))]
    Int16,
    /// 32-bit signed integer
    #[cfg_attr(feature = "serde", serde(rename = "int32"))]
    Int32,
    /// 64-bit signed integer
    #[cfg_attr(feature = "serde", serde(rename = "int64"))]
    Int64,

    // Unsigned explicit integers
    /// 8-bit unsigned integer
    #[cfg_attr(feature = "serde", serde(rename = "uint8"))]
    UInt8,
    /// 16-bit unsigned integer
    #[cfg_attr(feature = "serde", serde(rename = "uint16"))]
    UInt16,
    /// 32-bit unsigned integer
    #[cfg_attr(feature = "serde", serde(rename = "uint32"))]
    UInt32,
    /// 64-bit unsigned integer
    #[cfg_attr(feature = "serde", serde(rename = "uint64"))]
    UInt64,

    // Signed non-explicit integers
    /// Short integer (non-explicit size)
    #[cfg_attr(feature = "serde", serde(rename = "short"))]
    Short,
    /// Default integer (equivalent to long)
    #[cfg_attr(feature = "serde", serde(rename = "int"))]
    Int,
    /// Long integer (non-explicit size)
    #[cfg_attr(feature = "serde", serde(rename = "long"))]
    Long,
    /// Long long integer (non-explicit size)
    #[cfg_attr(feature = "serde", serde(rename = "long long"))]
    LongLong,

    // Unsigned non-explicit integers
    /// Unsigned short integer (non-explicit size)
    #[cfg_attr(feature = "serde", serde(rename = "unsigned short"))]
    UnsignedShort,
    /// Unsigned long integer (non-explicit size)
    #[cfg_attr(feature = "serde", serde(rename = "unsigned long"))]
    UnsignedLong,
    /// Unsigned long long integer (non-explicit size)
    #[cfg_attr(feature = "serde", serde(rename = "unsigned long long"))]
    UnsignedLongLong,

    // Floating point
    /// 32-bit floating point
    #[cfg_attr(feature = "serde", serde(rename = "float"))]
    Float,
    /// 64-bit floating point
    #[cfg_attr(feature = "serde", serde(rename = "double"))]
    Double,
    /// Long double floating point (non-explicit size)
    #[cfg_attr(feature = "serde", serde(rename = "long double"))]
    LongDouble,

    // Character
    /// Single character
    #[cfg_attr(feature = "serde", serde(rename = "char"))]
    Char,
    /// Wide character
    #[cfg_attr(feature = "serde", serde(rename = "wchar"))]
    WChar,

    // Boolean
    /// Boolean type
    #[cfg_attr(feature = "serde", serde(rename = "boolean"))]
    Boolean,

    // Octet
    /// 8-bit octet (unsigned byte)
    #[cfg_attr(feature = "serde", serde(rename = "octet"))]
    Octet,
}

impl BasicTypeKind {
    /// Parse a basic type kind from a string representation
    ///
    /// Returns `None` if the string doesn't match any known basic type.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "int8" => Some(Self::Int8),
            "int16" => Some(Self::Int16),
            "int32" => Some(Self::Int32),
            "int64" => Some(Self::Int64),
            "uint8" => Some(Self::UInt8),
            "uint16" => Some(Self::UInt16),
            "uint32" => Some(Self::UInt32),
            "uint64" => Some(Self::UInt64),
            "short" => Some(Self::Short),
            "int" => Some(Self::Int),
            "long" => Some(Self::Long),
            "long long" => Some(Self::LongLong),
            "unsigned short" => Some(Self::UnsignedShort),
            "unsigned long" => Some(Self::UnsignedLong),
            "unsigned long long" => Some(Self::UnsignedLongLong),
            "float" => Some(Self::Float),
            "double" => Some(Self::Double),
            "long double" => Some(Self::LongDouble),
            "char" => Some(Self::Char),
            "wchar" => Some(Self::WChar),
            "boolean" => Some(Self::Boolean),
            "octet" => Some(Self::Octet),
            _ => None,
        }
    }

    /// Get the IDL string representation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Int8 => "int8",
            Self::Int16 => "int16",
            Self::Int32 => "int32",
            Self::Int64 => "int64",
            Self::UInt8 => "uint8",
            Self::UInt16 => "uint16",
            Self::UInt32 => "uint32",
            Self::UInt64 => "uint64",
            Self::Short => "short",
            Self::Int => "int",
            Self::Long => "long",
            Self::LongLong => "long long",
            Self::UnsignedShort => "unsigned short",
            Self::UnsignedLong => "unsigned long",
            Self::UnsignedLongLong => "unsigned long long",
            Self::Float => "float",
            Self::Double => "double",
            Self::LongDouble => "long double",
            Self::Char => "char",
            Self::WChar => "wchar",
            Self::Boolean => "boolean",
            Self::Octet => "octet",
        }
    }

    /// Check if this is an integer type
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::UInt8
                | Self::UInt16
                | Self::UInt32
                | Self::UInt64
                | Self::Short
                | Self::Int
                | Self::Long
                | Self::LongLong
                | Self::UnsignedShort
                | Self::UnsignedLong
                | Self::UnsignedLongLong
        )
    }

    /// Check if this is a floating point type
    #[must_use]
    pub const fn is_floating_point(&self) -> bool {
        matches!(self, Self::Float | Self::Double | Self::LongDouble)
    }

    /// Check if this is a signed integer type
    #[must_use]
    pub const fn is_signed(&self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::Short
                | Self::Int
                | Self::Long
                | Self::LongLong
        )
    }

    /// Check if this is an unsigned integer type
    #[must_use]
    pub const fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Self::UInt8
                | Self::UInt16
                | Self::UInt32
                | Self::UInt64
                | Self::UnsignedShort
                | Self::UnsignedLong
                | Self::UnsignedLongLong
        )
    }
}

/// A basic type according to the IDL specification
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasicType {
    /// The kind of basic type
    #[cfg_attr(feature = "serde", serde(rename = "typename"))]
    pub kind: BasicTypeKind,
}

impl BasicType {
    /// Create a new `BasicType` from a kind
    ///
    /// This is the preferred way to create a `BasicType` as it avoids
    /// runtime string parsing overhead.
    #[must_use]
    pub const fn from_kind(kind: BasicTypeKind) -> Self {
        Self { kind }
    }

    /// Create a new `BasicType` from a string
    ///
    /// # Panics
    ///
    /// Panics if the typename is not a recognized IDL basic type.
    ///
    /// # Deprecated
    ///
    /// Use `from_kind` for better performance when the type is known at compile time.
    #[deprecated(
        since = "0.2.0",
        note = "Use `BasicType::from_kind(BasicTypeKind::...)` instead for better performance"
    )]
    pub fn new<S: AsRef<str>>(typename: S) -> Self {
        let kind = BasicTypeKind::parse(typename.as_ref())
            .unwrap_or_else(|| panic!("Unknown basic type: {}", typename.as_ref()));
        Self { kind }
    }

    /// Get the typename as a string reference
    #[must_use]
    pub fn typename(&self) -> &'static str {
        self.kind.as_str()
    }

    /// Check if this is an integer type
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        self.kind.is_integer()
    }

    /// Check if this is a floating point type
    #[must_use]
    pub const fn is_floating_point(&self) -> bool {
        self.kind.is_floating_point()
    }
}

// ============================================================================
// Named Types
// ============================================================================

/// A type identified by name only
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NamedType {
    /// The name of the type
    pub name: String,
}

impl NamedType {
    /// Create a new `NamedType`
    pub fn new<S: AsRef<str>>(name: S) -> Self {
        Self {
            name: name.as_ref().to_string(),
        }
    }

    /// Get a reference to the name (avoids cloning)
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A type identified by a name in a namespaced scope
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NamespacedType {
    /// The nested namespaces identifying a specific scope
    pub namespaces: Vec<String>,
    /// The name of the type within that scope
    pub name: String,
}

impl NamespacedType {
    /// Create a new `NamespacedType`
    pub fn new<S: AsRef<str>>(namespaces: Vec<String>, name: S) -> Self {
        Self {
            namespaces,
            name: name.as_ref().to_string(),
        }
    }

    /// Get a reference to the name (avoids cloning)
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a reference to the namespaces (avoids cloning)
    #[must_use]
    pub fn namespaces(&self) -> &[String] {
        &self.namespaces
    }

    /// Get the full namespaced name as a tuple
    #[must_use]
    pub fn namespaced_name(&self) -> Vec<String> {
        let mut result = self.namespaces.clone();
        result.push(self.name.clone());
        result
    }

    /// Get the full namespaced name as a string with separator
    #[must_use]
    pub fn full_name(&self, separator: &str) -> String {
        if self.namespaces.is_empty() {
            self.name.clone()
        } else {
            format!(
                "{}{separator}{}",
                self.namespaces.join(separator),
                self.name
            )
        }
    }
}

// ============================================================================
// String Types
// ============================================================================

/// Empty struct for unbounded strings - serializes as {}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnboundedString;

#[cfg(feature = "serde")]
impl Serialize for UnboundedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        serializer.serialize_map(Some(0))?.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for UnboundedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Accept an empty map {}
        struct EmptyMapVisitor;
        impl<'de> serde::de::Visitor<'de> for EmptyMapVisitor {
            type Value = UnboundedString;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an empty map {}")
            }
            fn visit_map<A>(self, _map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                Ok(UnboundedString)
            }
        }
        deserializer.deserialize_map(EmptyMapVisitor)
    }
}

/// Bounded string with maximum size
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BoundedString {
    /// Maximum number of characters allowed in the string
    pub maximum_size: u32,
}

/// Abstract base for string types
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum AbstractString {
    /// 8-bit string with limited length
    Bounded(BoundedString),
    /// 8-bit string with unlimited length
    Unbounded(UnboundedString),
}

impl AbstractString {
    /// Check if this string has a maximum size
    #[must_use]
    pub fn has_maximum_size(&self) -> bool {
        matches!(self, AbstractString::Bounded(_))
    }

    /// Get the maximum size if bounded
    #[must_use]
    pub fn maximum_size(&self) -> Option<u32> {
        match self {
            AbstractString::Bounded(b) => Some(b.maximum_size),
            AbstractString::Unbounded(_) => None,
        }
    }
}

/// Empty struct for unbounded wstrings - serializes as {}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnboundedWString;

#[cfg(feature = "serde")]
impl Serialize for UnboundedWString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        serializer.serialize_map(Some(0))?.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for UnboundedWString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Accept an empty map {}
        struct EmptyMapVisitor;
        impl<'de> serde::de::Visitor<'de> for EmptyMapVisitor {
            type Value = UnboundedWString;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an empty map {}")
            }
            fn visit_map<A>(self, _map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                Ok(UnboundedWString)
            }
        }
        deserializer.deserialize_map(EmptyMapVisitor)
    }
}

/// Bounded wstring with maximum size
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BoundedWString {
    /// Maximum number of characters allowed in the wide string
    pub maximum_size: u32,
}

/// Abstract base for wide string types (16-bit)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum AbstractWString {
    /// 16-bit string with limited length
    Bounded(BoundedWString),
    /// 16-bit string with unlimited length
    Unbounded(UnboundedWString),
}

impl AbstractWString {
    /// Check if this string has a maximum size
    #[must_use]
    pub fn has_maximum_size(&self) -> bool {
        matches!(self, AbstractWString::Bounded(_))
    }

    /// Get the maximum size if bounded
    #[must_use]
    pub fn maximum_size(&self) -> Option<u32> {
        match self {
            AbstractWString::Bounded(b) => Some(b.maximum_size),
            AbstractWString::Unbounded(_) => None,
        }
    }
}

// ============================================================================
// Nested Types (Arrays and Sequences)
// ============================================================================

/// An array type with a static size
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Array {
    /// The type of each element in the array
    pub value_type: Box<IdlType>,
    /// The number of elements in the array (must be greater than zero)
    pub size: u32,
}

impl Array {
    /// Create a new Array
    #[must_use]
    pub fn new(value_type: IdlType, size: u32) -> Self {
        Self {
            value_type: Box::new(value_type),
            size,
        }
    }

    /// Arrays always have a maximum size
    #[must_use]
    pub fn has_maximum_size(&self) -> bool {
        true
    }
}

/// A sequence type with bounded length
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BoundedSequence {
    /// The type of each element in the sequence
    pub value_type: Box<IdlType>,
    /// The maximum number of elements in the sequence
    pub maximum_size: u32,
}

impl BoundedSequence {
    /// Create a new `BoundedSequence`
    #[must_use]
    pub fn new(value_type: IdlType, maximum_size: u32) -> Self {
        Self {
            value_type: Box::new(value_type),
            maximum_size,
        }
    }

    /// Bounded sequences always have a maximum size
    #[must_use]
    pub fn has_maximum_size(&self) -> bool {
        true
    }
}

/// A sequence type with unlimited length
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnboundedSequence {
    /// The type of each element in the sequence
    pub value_type: Box<IdlType>,
}

impl UnboundedSequence {
    /// Create a new `UnboundedSequence`
    #[must_use]
    pub fn new(value_type: IdlType) -> Self {
        Self {
            value_type: Box::new(value_type),
        }
    }

    /// Unbounded sequences do not have a maximum size
    #[must_use]
    pub fn has_maximum_size(&self) -> bool {
        false
    }
}

// ============================================================================
// Annotations
// ============================================================================

/// An annotation identified by a name with an arbitrary value
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Annotation {
    /// The name/type of the annotation as defined in the IDL spec
    pub name: String,
    /// The annotation value (can be primitive or complex)
    pub value: IdlValue,
}

impl Annotation {
    /// Create a new Annotation
    pub fn new<S: AsRef<str>>(name: S, value: IdlValue) -> Self {
        Self {
            name: name.as_ref().to_string(),
            value,
        }
    }
}

/// Base trait for types which can have annotations
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Annotatable {
    /// List of annotations attached to this element
    pub annotations: Vec<Annotation>,
}

impl Annotatable {
    /// Create a new Annotatable
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are no annotations
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Serialize just the annotations list
    ///
    /// # Errors
    /// serialization error
    #[cfg(feature = "serde")]
    pub fn serialize_annotations<S>(
        annotatable: &Annotatable,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        annotatable.annotations.serialize(serializer)
    }

    /// Deserialize annotations list into Annotatable
    ///
    /// # Errors
    /// de-serialization error
    #[cfg(feature = "serde")]
    pub fn deserialize_annotations<'de, D>(deserializer: D) -> Result<Annotatable, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let annotations = Vec::<Annotation>::deserialize(deserializer)?;
        Ok(Annotatable { annotations })
    }

    /// Get the unique value of an annotation of a specific type
    #[must_use]
    pub fn get_annotation_value(&self, name: &str) -> Option<&IdlValue> {
        let values: Vec<_> = self
            .annotations
            .iter()
            .filter(|a| a.name == name)
            .map(|a| &a.value)
            .collect();
        if values.len() == 1 {
            Some(values[0])
        } else {
            None
        }
    }

    /// Get all values of annotations of a specific type
    #[must_use]
    pub fn get_annotation_values(&self, name: &str) -> Vec<&IdlValue> {
        self.annotations
            .iter()
            .filter(|a| a.name == name)
            .map(|a| &a.value)
            .collect()
    }

    /// Check if there is exactly one annotation of a specific type
    #[must_use]
    pub fn has_annotation(&self, name: &str) -> bool {
        self.get_annotation_values(name).len() == 1
    }

    /// Check if there are any annotations of a specific type
    #[must_use]
    pub fn has_annotations(&self, name: &str) -> bool {
        !self.get_annotation_values(name).is_empty()
    }

    /// Get comment lines from verbatim annotations
    #[must_use]
    pub fn get_comment_lines(&self) -> Vec<String> {
        self.annotations
            .iter()
            .filter_map(|annotation| {
                if annotation.name == "verbatim"
                    && let IdlValue::Object(ref map) = annotation.value
                    && let (Some(IdlValue::String(lang)), Some(IdlValue::String(text))) =
                        (map.get("language"), map.get("text"))
                    && lang == "comment"
                {
                    return Some(
                        text.lines()
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>(),
                    );
                }
                None
            })
            .flatten()
            .collect()
    }

    /// Get default value from @default annotation
    #[must_use]
    pub fn get_default_value(&self) -> Option<String> {
        if let Some(IdlValue::Object(map)) = self.get_annotation_value("default")
            && let Some(value) = map.get("value")
        {
            // Convert the IdlValue to a string representation
            return Some(match value {
                IdlValue::Bool(b) => b.to_string(),
                IdlValue::Int8(i) => i.to_string(),
                IdlValue::UInt8(u) => u.to_string(),
                IdlValue::Int16(i) => i.to_string(),
                IdlValue::UInt16(u) => u.to_string(),
                IdlValue::Int32(i) => i.to_string(),
                IdlValue::UInt32(u) => u.to_string(),
                IdlValue::Int64(i) => i.to_string(),
                IdlValue::UInt64(u) => u.to_string(),
                IdlValue::Float32(f) => f.to_string(),
                IdlValue::Float64(f) => f.to_string(),
                IdlValue::Char(c) => c.to_string(),
                IdlValue::String(s) => s.clone(),
                _ => return None,
            });
        }
        None
    }
}

// ============================================================================
// Structure Members and Fields
// ============================================================================

/// A member of a structure
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Member {
    /// The type of the member
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub member_type: IdlType,
    /// The name of the member
    pub name: String,
    /// Annotations attached to this member
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Annotatable::is_empty")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "Annotatable::serialize_annotations")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "Annotatable::deserialize_annotations", default)
    )]
    pub annotations: Annotatable,
}

impl Member {
    /// Create a new Member
    pub fn new<S: AsRef<str>>(member_type: IdlType, name: S) -> Self {
        Self {
            member_type,
            name: name.as_ref().to_string(),
            annotations: Annotatable::new(),
        }
    }
}

// ============================================================================
// Structures and Messages
// ============================================================================

/// A structure containing members
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Structure {
    /// The namespaced type identifying the structure
    pub namespaced_type: NamespacedType,
    /// The members of the structure
    pub members: Vec<Member>,
    /// Annotations attached to this structure
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Annotatable::is_empty")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "Annotatable::serialize_annotations")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "Annotatable::deserialize_annotations", default)
    )]
    pub annotations: Annotatable,
}

impl Structure {
    /// Create a new Structure
    #[must_use]
    pub fn new(namespaced_type: NamespacedType) -> Self {
        Self {
            namespaced_type,
            members: Vec::new(),
            annotations: Annotatable::new(),
        }
    }

    /// Check whether any member has a particular annotation
    #[must_use]
    pub fn has_any_member_with_annotation(&self, name: &str) -> bool {
        self.members
            .iter()
            .any(|member| member.annotations.has_annotation(name))
    }
}

/// A message is a structure that can contain constants
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Message {
    /// The structure of the message
    pub structure: Structure,
    /// Constants defined in the message
    pub constants: Vec<Constant>,
}

impl Message {
    /// Create a new Message
    #[must_use]
    pub fn new(structure: Structure) -> Self {
        Self {
            structure,
            constants: Vec::new(),
        }
    }
}

// ============================================================================
// Constants
// ============================================================================

/// A constant definition
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Constant {
    /// The name of the constant
    pub name: String,
    /// The type of the constant
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub constant_type: IdlType,
    /// The value of the constant
    pub value: IdlValue,
    /// Annotations attached to this constant
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Annotatable::is_empty")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "Annotatable::serialize_annotations")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "Annotatable::deserialize_annotations", default)
    )]
    pub annotations: Annotatable,
}

impl Constant {
    /// Create a new Constant
    pub fn new<S: AsRef<str>>(name: S, constant_type: IdlType, value: IdlValue) -> Self {
        Self {
            name: name.as_ref().to_string(),
            constant_type,
            value,
            annotations: Annotatable::new(),
        }
    }
}

// ============================================================================
// Services
// ============================================================================

/// A service containing request and response messages
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Service {
    /// The namespaced type identifying the service
    pub namespaced_type: NamespacedType,
    /// The request message
    pub request_message: Message,
    /// The response message  
    pub response_message: Message,
    /// The event message (automatically generated)
    pub event_message: Message,
}

impl Service {
    /// Create a new Service
    #[must_use]
    pub fn new(namespaced_type: NamespacedType, request: Message, response: Message) -> Self {
        // Create event message structure with info, request, response members
        let mut event_structure = Structure::new(NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!("{}{}", namespaced_type.name, SERVICE_EVENT_MESSAGE_SUFFIX),
        ));

        // Add 'info' member - ServiceEventInfo type
        let info_member = Member::new(
            IdlType::Namespaced(NamespacedType::new(
                vec!["service_msgs".to_string(), "msg".to_string()],
                "ServiceEventInfo",
            )),
            "info",
        );
        event_structure.members.push(info_member);

        // Add 'request' member - bounded sequence of request type
        let request_type = NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!("{}{}", namespaced_type.name, SERVICE_REQUEST_MESSAGE_SUFFIX),
        );
        let request_member = Member::new(
            IdlType::BoundedSequence(BoundedSequence::new(IdlType::Namespaced(request_type), 1)),
            "request",
        );
        event_structure.members.push(request_member);

        // Add 'response' member - bounded sequence of response type
        let response_type = NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!(
                "{}{}",
                namespaced_type.name, SERVICE_RESPONSE_MESSAGE_SUFFIX
            ),
        );
        let response_member = Member::new(
            IdlType::BoundedSequence(BoundedSequence::new(IdlType::Namespaced(response_type), 1)),
            "response",
        );
        event_structure.members.push(response_member);

        let event_message = Message::new(event_structure);

        Self {
            namespaced_type,
            request_message: request,
            response_message: response,
            event_message,
        }
    }
}

// ============================================================================
// Actions
// ============================================================================

/// An action containing goal, result, and feedback messages
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Action {
    /// The namespaced type identifying the action
    pub namespaced_type: NamespacedType,
    /// The goal message
    pub goal: Message,
    /// The result message
    pub result: Message,
    /// The feedback message
    pub feedback: Message,
    /// The send goal service (derived)
    pub send_goal_service: Service,
    /// The get result service (derived)
    pub get_result_service: Service,
    /// The feedback message (derived)
    pub feedback_message: Message,
    /// Implicit includes required for the action
    pub implicit_includes: Vec<Include>,
}

impl Action {
    /// Create a new Action with derived types
    #[must_use]
    pub fn new(
        namespaced_type: NamespacedType,
        goal: Message,
        result: Message,
        feedback: Message,
    ) -> Self {
        // Helper types
        let goal_id_type = IdlType::Namespaced(NamespacedType::new(
            vec!["unique_identifier_msgs".to_string(), "msg".to_string()],
            "UUID",
        ));
        let time_type = IdlType::Namespaced(NamespacedType::new(
            vec!["builtin_interfaces".to_string(), "msg".to_string()],
            "Time",
        ));

        // SendGoal service
        let goal_service_name = format!("{}{}", namespaced_type.name, ACTION_GOAL_SERVICE_SUFFIX);

        let mut send_goal_request_struct = Structure::new(NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!("{goal_service_name}{SERVICE_REQUEST_MESSAGE_SUFFIX}"),
        ));
        send_goal_request_struct
            .members
            .push(Member::new(goal_id_type.clone(), "goal_id"));
        send_goal_request_struct.members.push(Member::new(
            IdlType::Namespaced(goal.structure.namespaced_type.clone()),
            "goal",
        ));
        let send_goal_request = Message::new(send_goal_request_struct);

        let mut send_goal_response_struct = Structure::new(NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!("{goal_service_name}{SERVICE_RESPONSE_MESSAGE_SUFFIX}"),
        ));
        send_goal_response_struct.members.push(Member::new(
            IdlType::Basic(BasicType::from_kind(BasicTypeKind::Boolean)),
            "accepted",
        ));
        send_goal_response_struct
            .members
            .push(Member::new(time_type.clone(), "stamp"));
        let send_goal_response = Message::new(send_goal_response_struct);

        let send_goal_service = Service::new(
            NamespacedType::new(namespaced_type.namespaces.clone(), goal_service_name),
            send_goal_request,
            send_goal_response,
        );

        // GetResult service
        let result_service_name =
            format!("{}{}", namespaced_type.name, ACTION_RESULT_SERVICE_SUFFIX);

        let mut get_result_request_struct = Structure::new(NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!("{result_service_name}{SERVICE_REQUEST_MESSAGE_SUFFIX}"),
        ));
        get_result_request_struct
            .members
            .push(Member::new(goal_id_type.clone(), "goal_id"));
        let get_result_request = Message::new(get_result_request_struct);

        let mut get_result_response_struct = Structure::new(NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!("{result_service_name}{SERVICE_RESPONSE_MESSAGE_SUFFIX}"),
        ));
        get_result_response_struct.members.push(Member::new(
            IdlType::Basic(BasicType::from_kind(BasicTypeKind::Int8)),
            "status",
        ));
        get_result_response_struct.members.push(Member::new(
            IdlType::Namespaced(result.structure.namespaced_type.clone()),
            "result",
        ));
        let get_result_response = Message::new(get_result_response_struct);

        let get_result_service = Service::new(
            NamespacedType::new(namespaced_type.namespaces.clone(), result_service_name),
            get_result_request,
            get_result_response,
        );

        // Feedback message
        let mut feedback_message_struct = Structure::new(NamespacedType::new(
            namespaced_type.namespaces.clone(),
            format!("{}{}", namespaced_type.name, ACTION_FEEDBACK_MESSAGE_SUFFIX),
        ));
        feedback_message_struct
            .members
            .push(Member::new(goal_id_type, "goal_id"));
        feedback_message_struct.members.push(Member::new(
            IdlType::Namespaced(feedback.structure.namespaced_type.clone()),
            "feedback",
        ));
        let feedback_message = Message::new(feedback_message_struct);

        let implicit_includes = vec![
            Include::new("builtin_interfaces/msg/Time.idl"),
            Include::new("unique_identifier_msgs/msg/UUID.idl"),
        ];

        Self {
            namespaced_type,
            goal,
            result,
            feedback,
            send_goal_service,
            get_result_service,
            feedback_message,
            implicit_includes,
        }
    }
}

// ============================================================================
// Includes and File Structure
// ============================================================================

/// An include statement
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Include {
    /// A URI identifying the included file
    pub locator: String,
}

impl Include {
    /// Create a new Include
    pub fn new<S: AsRef<str>>(locator: S) -> Self {
        Self {
            locator: locator.as_ref().to_string(),
        }
    }
}

/// IDL file locator
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IdlLocator {
    /// The base path of the file
    pub basepath: PathBuf,
    /// The relative path to the file
    pub relative_path: PathBuf,
}

impl IdlLocator {
    /// Create a new `IdlLocator`
    #[must_use]
    pub fn new(basepath: PathBuf, relative_path: PathBuf) -> Self {
        Self {
            basepath,
            relative_path,
        }
    }

    /// Get the absolute path to the file
    #[must_use]
    pub fn get_absolute_path(&self) -> PathBuf {
        self.basepath.join(&self.relative_path)
    }
}

/// IDL content element (can be Include, Message, Service, or Action)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[allow(clippy::large_enum_variant)]
pub enum IdlContentElement {
    /// Include directive
    Include(Include),
    /// Message definition
    Message(Message),
    /// Service definition
    Service(Service),
    /// Action definition
    Action(Action),
    /// Structure definition
    Structure(Structure),
    /// Constant definition
    Constant(Constant),
}

/// The content of an IDL file
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IdlContent {
    /// List of elements in the IDL file
    pub elements: Vec<IdlContentElement>,
}

impl IdlContent {
    /// Create a new `IdlContent`
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get elements of a specific type
    pub fn get_elements_of_type<T>(
        &self,
        element_type: fn(&IdlContentElement) -> Option<&T>,
    ) -> Vec<&T> {
        self.elements.iter().filter_map(element_type).collect()
    }

    /// Get all includes
    #[must_use]
    pub fn get_includes(&self) -> Vec<&Include> {
        self.get_elements_of_type(|e| match e {
            IdlContentElement::Include(inc) => Some(inc),
            _ => None,
        })
    }

    /// Get all messages
    #[must_use]
    pub fn get_messages(&self) -> Vec<&Message> {
        self.get_elements_of_type(|e| match e {
            IdlContentElement::Message(msg) => Some(msg),
            _ => None,
        })
    }

    /// Get all services
    #[must_use]
    pub fn get_services(&self) -> Vec<&Service> {
        self.get_elements_of_type(|e| match e {
            IdlContentElement::Service(srv) => Some(srv),
            _ => None,
        })
    }

    /// Get all actions
    #[must_use]
    pub fn get_actions(&self) -> Vec<&Action> {
        self.get_elements_of_type(|e| match e {
            IdlContentElement::Action(act) => Some(act),
            _ => None,
        })
    }
}

/// Descriptor for a parsed IDL file
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IdlFile {
    /// The locator of the IDL file
    pub locator: IdlLocator,
    /// The content of the IDL file
    pub content: IdlContent,
}

impl IdlFile {
    /// Create a new `IdlFile`
    #[must_use]
    pub fn new(locator: IdlLocator, content: IdlContent) -> Self {
        Self { locator, content }
    }
}

// ============================================================================
// Parser Helper Types
// ============================================================================

/// Helper struct for declarator information used in LALRPOP parser
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DeclaratorInfo {
    /// The name of the declarator
    pub name: String,
    /// Array sizes if this is an array declarator
    pub array_sizes: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_from_kind() {
        let basic = BasicType::from_kind(BasicTypeKind::Long);
        assert_eq!(basic.typename(), "long");
    }

    #[test]
    fn test_basic_type_is_integer() {
        let int_type = BasicType::from_kind(BasicTypeKind::Int32);
        assert!(int_type.is_integer());

        let long_type = BasicType::from_kind(BasicTypeKind::Long);
        assert!(long_type.is_integer());

        let float_type = BasicType::from_kind(BasicTypeKind::Float);
        assert!(!float_type.is_integer());
    }

    #[test]
    fn test_basic_type_is_floating_point() {
        let float_type = BasicType::from_kind(BasicTypeKind::Float);
        assert!(float_type.is_floating_point());

        let double_type = BasicType::from_kind(BasicTypeKind::Double);
        assert!(double_type.is_floating_point());

        let int_type = BasicType::from_kind(BasicTypeKind::Int32);
        assert!(!int_type.is_floating_point());
    }

    #[test]
    fn test_named_type() {
        let named = NamedType::new("MyType");
        assert_eq!(named.name, "MyType");
    }

    #[test]
    fn test_namespaced_type() {
        let ns = NamespacedType::new(vec!["pkg".to_string(), "msg".to_string()], "Point");
        assert_eq!(ns.namespaces, vec!["pkg", "msg"]);
        assert_eq!(ns.name, "Point");
        assert_eq!(ns.full_name("::"), "pkg::msg::Point");
        assert_eq!(ns.namespaced_name(), vec!["pkg", "msg", "Point"]);
    }

    #[test]
    fn test_namespaced_type_no_namespaces() {
        let ns = NamespacedType::new(vec![], "Point");
        assert_eq!(ns.full_name("::"), "Point");
        assert_eq!(ns.namespaced_name(), vec!["Point"]);
    }

    #[test]
    fn test_abstract_string_bounded() {
        let bounded = AbstractString::Bounded(BoundedString { maximum_size: 100 });
        assert!(bounded.has_maximum_size());
        assert_eq!(bounded.maximum_size(), Some(100));
    }

    #[test]
    fn test_abstract_string_unbounded() {
        let unbounded = AbstractString::Unbounded(UnboundedString);
        assert!(!unbounded.has_maximum_size());
        assert_eq!(unbounded.maximum_size(), None);
    }

    #[test]
    fn test_abstract_wstring_bounded() {
        let bounded = AbstractWString::Bounded(BoundedWString { maximum_size: 50 });
        assert!(bounded.has_maximum_size());
        assert_eq!(bounded.maximum_size(), Some(50));
    }

    #[test]
    fn test_abstract_wstring_unbounded() {
        let unbounded = AbstractWString::Unbounded(UnboundedWString);
        assert!(!unbounded.has_maximum_size());
        assert_eq!(unbounded.maximum_size(), None);
    }

    #[test]
    fn test_array_type() {
        let arr = Array::new(
            IdlType::Basic(BasicType::from_kind(BasicTypeKind::Long)),
            10,
        );
        assert_eq!(arr.size, 10);
        assert!(arr.has_maximum_size());
    }

    #[test]
    fn test_bounded_sequence() {
        let seq = BoundedSequence::new(
            IdlType::Basic(BasicType::from_kind(BasicTypeKind::Long)),
            100,
        );
        assert_eq!(seq.maximum_size, 100);
        assert!(seq.has_maximum_size());
    }

    #[test]
    fn test_unbounded_sequence() {
        let seq = UnboundedSequence::new(IdlType::Basic(BasicType::from_kind(BasicTypeKind::Long)));
        assert!(!seq.has_maximum_size());
    }

    #[test]
    fn test_idl_type_has_maximum_size() {
        let basic = IdlType::Basic(BasicType::from_kind(BasicTypeKind::Long));
        assert!(basic.has_maximum_size());

        let unbounded_str = IdlType::String(AbstractString::Unbounded(UnboundedString));
        assert!(!unbounded_str.has_maximum_size());

        let bounded_str =
            IdlType::String(AbstractString::Bounded(BoundedString { maximum_size: 50 }));
        assert!(bounded_str.has_maximum_size());

        let unbounded_seq = IdlType::UnboundedSequence(UnboundedSequence::new(IdlType::Basic(
            BasicType::from_kind(BasicTypeKind::Long),
        )));
        assert!(!unbounded_seq.has_maximum_size());
    }

    #[test]
    fn test_idl_type_is_primitive() {
        let basic = IdlType::Basic(BasicType::from_kind(BasicTypeKind::Int32));
        assert!(basic.is_primitive());

        let named = IdlType::Named(NamedType::new("CustomType"));
        assert!(!named.is_primitive());
    }

    #[test]
    fn test_idl_locator() {
        let locator = IdlLocator::new(
            std::path::PathBuf::from("/base"),
            std::path::PathBuf::from("file.idl"),
        );
        assert_eq!(locator.basepath, std::path::PathBuf::from("/base"));
        assert_eq!(locator.relative_path, std::path::PathBuf::from("file.idl"));
        assert_eq!(
            locator.get_absolute_path(),
            std::path::PathBuf::from("/base/file.idl")
        );
    }

    #[test]
    fn test_idl_file() {
        let locator = IdlLocator::new(
            std::path::PathBuf::from("."),
            std::path::PathBuf::from("test.idl"),
        );
        let content = IdlContent::new();
        let file = IdlFile::new(locator, content);

        assert_eq!(
            file.locator.relative_path,
            std::path::PathBuf::from("test.idl")
        );
    }

    #[test]
    fn test_type_constants() {
        assert!(SIGNED_NONEXPLICIT_INTEGER_TYPES.contains(&"long"));
        assert!(UNSIGNED_NONEXPLICIT_INTEGER_TYPES.contains(&"unsigned long"));
        assert!(FLOATING_POINT_TYPES.contains(&"double"));
        assert!(CHARACTER_TYPES.contains(&"char"));
        assert_eq!(BOOLEAN_TYPE, "boolean");
        assert_eq!(OCTET_TYPE, "octet");
    }

    #[test]
    fn test_service_message_suffixes() {
        assert_eq!(SERVICE_REQUEST_MESSAGE_SUFFIX, "_Request");
        assert_eq!(SERVICE_RESPONSE_MESSAGE_SUFFIX, "_Response");
        assert_eq!(SERVICE_EVENT_MESSAGE_SUFFIX, "_Event");
    }

    #[test]
    fn test_action_suffixes() {
        assert_eq!(ACTION_GOAL_SUFFIX, "_Goal");
        assert_eq!(ACTION_RESULT_SUFFIX, "_Result");
        assert_eq!(ACTION_FEEDBACK_SUFFIX, "_Feedback");
        assert_eq!(ACTION_GOAL_SERVICE_SUFFIX, "_SendGoal");
        assert_eq!(ACTION_RESULT_SERVICE_SUFFIX, "_GetResult");
        assert_eq!(ACTION_FEEDBACK_MESSAGE_SUFFIX, "_FeedbackMessage");
    }

    #[test]
    fn test_empty_structure_constant() {
        assert_eq!(
            EMPTY_STRUCTURE_REQUIRED_MEMBER_NAME,
            "structure_needs_at_least_one_member"
        );
    }
}
