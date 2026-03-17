//! Prelude module for convenient imports.
//!
//! This module re-exports the most commonly used types and traits
//! for ROS2 development with oxidros.
//!
//! # Example
//!
//! ```ignore
//! use oxidros::prelude::*;
//!
//! let ctx = Context::new()?;
//! let node = ctx.create_node("my_node", None)?;
//! let publisher = node.create_publisher::<MyMessage>("topic", None)?;
//! publisher.send(&msg)?;
//! ```

// Re-export error types
pub use oxidros_core::error::Error;

// Re-export API traits
pub use oxidros_core::api::{
    RosClient, RosContext, RosNode, RosPublisher, RosSelector, RosServer, RosSubscriber,
    ServiceRequest,
};

// Re-export message traits
pub use oxidros_core::{ActionGoal, ActionMsg, ActionResult, ServiceMsg, TypeSupport};

// Re-export QoS types
pub use oxidros_core::qos::Profile;

// Re-export message utilities
pub use oxidros_core::message::Message;

// Re-export selector callback result
pub use oxidros_core::selector::CallbackResult;

// Re-export parameter types
pub use oxidros_core::{Parameter, Value};

// Backend-specific types from our own modules
pub use crate::clock::Clock;
pub use crate::logger::init_ros_logging;
pub use crate::parameter::ParameterServer;
pub use crate::service::client::Client;
pub use crate::service::server::Server;
pub use crate::topic::publisher::Publisher;
pub use crate::topic::subscriber::Subscriber;

// Context, Node, Selector come from the backend
#[cfg(feature = "rcl")]
pub use oxidros_wrapper::{Context, Node, Selector};

#[cfg(feature = "zenoh")]
pub use oxidros_zenoh::{Context, Node, Selector};
