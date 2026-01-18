//! Derive macros for ROS2 types
//!
//! This crate provides derive macros and helper macros for ROS2 message types:
//!
//! - `TypeDescription`: Generates type descriptions for ROS2 type hash computation
//! - `Ros2Msg`: Generates ROS2 message bindings (FFI with `rcl` feature, pure Rust otherwise)
//! - `ros2_service!`: Generates service wrapper types
//! - `ros2_action!`: Generates action wrapper types
//!
//! # Features
//!
//! - `rcl`: Enable FFI code generation for ROS2 C libraries. When disabled, generates
//!   pure Rust implementations (Clone, Default, PartialEq, Eq).
//!
//! # Container Attributes
//!
//! - `#[ros2(package = "pkg_name")]` - Specify the ROS2 package name
//! - `#[ros2(interface_type = "msg|srv|action")]` - Specify the interface type (default: "msg")
//!
//! # Field Attributes
//!
//! - `#[ros2(ros2_type = "byte")]` - Override field type (for byte, char, wstring)
//! - `#[ros2(capacity = 255)]` - Specify capacity for bounded strings/sequences
//! - `#[ros2(default = "0")]` - Specify default value
//!
//! # Message Example
//!
//! ```ignore
//! use ros2_types_derive::{TypeDescription, Ros2Msg};
//!
//! #[derive(TypeDescription, Ros2Msg)]
//! #[ros2(package = "std_msgs", interface_type = "msg")]
//! #[repr(C)]
//! pub struct Header {
//!     pub stamp: Time,
//!     pub frame_id: String,
//! }
//! ```
//!
//! # Service Example
//!
//! ```ignore
//! use ros2_types_derive::{Ros2Msg, ros2_service};
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "srv")]
//! #[repr(C)]
//! pub struct AddTwoInts_Request {
//!     pub a: i64,
//!     pub b: i64,
//! }
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "srv")]
//! #[repr(C)]
//! pub struct AddTwoInts_Response {
//!     pub sum: i64,
//! }
//!
//! // Generate the service wrapper
//! ros2_service!(example_interfaces, AddTwoInts);
//! ```
//!
//! # Action Example
//!
//! ```ignore
//! use ros2_types_derive::{Ros2Msg, ros2_action};
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "action")]
//! #[repr(C)]
//! pub struct Fibonacci_Goal {
//!     pub order: i32,
//! }
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "action")]
//! #[repr(C)]
//! pub struct Fibonacci_Result {
//!     pub sequence: I32Seq<0>,
//! }
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "action")]
//! #[repr(C)]
//! pub struct Fibonacci_Feedback {
//!     pub partial_sequence: I32Seq<0>,
//! }
//!
//! // Generate the action wrapper with all helper types
//! ros2_action!(example_interfaces, Fibonacci);
//! ```

mod attrs;
mod ros2_msg;
mod service_action_type_description;
mod type_description;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro for the TypeDescription trait
///
/// This macro automatically implements the TypeDescription trait for structs,
/// generating the type description based on the struct's fields.
///
/// # Container Attributes
///
/// - `#[ros2(package = "pkg_name")]` - Specify the ROS2 package name
/// - `#[ros2(interface_type = "msg|srv|action")]` - Specify the interface type (default: "msg")
///
/// # Field Attributes
///
/// - `#[ros2(ros2_type = "byte")]` - Override field type (for byte, char, wstring)
/// - `#[ros2(capacity = 255)]` - Specify capacity for bounded strings/sequences
/// - `#[ros2(default = "0")]` - Specify default value
///
/// # Example
///
/// ```ignore
/// use ros2_types::TypeDescription;
///
/// #[derive(TypeDescription)]
/// #[ros2(package = "geometry_msgs", interface_type = "msg")]
/// struct Point {
///     x: f64,
///     y: f64,
///     z: f64,
/// }
/// ```
#[proc_macro_derive(TypeDescription, attributes(ros2))]
pub fn derive_type_description(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match type_description::derive_type_description_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derive macro for ROS2 message types
///
/// This macro generates the necessary implementations for ROS2 message types:
///
/// - **With `rcl` feature**: Generates FFI bindings to ROS2 C libraries including
///   TypeSupport, TryClone, PartialEq, Drop, and sequence wrapper types.
///
/// - **Without `rcl` feature**: Generates pure Rust implementations including
///   Default, Clone, PartialEq, and Eq.
///
/// # Container Attributes
///
/// - `#[ros2(package = "pkg_name")]` - Specify the ROS2 package name (required for FFI)
/// - `#[ros2(interface_type = "msg|srv|action")]` - Specify the interface type (default: "msg")
///
/// # Field Attributes
///
/// - `#[ros2(default = "value")]` - Specify default value for pure Rust Default impl
///
/// # Example
///
/// ```ignore
/// use ros2_types_derive::Ros2Msg;
///
/// #[derive(Ros2Msg)]
/// #[ros2(package = "std_msgs", interface_type = "msg")]
/// #[repr(C)]
/// pub struct Header {
///     pub stamp: Time,
///     pub frame_id: RosString<0>,
/// }
/// ```
///
/// With `rcl` feature enabled, this generates:
/// - extern "C" function declarations for init, fini, are_equal, copy
/// - TypeSupport implementation
/// - TryClone implementation
/// - HeaderSeq<N> sequence wrapper type
///
/// Without `rcl` feature, this generates:
/// - Default implementation
/// - Clone implementation
/// - PartialEq and Eq implementations
#[proc_macro_derive(Ros2Msg, attributes(ros2))]
pub fn derive_ros2_msg(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match ros2_msg::derive_ros2_msg_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Macro to generate a ROS2 service wrapper type
///
/// This macro generates a service wrapper struct that implements `ServiceMsg` trait,
/// linking together the Request and Response types.
///
/// # Usage
///
/// ```ignore
/// // First, define the request and response types with #[derive(Ros2Msg)]
/// #[derive(Ros2Msg)]
/// #[ros2(package = "example_interfaces", interface_type = "srv")]
/// pub struct AddTwoInts_Request { pub a: i64, pub b: i64 }
///
/// #[derive(Ros2Msg)]
/// #[ros2(package = "example_interfaces", interface_type = "srv")]
/// pub struct AddTwoInts_Response { pub sum: i64 }
///
/// // Then generate the service wrapper
/// ros2_service!(example_interfaces, AddTwoInts);
/// ```
///
/// # Generated Code
///
/// With `rcl` feature enabled, this generates:
/// - `extern "C"` declaration for service type support
/// - Service wrapper struct implementing `ServiceMsg` trait
///
/// Without `rcl` feature, only the wrapper struct is generated (without trait impl)
#[proc_macro]
pub fn ros2_service(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let parts: Vec<&str> = input_str.split(',').map(|s| s.trim()).collect();

    if parts.len() != 2 {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "Expected: ros2_service!(package_name, ServiceName)",
        )
        .to_compile_error()
        .into();
    }

    let package = parts[0];
    let service_name = parts[1];

    ros2_msg::generate_service_wrapper(package, service_name).into()
}

/// Macro to generate a ROS2 action wrapper type
///
/// This macro generates all the necessary types for a ROS2 action:
/// - Action wrapper struct implementing `ActionMsg` trait
/// - `SendGoal` service types (Request/Response) with `ActionGoal` trait
/// - `GetResult` service types (Request/Response) with `ActionResult` trait
/// - `FeedbackMessage` type with `GetUUID` trait
///
/// # Requirements
///
/// Actions require the following dependencies:
/// - `unique_identifier_msgs` package for UUID types
/// - `builtin_interfaces` package for Time types
///
/// # Usage
///
/// ```ignore
/// // First, define the Goal, Result, and Feedback types
/// #[derive(Ros2Msg)]
/// #[ros2(package = "example_interfaces", interface_type = "action")]
/// pub struct Fibonacci_Goal { pub order: i32 }
///
/// #[derive(Ros2Msg)]
/// #[ros2(package = "example_interfaces", interface_type = "action")]
/// pub struct Fibonacci_Result { pub sequence: I32Seq<0> }
///
/// #[derive(Ros2Msg)]
/// #[ros2(package = "example_interfaces", interface_type = "action")]
/// pub struct Fibonacci_Feedback { pub partial_sequence: I32Seq<0> }
///
/// // Then generate the action wrapper with all helper types
/// ros2_action!(example_interfaces, Fibonacci);
///
/// // Or with a custom path prefix for unique_identifier_msgs:
/// ros2_action!(example_interfaces, Fibonacci, crate);
/// ```
///
/// # Generated Types
///
/// For `ros2_action!(pkg, Fibonacci)`, this generates:
/// - `Fibonacci` - Action wrapper implementing `ActionMsg`
/// - `Fibonacci_SendGoal` - SendGoal service implementing `ActionGoal`
/// - `Fibonacci_SendGoal_Request` - Goal request with UUID
/// - `Fibonacci_SendGoal_Response` - Acceptance response with timestamp
/// - `Fibonacci_GetResult` - GetResult service implementing `ActionResult`
/// - `Fibonacci_GetResult_Request` - Result request with UUID
/// - `Fibonacci_GetResult_Response` - Result response with status
/// - `Fibonacci_FeedbackMessage` - Feedback with UUID
#[proc_macro]
pub fn ros2_action(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let parts: Vec<&str> = input_str.split(',').map(|s| s.trim()).collect();

    if parts.len() < 2 || parts.len() > 3 {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "Expected: ros2_action!(package_name, ActionName) or ros2_action!(package_name, ActionName, crate_path)"
        ).to_compile_error().into();
    }

    let package = parts[0];
    let action_name = parts[1];
    let uuid_path_prefix = parts.get(2).copied();

    ros2_msg::generate_action_wrapper(package, action_name, uuid_path_prefix).into()
}

/// Derive macro for ServiceTypeDescription trait
///
/// This macro generates the `ServiceTypeDescription` trait implementation for a marker struct.
/// It computes the service type hash using the Request and Response types' TypeDescription.
///
/// # Requirements
///
/// - The Request and Response types must already have `TypeDescription` implemented
/// - Types must follow naming convention: `{ServiceName}_Request` and `{ServiceName}_Response`
///
/// # Example
///
/// ```ignore
/// use ros2_types_derive::{TypeDescription, ServiceTypeDescription};
///
/// #[derive(TypeDescription)]
/// #[ros2(package = "example_interfaces", interface_type = "srv")]
/// pub struct AddTwoInts_Request { pub a: i64, pub b: i64 }
///
/// #[derive(TypeDescription)]
/// #[ros2(package = "example_interfaces", interface_type = "srv")]
/// pub struct AddTwoInts_Response { pub sum: i64 }
///
/// // Generate ServiceTypeDescription for the service
/// #[derive(ServiceTypeDescription)]
/// #[ros2(package = "example_interfaces")]
/// pub struct AddTwoInts;
/// ```
#[proc_macro_derive(ServiceTypeDescription, attributes(ros2))]
pub fn derive_service_type_description(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match service_action_type_description::derive_service_type_description_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derive macro for ActionTypeDescription trait
///
/// This macro generates the `ActionTypeDescription` trait implementation for a marker struct.
/// It computes the action type hash using the Goal, Result, and Feedback types' TypeDescription.
///
/// # Requirements
///
/// - Goal, Result, and Feedback types must already have `TypeDescription` implemented
/// - Types must follow naming convention: `{ActionName}_Goal`, `{ActionName}_Result`, `{ActionName}_Feedback`
///
/// # Example
///
/// ```ignore
/// use ros2_types_derive::{TypeDescription, ActionTypeDescription};
///
/// #[derive(TypeDescription)]
/// #[ros2(package = "example_interfaces", interface_type = "action")]
/// pub struct Fibonacci_Goal { pub order: i32 }
///
/// #[derive(TypeDescription)]
/// #[ros2(package = "example_interfaces", interface_type = "action")]
/// pub struct Fibonacci_Result { pub sequence: Vec<i32> }
///
/// #[derive(TypeDescription)]
/// #[ros2(package = "example_interfaces", interface_type = "action")]
/// pub struct Fibonacci_Feedback { pub partial_sequence: Vec<i32> }
///
/// // Generate ActionTypeDescription for the action
/// #[derive(ActionTypeDescription)]
/// #[ros2(package = "example_interfaces")]
/// pub struct Fibonacci;
/// ```
#[proc_macro_derive(ActionTypeDescription, attributes(ros2))]
pub fn derive_action_type_description(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match service_action_type_description::derive_action_type_description_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
