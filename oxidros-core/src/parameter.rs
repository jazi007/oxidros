//! Parameter types and structures for ROS2 parameter server.

use crate::helper::Contains;
use num_traits::Zero;
use std::fmt::Display;

/// Describes a range of integers for parameter validation.
///
/// # Example
///
/// ```
/// use oxidros_core::{helper::Contains, parameter::IntegerRange};
/// let range = IntegerRange { min: -5, max: 10, step: 3 };
/// assert!(range.contains(-5));
/// assert!(range.contains(-2));
/// assert!(range.contains(10));
/// assert!(!range.contains(9));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntegerRange {
    /// Minimum value (inclusive).
    pub min: i64,

    /// Maximum value (inclusive).
    pub max: i64,

    /// Step size for valid values.
    pub step: usize,
}

impl Contains for IntegerRange {
    type T = i64;

    fn contains(&self, val: i64) -> bool {
        let range = self.min..=self.max;
        if range.contains(&val) {
            let diff = val - self.min;
            (diff % self.step as i64) == 0
        } else {
            false
        }
    }
}

/// Describes a range of floating point numbers for parameter validation.
///
/// # Example
///
/// ```
/// use oxidros_core::{helper::Contains, parameter::FloatingPointRange};
/// let range = FloatingPointRange { min: -5.0, max: 10.0, step: 3.0 };
/// assert!(range.contains(-5.0));
/// assert!(range.contains(-2.0));
/// assert!(range.contains(10.0));
/// assert!(!range.contains(9.0));
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct FloatingPointRange {
    /// Minimum value (inclusive).
    pub min: f64,

    /// Maximum value (inclusive).
    pub max: f64,

    /// Step size for valid values.
    pub step: f64,
}

impl Contains for FloatingPointRange {
    type T = f64;

    fn contains(&self, val: f64) -> bool {
        let range = self.min..=self.max;
        if range.contains(&val) {
            if self.step.is_zero() {
                return true;
            }

            let diff = val - self.min;
            (diff % self.step).is_zero()
        } else {
            false
        }
    }
}

/// Describes a parameter including its constraints and metadata.
#[derive(Debug, Clone)]
pub struct Descriptor {
    /// Human-readable description of the parameter.
    pub description: String,

    /// Additional constraints description.
    pub additional_constraints: String,

    /// Whether the parameter is read-only after initialization.
    pub read_only: bool,

    /// Whether the parameter allows dynamic type changes.
    pub dynamic_typing: bool,

    /// Floating point range constraint (if applicable).
    pub floating_point_range: Option<FloatingPointRange>,

    /// Integer range constraint (if applicable).
    pub integer_range: Option<IntegerRange>,
}

/// Represents a parameter with its descriptor and current value.
#[derive(Debug, Clone)]
pub struct Parameter {
    /// The parameter's descriptor.
    pub descriptor: Descriptor,

    /// The parameter's current value.
    pub value: Value,
}

impl Parameter {
    /// Creates a new parameter with the given value and settings.
    pub fn new(value: Value, read_only: bool, dynamic_typing: bool, description: String) -> Self {
        Self {
            descriptor: Descriptor {
                description,
                additional_constraints: String::new(),
                read_only,
                dynamic_typing,
                floating_point_range: None,
                integer_range: None,
            },
            value,
        }
    }

    /// Checks if the given value satisfies the parameter's range constraints.
    pub fn check_range(&self, value: &Value) -> bool {
        match (value, &self.descriptor.integer_range) {
            (Value::I64(x), Some(range)) => return range.contains(*x),
            (Value::VecI64(arr), Some(range)) => return arr.iter().all(|x| range.contains(*x)),
            _ => (),
        }

        match (value, &self.descriptor.floating_point_range) {
            (Value::F64(x), Some(range)) => range.contains(*x),
            (Value::VecF64(arr), Some(range)) => arr.iter().all(|x| range.contains(*x)),
            _ => true,
        }
    }
}

/// Represents a parameter value of various types.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    /// Parameter value not set.
    NotSet,

    /// Boolean value.
    Bool(bool),

    /// 64-bit signed integer value.
    I64(i64),

    /// 64-bit floating point value.
    F64(f64),

    /// String value.
    String(String),

    /// Array of boolean values.
    VecBool(Vec<bool>),

    /// Array of 64-bit signed integer values.
    VecI64(Vec<i64>),

    /// Array of 8-bit unsigned integer values (byte array).
    VecU8(Vec<u8>),

    /// Array of 64-bit floating point values.
    VecF64(Vec<f64>),

    /// Array of string values.
    VecString(Vec<String>),
}

impl Value {
    /// Checks if this value has the same type as another value.
    pub fn type_check(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Value::Bool(_), Value::Bool(_))
                | (Value::I64(_), Value::I64(_))
                | (Value::F64(_), Value::F64(_))
                | (Value::String(_), Value::String(_))
                | (Value::VecBool(_), Value::VecBool(_))
                | (Value::VecI64(_), Value::VecI64(_))
                | (Value::VecU8(_), Value::VecU8(_))
                | (Value::VecF64(_), Value::VecF64(_))
                | (Value::VecString(_), Value::VecString(_))
        )
    }

    /// Returns the type name of this value as a string.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::NotSet => "NotSet",
            Value::Bool(_) => "Bool",
            Value::I64(_) => "I64",
            Value::F64(_) => "F64",
            Value::String(_) => "String",
            Value::VecBool(_) => "VecBool",
            Value::VecI64(_) => "VecI64",
            Value::VecU8(_) => "VecU8",
            Value::VecF64(_) => "VecF64",
            Value::VecString(_) => "VecString",
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::NotSet => write!(f, "NotSet"),
            Value::Bool(v) => write!(f, "{}", v),
            Value::I64(v) => write!(f, "{}", v),
            Value::F64(v) => write!(f, "{}", v),
            Value::String(v) => write!(f, "{}", v),
            Value::VecBool(v) => write!(f, "{:?}", v),
            Value::VecI64(v) => write!(f, "{:?}", v),
            Value::VecU8(v) => write!(f, "{:?}", v),
            Value::VecF64(v) => write!(f, "{:?}", v),
            Value::VecString(v) => write!(f, "{:?}", v),
        }
    }
}
