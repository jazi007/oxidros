//! Message traits and types for ROS2 messages, services, and actions.
//!
//! This module re-exports the core ROS2 message traits from `ros2-type-hash`
//! and provides additional oxidros-specific implementations.

// Re-export all traits from ros2-type-hash
pub use ros2_types::{
    ActionGoal, ActionMsg, ActionResult, GetUUID, GoalResponse, ResultResponse, ServiceMsg,
    TryClone, TypeSupport,
};
