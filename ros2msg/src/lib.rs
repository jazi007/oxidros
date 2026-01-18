#![deny(
    unsafe_code,
    unused_must_use,
    unreachable_pub,
    rust_2018_idioms,
    missing_docs,
    clippy::pedantic
)]

//! # ROS2 Message Parser
//!
//! A comprehensive Rust library for parsing ROS2 message, service, and action files.
//! This crate provides functionality to parse `.msg`, `.srv`, and `.action` files
//! according to the ROS2 IDL specification.
//!
//! ## Features
//!
//! - **Message parsing**: Parse `.msg` files with support for primitive types, arrays, and constants
//! - **Service parsing**: Parse `.srv` files with request/response separation
//! - **Action parsing**: Parse `.action` files with goal/result/feedback sections
//! - **Comprehensive validation**: Validates names, types, and values according to ROS2 standards
//! - **Error handling**: Detailed error messages for debugging parsing issues
//! - **Serde support**: Optional serialization support with the `serde` feature
//!
//! ## Quick Start
//!
//! ```rust
//! use ros2msg::{parse_message_string, parse_service_string, parse_action_string};
//!
//! // Parse a message
//! let msg_content = r#"
//! int32 x
//! int32 y
//! string name
//! "#;
//! let msg_spec = parse_message_string("geometry_msgs", "Point", msg_content)?;
//! println!("Parsed message: {}", msg_spec);
//!
//! // Parse a service
//! let srv_content = r#"
//! int32 a
//! int32 b
//! ---
//! int32 sum
//! "#;
//! let srv_spec = parse_service_string("example_msgs", "AddTwoInts", srv_content)?;
//! println!("Parsed service: {}", srv_spec);
//!
//! // Parse an action
//! let action_content = r#"
//! int32 order
//! ---
//! int32[] sequence
//! ---
//! int32[] partial_sequence
//! "#;
//! let action_spec = parse_action_string("example_msgs", "Fibonacci", action_content)?;
//! println!("Parsed action: {}", action_spec);
//! # Ok::<(), ros2msg::ParseError>(())
//! ```
//!
//! ## Modules
//!
//! - [`msg`]: ROS2 message/service/action parser (.msg, .srv, .action files)
//! - [`idl`]: ROS2 IDL parser (full IDL specification support)
//! - [`generator`]: Code generator for converting ROS2 interfaces to Rust types
//! - [`ros2args`]: ROS2 command-line arguments parser

// Public modules
/// ROS2 Message/Service/Action parser
///
/// This module handles the traditional ROS2 message format parsing for
/// `.msg`, `.srv`, and `.action` files.
pub mod msg;

/// ROS2 IDL parser
///
/// This module provides full ROS2 IDL specification support for parsing
/// advanced IDL files with complex types, modules, and annotations.
pub mod idl;

/// Code generator for ROS2 interfaces
///
/// This module provides a bindgen-style API for generating Rust code from
/// ROS2 message, service, action, and IDL files.
pub mod generator;

/// MSG/SRV/Action to IDL converter
///
/// This module converts ROS2 message, service, and action definitions to IDL format,
/// matching the behavior of rosidl_adapter.
pub mod idl_adapter;

// Re-export commonly used types and functions from the msg module
// for backward compatibility
pub use msg::{
    ActionSpecification, AnnotationValue, Annotations, BaseType, Constant, Field,
    InterfaceSpecification, MessageSpecification, PRIMITIVE_TYPES, ParseError, ParseResult,
    PrimitiveValue, ServiceSpecification, Type, Value, create_feedback_message,
    create_service_event_message, is_valid_constant_name, is_valid_field_name,
    is_valid_message_name, is_valid_package_name, parse_action_file, parse_action_string,
    parse_interface_file, parse_message_file, parse_message_string, parse_primitive_value_string,
    parse_service_file, parse_service_string,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
/// # Example
///
/// ```rust,no_run
/// use ros2msg::{parse_interface_file, InterfaceSpecification};
/// use std::path::Path;
///
/// let spec = parse_interface_file("geometry_msgs", Path::new("Point.msg"))?;
/// match spec {
///     InterfaceSpecification::Message(msg_spec) => {
///         println!("Parsed message: {}", msg_spec.msg_name);
///     }
///     InterfaceSpecification::Service(srv_spec) => {
///         println!("Parsed service: {}", srv_spec.srv_name);
///     }
///     InterfaceSpecification::Action(action_spec) => {
///         println!("Parsed action: {}", action_spec.action_name);
///     }
/// }
/// # Ok::<(), ros2msg::ParseError>(())
/// ```
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(VERSION.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_interface_specification() {
        let msg_spec = parse_message_string("test_msgs", "TestMsg", "int32 x").unwrap();
        let interface_spec = InterfaceSpecification::Message(msg_spec);

        assert!(matches!(interface_spec, InterfaceSpecification::Message(_)));
        assert!(!matches!(
            interface_spec,
            InterfaceSpecification::Service(_)
        ));
        assert!(!matches!(interface_spec, InterfaceSpecification::Action(_)));

        if let InterfaceSpecification::Message(msg) = &interface_spec {
            assert_eq!(msg.pkg_name, "test_msgs");
            assert_eq!(msg.msg_name, "TestMsg");
        }
    }

    #[test]
    fn test_comprehensive_parsing() {
        // Test message parsing
        let msg_content = r#"
        # A test message
        int32 x  # X coordinate
        int32 y  # Y coordinate
        string name "default"
        "#;
        let msg_spec = parse_message_string("geometry_msgs", "Point", msg_content).unwrap();
        assert_eq!(msg_spec.fields.len(), 3);

        // Test service parsing
        let srv_content = r"
        int32 a
        int32 b
        ---
        int32 sum
        ";
        let srv_spec = parse_service_string("example_msgs", "AddTwoInts", srv_content).unwrap();
        assert_eq!(srv_spec.request.fields.len(), 2);
        assert_eq!(srv_spec.response.fields.len(), 1);

        // Test action parsing
        let action_content = r"
        int32 order
        ---
        int32[] sequence
        ---
        int32[] partial_sequence
        ";
        let action_spec = parse_action_string("example_msgs", "Fibonacci", action_content).unwrap();
        assert_eq!(action_spec.goal.fields.len(), 1);
        assert_eq!(action_spec.result.fields.len(), 1);
        assert_eq!(action_spec.feedback.fields.len(), 1);
    }
}
