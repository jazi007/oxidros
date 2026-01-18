//! ROS2 Message/Service/Action Parser Module
//!
//! This module provides functionality for parsing ROS2 `.msg`, `.srv`, and `.action` files.
//! It handles the traditional ROS2 message format with simple field definitions.

// Re-export the core modules for the msg parser
/// Action parsing functionality
pub mod action;
/// Error types and handling
pub mod errors;
/// Message parsing functionality
pub mod message;
/// Service parsing functionality
pub mod service;
/// Core data structures
pub mod types;
/// Validation utilities
pub mod validation;

// Re-export commonly used types and functions
pub use action::{
    ActionSpecification, create_feedback_message, parse_action_file, parse_action_string,
};
pub use errors::{ParseError, ParseResult};
pub use message::{MessageSpecification, parse_message_file, parse_message_string};
pub use service::{
    ServiceSpecification, create_service_event_message, parse_service_file, parse_service_string,
};
pub use types::{AnnotationValue, Annotations, BaseType, Constant, Field, Type, Value};
pub use validation::{
    PRIMITIVE_TYPES, PrimitiveValue, is_valid_constant_name, is_valid_field_name,
    is_valid_message_name, is_valid_package_name, parse_primitive_value_string,
};

/// Interface specification that can be either a Message, Service, or Action
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum InterfaceSpecification {
    /// A message specification
    Message(MessageSpecification),
    /// A service specification  
    Service(ServiceSpecification),
    /// An action specification
    Action(ActionSpecification),
}

impl InterfaceSpecification {
    /// Get the package name
    #[must_use]
    pub fn package_name(&self) -> &str {
        match self {
            InterfaceSpecification::Message(spec) => &spec.pkg_name,
            InterfaceSpecification::Service(spec) => &spec.pkg_name,
            InterfaceSpecification::Action(spec) => &spec.pkg_name,
        }
    }

    /// Get the interface name
    #[must_use]
    pub fn interface_name(&self) -> &str {
        match self {
            InterfaceSpecification::Message(spec) => &spec.msg_name,
            InterfaceSpecification::Service(spec) => &spec.srv_name,
            InterfaceSpecification::Action(spec) => &spec.action_name,
        }
    }

    /// Get the full interface name (package/interface)
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.package_name(), self.interface_name())
    }

    /// Check if this is a message specification
    #[must_use]
    pub fn is_message(&self) -> bool {
        matches!(self, InterfaceSpecification::Message(_))
    }

    /// Check if this is a service specification
    #[must_use]
    pub fn is_service(&self) -> bool {
        matches!(self, InterfaceSpecification::Service(_))
    }

    /// Check if this is an action specification
    #[must_use]
    pub fn is_action(&self) -> bool {
        matches!(self, InterfaceSpecification::Action(_))
    }

    /// Get as message specification if it is one
    #[must_use]
    pub fn as_message(&self) -> Option<&MessageSpecification> {
        match self {
            InterfaceSpecification::Message(spec) => Some(spec),
            _ => None,
        }
    }

    /// Get as service specification if it is one
    #[must_use]
    pub fn as_service(&self) -> Option<&ServiceSpecification> {
        match self {
            InterfaceSpecification::Service(spec) => Some(spec),
            _ => None,
        }
    }

    /// Get as action specification if it is one
    #[must_use]
    pub fn as_action(&self) -> Option<&ActionSpecification> {
        match self {
            InterfaceSpecification::Action(spec) => Some(spec),
            _ => None,
        }
    }
}

impl std::fmt::Display for InterfaceSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceSpecification::Message(msg) => write!(f, "{msg}"),
            InterfaceSpecification::Service(srv) => write!(f, "{srv}"),
            InterfaceSpecification::Action(action) => write!(f, "{action}"),
        }
    }
}

/// Parse any ROS2 interface file based on its extension
///
/// Automatically detects the file type based on the extension:
/// - `.msg` files are parsed as messages
/// - `.srv` files are parsed as services  
/// - `.action` files are parsed as actions
///
/// # Arguments
///
/// * `pkg_name` - The package name containing the interface
/// * `file_path` - Path to the interface file
///
/// # Returns
///
/// Returns an `InterfaceSpecification` enum containing the parsed specification
///
/// # Errors
///
/// Returns an error if:
/// - The file extension is not recognized (must be .msg, .srv, or .action)
/// - The file cannot be read
/// - The content cannot be parsed for the detected file type
pub fn parse_interface_file(
    pkg_name: &str,
    file_path: &std::path::Path,
) -> ParseResult<InterfaceSpecification> {
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| ParseError::InvalidType {
            type_string: "file extension".to_string(),
            reason: "File must have an extension".to_string(),
        })?;

    match extension {
        "msg" => {
            let msg_spec = parse_message_file(pkg_name, file_path)?;
            Ok(InterfaceSpecification::Message(msg_spec))
        }
        "srv" => {
            let srv_spec = parse_service_file(pkg_name, file_path)?;
            Ok(InterfaceSpecification::Service(srv_spec))
        }
        "action" => {
            let action_spec = parse_action_file(pkg_name, file_path)?;
            Ok(InterfaceSpecification::Action(action_spec))
        }
        _ => Err(ParseError::InvalidType {
            type_string: extension.to_string(),
            reason: "Expected .msg, .srv, or .action extension".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_specification_message_methods() {
        let msg = parse_message_string("test_pkg", "TestMsg", "int32 x\n").unwrap();
        let iface = InterfaceSpecification::Message(msg);

        assert_eq!(iface.package_name(), "test_pkg");
        assert_eq!(iface.interface_name(), "TestMsg");
        assert_eq!(iface.full_name(), "test_pkg/TestMsg");
        assert!(iface.is_message());
        assert!(!iface.is_service());
        assert!(!iface.is_action());
        assert!(iface.as_message().is_some());
        assert!(iface.as_service().is_none());
        assert!(iface.as_action().is_none());
    }

    #[test]
    fn test_interface_specification_service_methods() {
        let srv = parse_service_string("test_pkg", "TestSrv", "int32 a\n---\nint32 b\n").unwrap();
        let iface = InterfaceSpecification::Service(srv);

        assert_eq!(iface.package_name(), "test_pkg");
        assert_eq!(iface.interface_name(), "TestSrv");
        assert_eq!(iface.full_name(), "test_pkg/TestSrv");
        assert!(!iface.is_message());
        assert!(iface.is_service());
        assert!(!iface.is_action());
        assert!(iface.as_message().is_none());
        assert!(iface.as_service().is_some());
        assert!(iface.as_action().is_none());
    }

    #[test]
    fn test_interface_specification_action_methods() {
        let action = parse_action_string(
            "test_pkg",
            "TestAction",
            "int32 x\n---\nint32 y\n---\nint32 z\n",
        )
        .unwrap();
        let iface = InterfaceSpecification::Action(action);

        assert_eq!(iface.package_name(), "test_pkg");
        assert_eq!(iface.interface_name(), "TestAction");
        assert_eq!(iface.full_name(), "test_pkg/TestAction");
        assert!(!iface.is_message());
        assert!(!iface.is_service());
        assert!(iface.is_action());
        assert!(iface.as_message().is_none());
        assert!(iface.as_service().is_none());
        assert!(iface.as_action().is_some());
    }

    #[test]
    fn test_interface_specification_display() {
        let msg = parse_message_string("pkg", "Msg", "int32 x\n").unwrap();
        let iface = InterfaceSpecification::Message(msg);
        let display = format!("{iface}");
        assert!(display.contains("Msg"));
    }

    #[test]
    fn test_parse_interface_file_msg() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("Test.msg");
        std::fs::write(&path, "int32 value\n").unwrap();

        let result = parse_interface_file("test_pkg", &path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_message());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_interface_file_srv() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("Test.srv");
        std::fs::write(&path, "int32 a\n---\nint32 b\n").unwrap();

        let result = parse_interface_file("test_pkg", &path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_service());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_interface_file_action() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("Test.action");
        std::fs::write(&path, "int32 x\n---\nint32 y\n---\nint32 z\n").unwrap();

        let result = parse_interface_file("test_pkg", &path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_action());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_interface_file_invalid_extension() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test.txt");
        std::fs::write(&path, "int32 x\n").unwrap();

        let result = parse_interface_file("test_pkg", &path);
        assert!(result.is_err());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_interface_file_no_extension() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test");
        std::fs::write(&path, "int32 x\n").unwrap();

        let result = parse_interface_file("test_pkg", &path);
        assert!(result.is_err());

        std::fs::remove_file(&path).ok();
    }
}
