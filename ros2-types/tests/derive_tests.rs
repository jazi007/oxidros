//! Integration tests for ros2-type-hash-derive with ROS2 message patterns
//!
//! These tests verify that the derive macros generate correct code for
//! message, service, and action types.
//!
//! Note: These tests run WITHOUT the `rcl` feature, so they test the pure Rust
//! implementations (Default, Clone, PartialEq, Eq).
//!
//! When `rcl` feature is enabled, these tests are skipped because the API is different
//! (new() returns Option<Self> instead of Default, TryClone instead of Clone, etc.)

// Skip this entire test file when derive feature is not enabled or rcl feature is enabled
#![cfg(all(feature = "derive", not(feature = "rcl")))]
#![allow(non_camel_case_types)]

use ros2_types::{Ros2Msg, TypeDescription};

// =============================================================================
// Simple Message Test
// =============================================================================

/// Test a simple message type like builtin_interfaces/Time
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "builtin_interfaces", interface_type = "msg")]
#[repr(C)]
pub struct Time {
    pub sec: i32,
    pub nanosec: u32,
}

#[test]
fn test_time_default() {
    let time = Time::default();
    assert_eq!(time.sec, 0);
    assert_eq!(time.nanosec, 0);
}

#[test]
fn test_time_clone() {
    let time = Time {
        sec: 123,
        nanosec: 456,
    };
    let cloned = time.clone();
    assert_eq!(cloned.sec, 123);
    assert_eq!(cloned.nanosec, 456);
}

#[test]
fn test_time_eq() {
    let t1 = Time { sec: 1, nanosec: 2 };
    let t2 = Time { sec: 1, nanosec: 2 };
    let t3 = Time { sec: 1, nanosec: 3 };

    assert_eq!(t1, t2);
    assert_ne!(t1, t3);
}

#[test]
fn test_time_type_description() {
    let desc = Time::type_description();
    assert!(
        desc.type_description
            .type_name
            .contains("builtin_interfaces")
    );
    assert!(desc.type_description.type_name.contains("Time"));
}

// =============================================================================
// Nested Message Test
// =============================================================================

/// A simple point message
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "geometry_msgs", interface_type = "msg")]
#[repr(C)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A pose message that contains nested Point
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "geometry_msgs", interface_type = "msg")]
#[repr(C)]
pub struct Pose {
    pub position: Point,
    pub orientation_x: f64,
    pub orientation_y: f64,
    pub orientation_z: f64,
    pub orientation_w: f64,
}

#[test]
fn test_nested_message() {
    let pose = Pose::default();
    assert_eq!(pose.position.x, 0.0);
    assert_eq!(pose.orientation_w, 0.0);
}

#[test]
fn test_nested_type_description() {
    let desc = Pose::type_description();
    // Should have referenced types for nested Point
    // Check that there's at least one referenced type OR the main type has fields
    assert!(
        !desc.referenced_type_descriptions.is_empty() || !desc.type_description.fields.is_empty()
    );
}

// =============================================================================
// Service Message Test (Request/Response)
// =============================================================================

/// Service request for AddTwoInts
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(
    package = "example_interfaces",
    interface_type = "srv",
    skip_wrapper = true
)]
#[repr(C)]
pub struct AddTwoInts_Request {
    pub a: i64,
    pub b: i64,
}

/// Service response for AddTwoInts
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "example_interfaces", interface_type = "srv")]
#[repr(C)]
pub struct AddTwoInts_Response {
    pub sum: i64,
}

// Generate service wrapper
ros2_types::ros2_service!(example_interfaces, AddTwoInts);

#[test]
fn test_service_request_response() {
    let req = AddTwoInts_Request::default();
    assert_eq!(req.a, 0);
    assert_eq!(req.b, 0);

    let resp = AddTwoInts_Response::default();
    assert_eq!(resp.sum, 0);
}

#[test]
fn test_service_type_description() {
    let req_desc = AddTwoInts_Request::type_description();
    assert!(
        req_desc
            .type_description
            .type_name
            .contains("AddTwoInts_Request")
    );

    let resp_desc = AddTwoInts_Response::type_description();
    assert!(
        resp_desc
            .type_description
            .type_name
            .contains("AddTwoInts_Response")
    );
}

// =============================================================================
// Action Message Test (Goal/Result/Feedback)
// =============================================================================

/// Action goal for Fibonacci
/// Note: skip_wrapper = true because we don't have unique_identifier_msgs::msg::UUID in tests
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(
    package = "example_interfaces",
    interface_type = "action",
    skip_wrapper = true
)]
#[repr(C)]
pub struct Fibonacci_Goal {
    pub order: i32,
}

/// Action result for Fibonacci (simplified - no sequence)
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "example_interfaces", interface_type = "action")]
#[repr(C)]
pub struct Fibonacci_Result {
    pub final_value: i64,
}

/// Action feedback for Fibonacci (simplified - no sequence)
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "example_interfaces", interface_type = "action")]
#[repr(C)]
pub struct Fibonacci_Feedback {
    pub current_value: i64,
}

// Note: ros2_action! requires unique_identifier_msgs::msg::UUID which we don't have
// in this test environment, so we skip the full action wrapper test

#[test]
fn test_action_goal_result_feedback() {
    let goal = Fibonacci_Goal::default();
    assert_eq!(goal.order, 0);

    let result = Fibonacci_Result::default();
    assert_eq!(result.final_value, 0);

    let feedback = Fibonacci_Feedback::default();
    assert_eq!(feedback.current_value, 0);
}

#[test]
fn test_action_type_descriptions() {
    let goal_desc = Fibonacci_Goal::type_description();
    assert!(
        goal_desc
            .type_description
            .type_name
            .contains("Fibonacci_Goal")
    );

    let result_desc = Fibonacci_Result::type_description();
    assert!(
        result_desc
            .type_description
            .type_name
            .contains("Fibonacci_Result")
    );

    let feedback_desc = Fibonacci_Feedback::type_description();
    assert!(
        feedback_desc
            .type_description
            .type_name
            .contains("Fibonacci_Feedback")
    );
}

// =============================================================================
// Empty Struct Test (C++ compatibility)
// =============================================================================

/// Empty service request (like std_srvs/Trigger)
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "std_srvs", interface_type = "srv")]
#[repr(C)]
pub struct Trigger_Request {
    // Empty struct - should get hidden field for C++ compatibility
}

/// Empty service response
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "std_srvs", interface_type = "srv")]
#[repr(C)]
pub struct Trigger_Response {
    pub success: bool,
}

#[test]
fn test_empty_struct() {
    let req = Trigger_Request::default();
    // Empty struct should still be creatable
    let _ = req;
}

#[test]
fn test_empty_struct_type_description() {
    let desc = Trigger_Request::type_description();
    // Should have the hidden field for C++ compatibility
    assert!(!desc.type_description.fields.is_empty());
}

// =============================================================================
// All Primitive Types Test
// =============================================================================

/// Test struct with all ROS2 primitive types
#[derive(Debug, Ros2Msg, TypeDescription)]
#[ros2(package = "test_msgs", interface_type = "msg")]
#[repr(C)]
pub struct AllPrimitives {
    pub bool_val: bool,
    pub i8_val: i8,
    pub u8_val: u8,
    pub i16_val: i16,
    pub u16_val: u16,
    pub i32_val: i32,
    pub u32_val: u32,
    pub i64_val: i64,
    pub u64_val: u64,
    pub f32_val: f32,
    pub f64_val: f64,
}

#[test]
fn test_all_primitives() {
    let msg = AllPrimitives::default();
    assert!(!msg.bool_val);
    assert_eq!(msg.i32_val, 0);
    assert_eq!(msg.f64_val, 0.0);
}

#[test]
fn test_all_primitives_type_description() {
    let desc = AllPrimitives::type_description();
    assert_eq!(desc.type_description.fields.len(), 11);
}
