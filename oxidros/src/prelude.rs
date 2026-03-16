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

// Backend-specific types - only one will be active at a time
// RCL backend: use oxidros-wrapper types (which implement core API traits)
#[cfg(feature = "rcl")]
mod backend {
    pub use oxidros_wrapper::logger::init_ros_logging;
    pub use oxidros_wrapper::{
        Client, Context, Node, ParameterServer, Publisher, Selector, Server, Subscriber,
    };
}

// Zenoh backend - used when `zenoh` is explicit OR `rcl` is not enabled
#[cfg(feature = "zenoh")]
mod backend {
    pub use oxidros_zenoh::Context;
    pub use oxidros_zenoh::Node;
    pub use oxidros_zenoh::Selector;
    pub use oxidros_zenoh::logger::init_ros_logging;
    pub use oxidros_zenoh::parameter::ParameterServer;
    pub use oxidros_zenoh::service::{client::Client, server::Server};
    pub use oxidros_zenoh::topic::{publisher::Publisher, subscriber::Subscriber};
}

#[cfg(any(feature = "zenoh", feature = "rcl"))]
pub use backend::*;
