//! Core traits and types for oxidros ROS2 client library.
//!
//! This crate provides the foundational abstractions for ROS2 functionality
//! without depending on any specific implementation (like RCL). It allows
//! for multiple implementations (real RCL, mock, alternative DDS, etc.).

pub mod action;
pub mod api;
pub mod delta_list;
pub mod error;
pub mod helper;
pub mod message;
pub mod msg;
pub mod parameter;
pub mod qos;
pub mod selector;
pub mod time;

// Re-export commonly used error types
pub use error::{ActionError, Error, RclError, Result};

// Re-export API traits
pub use api::{
    RosClient, RosContext, RosNode, RosPublisher, RosSelector, RosServer, RosSubscriber,
    ServiceRequest,
};

// Re-export message traits
pub use msg::{
    ActionGoal, ActionMsg, ActionResult, GetUUID, GoalResponse, ResultResponse, ServiceMsg,
    TryClone, TypeSupport,
};
pub use parameter::{Descriptor, FloatingPointRange, IntegerRange, Parameter, Value};
pub use qos::{DurabilityPolicy, HistoryPolicy, LivelinessPolicy, Profile, ReliabilityPolicy};
pub use ros2_types::*;
pub use time::{UnsafeDuration, UnsafeTime};
