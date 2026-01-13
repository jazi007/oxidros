//! Parse callbacks for ROS2 code generation.
//!
//! This module provides the [`RosCallbacks`] implementation that customizes
//! how ROS2 interface types are generated as Rust code.

use ros2msg::generator::{
    FieldInfo, InterfaceKind, ItemInfo, ModuleInfo, ModuleLevel, ParseCallbacks,
};

/// Callbacks for generating ROS2 FFI code using ros2-types-derive.
///
/// This struct implements [`ParseCallbacks`] to customize code generation,
/// adding appropriate attributes, derives, and type mappings for ROS2 interop.
#[derive(Debug, Default)]
pub struct RosCallbacks {
    /// Path prefix for unique_identifier_msgs (for action types).
    uuid_path: Option<String>,
    /// Path prefix for primitive types.
    primitive_path: Option<String>,
}

impl RosCallbacks {
    /// Creates a new [`RosCallbacks`] with the specified path prefixes.
    ///
    /// # Arguments
    ///
    /// * `uuid_path` - Path prefix for unique_identifier_msgs
    /// * `primitive_path` - Path prefix for primitive types
    pub fn new(uuid_path: Option<String>, primitive_path: Option<String>) -> Self {
        Self {
            uuid_path,
            primitive_path,
        }
    }

    /// Returns the primitive path, defaulting to "oxidros_msg" if not set.
    fn primitive_path(&self) -> &str {
        self.primitive_path
            .as_ref()
            .map_or("oxidros_msg", |v| v.as_str())
    }
}

impl ParseCallbacks for RosCallbacks {
    /// Add the ros2 attribute with package and interface type.
    fn add_attributes(&self, info: &ItemInfo) -> Vec<String> {
        let package = info.package();
        let interface_type = match info.interface_kind() {
            InterfaceKind::Message => "msg",
            InterfaceKind::Service => "srv",
            InterfaceKind::Action => "action",
        };
        // For action types, add uuid_path so the derive macro knows how to find unique_identifier_msgs
        let mut attributes = if matches!(info.interface_kind(), InterfaceKind::Action)
            && let Some(uuid_path) = self.uuid_path.as_ref()
        {
            vec![format!(
                "#[ros2(package = \"{}\", interface_type = \"{}\", uuid_path = \"{}\")]",
                package, interface_type, uuid_path
            )]
        } else {
            vec![format!(
                "#[ros2(package = \"{}\", interface_type = \"{}\")]",
                package, interface_type
            )]
        };
        attributes.push("#[cfg_attr(not(feature = \"rcl\"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]".to_string());
        attributes.push(
            "#[cfg_attr(not(feature = \"rcl\"), serde(crate = \"ros2_types::serde\"))]".to_string(),
        );
        attributes
    }

    /// Adds BigArray attribute for fields with large arrays (> 32 elements).
    ///
    /// serde only supports arrays up to 32 elements by default. For larger arrays,
    /// we use `serde_big_array::BigArray` which is re-exported as `ros2_types::BigArray`.
    fn add_field_attributes(&self, field_info: &FieldInfo) -> Vec<String> {
        let mut attrs = Vec::new();

        // Build #[ros2(...)] attributes for type hash metadata
        let mut ros2_parts = Vec::new();

        // Add type override if present
        if let Some(type_override) = field_info.ros2_type_override() {
            ros2_parts.push(format!("ros2_type = \"{}\"", type_override));
        }

        // If the field is a sequence (not a fixed-size array), mark it explicitly
        // so derives know. Fixed-size arrays have `array_size()` set and are
        // represented as arrays in ROS2, not sequences.
        //
        // Detection methods:
        // 1. ROS type name starts with "sequence" (explicit IDL sequence)
        // 2. Has capacity but no array_size (bounded sequence)
        // 3. Rust field type is Vec<...> without array_size (unbounded sequence)
        // 4. Rust field type contains "Seq<" (custom sequence types like BoolSeq<0>, GoalStatusSeq<0>)
        let ros_type_name = field_info.ros_type_name();
        let rust_type = field_info.field_type();

        // Check if this is a string type (bounded strings have capacity but are NOT sequences)
        let is_string_type = rust_type.contains("RosString") || rust_type.contains("RosWString");

        let is_sequence = ros_type_name.starts_with("sequence")
            // capacity + no array_size = sequence, BUT NOT for bounded strings (they use capacity for string length)
            || (field_info.capacity().is_some() && field_info.array_size().is_none() && !is_string_type)
            || (rust_type.starts_with("Vec<") && field_info.array_size().is_none())
            || rust_type.contains("Seq<");

        if is_sequence {
            ros2_parts.push("sequence".to_string());
        }

        // Add string/wstring attribute for string types
        let is_string = is_string_type && !rust_type.contains("RosWString");
        let is_wstring = rust_type.contains("RosWString");

        if is_string {
            ros2_parts.push("string".to_string());
        }
        if is_wstring {
            ros2_parts.push("wstring".to_string());
        }

        // Add capacity if present
        if let Some(capacity) = field_info.capacity() {
            ros2_parts.push(format!("capacity = {}", capacity));
        }

        // Add default value if present
        if let Some(default_value) = field_info.default_value() {
            ros2_parts.push(format!("default = \"{}\"", default_value));
        }

        if !ros2_parts.is_empty() {
            attrs.push(format!("#[ros2({})]", ros2_parts.join(", ")));
        }

        // Add serde_big_array attribute for large fixed-size arrays (> 32 elements)
        if let Some(size) = field_info.array_size()
            && size > 32
        {
            attrs.push(
                "#[cfg_attr(not(feature = \"rcl\"), serde(with = \"ros2_types::BigArray\"))]"
                    .to_string(),
            );
        }
        attrs
    }

    /// Add derives for ROS2 types including Ros2Msg from ros2-types-derive.
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        vec![
            "ros2_types::Ros2Msg".to_string(),
            "ros2_types::TypeDescription".to_string(),
        ]
    }

    /// Custom type mapping for ROS2 FFI types - strings.
    fn string_type(&self, max_size: Option<u32>) -> Option<String> {
        Some(format!(
            "{}::msg::RosString<{}>",
            self.primitive_path(),
            max_size.unwrap_or(0)
        ))
    }

    /// Custom type mapping for ROS2 FFI types - wide strings.
    fn wstring_type(&self, max_size: Option<u32>) -> Option<String> {
        Some(format!(
            "{}::msg::RosWString<{}>",
            self.primitive_path(),
            max_size.unwrap_or(0)
        ))
    }

    /// Custom type mapping for sequences.
    fn sequence_type(&self, element_type: &str, max_size: Option<u32>) -> Option<String> {
        let size = max_size.unwrap_or(0);
        let path = self.primitive_path();
        match element_type {
            "bool" => Some(format!("{path}::msg::BoolSeq<{size}>")),
            "u8" => Some(format!("{path}::msg::U8Seq<{size}>")),
            "i8" => Some(format!("{path}::msg::I8Seq<{size}>")),
            "u16" => Some(format!("{path}::msg::U16Seq<{size}>")),
            "i16" => Some(format!("{path}::msg::I16Seq<{size}>")),
            "u32" => Some(format!("{path}::msg::U32Seq<{size}>")),
            "i32" => Some(format!("{path}::msg::I32Seq<{size}>")),
            "u64" => Some(format!("{path}::msg::U64Seq<{size}>")),
            "i64" => Some(format!("{path}::msg::I64Seq<{size}>")),
            "f32" => Some(format!("{path}::msg::F32Seq<{size}>")),
            "f64" => Some(format!("{path}::msg::F64Seq<{size}>")),
            s => {
                // Check for RosString sequences
                let ros_string_prefix = format!("{}::msg::RosString<", path);
                if let Some(rest) = s.strip_prefix(&ros_string_prefix) {
                    let str_len = rest.strip_suffix(">").unwrap_or("0");
                    return Some(format!("{path}::msg::RosStringSeq<{str_len}, {size}>"));
                }
                // Check for RosWString sequences
                let ros_wstring_prefix = format!("{}::msg::RosWString<", path);
                if let Some(rest) = s.strip_prefix(&ros_wstring_prefix) {
                    let str_len = rest.strip_suffix(">").unwrap_or("0");
                    return Some(format!("{path}::msg::RosWStringSeq<{str_len}, {size}>"));
                }
                // For custom message types, use the generated XxxSeq<N> type
                // The Ros2Msg derive macro generates these Seq types automatically
                Some(format!("{element_type}Seq<{size}>"))
            }
        }
    }

    /// Add re-exports after type modules.
    fn post_module(&self, info: &ModuleInfo) -> Option<String> {
        match info.module_level() {
            ModuleLevel::Type(_) => Some(format!("pub use {}::*;\n", info.module_name())),
            _ => None,
        }
    }
}
