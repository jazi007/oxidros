//! Native ROS2 implementation using Zenoh middleware.
//!
//! This crate provides a pure Rust ROS2 implementation that communicates over Zenoh,
//! compatible with `rmw_zenoh_cpp`. This allows ROS2 nodes built with this crate to
//! interoperate with standard ROS2 nodes using the Zenoh RMW.
//!
//! # Architecture
//!
//! The implementation follows the [rmw_zenoh design](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md):
//!
//! - Each [`Context`] maps to a Zenoh session
//! - [`Node`]s are logical groupings with liveliness tokens
//! - [`Publisher`]/[`Subscriber`] use Zenoh pub/sub
//! - [`Client`]/[`Server`] use Zenoh queryables
//! - Graph discovery via Zenoh liveliness tokens
//!
//! # Example
//!
//! ```ignore
//! use oxidros_zenoh::{Context, Node};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let ctx = Context::new()?;
//!     let node = ctx.create_node("my_node", None)?;
//!
//!     // Create publisher, subscriber, etc.
//!     Ok(())
//! }
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

mod attachment;
mod context;
mod error;
mod graph_cache;
mod keyexpr;
mod node;
mod qos;

pub mod service;
pub mod topic;

pub mod parameter;

// Re-exports
pub use context::Context;
pub use error::{Error, Result};
pub use graph_cache::GraphCache;
pub use node::Node;
pub use qos::QosMapping;
pub use service::ServiceRequest;

// Re-export core types
pub use oxidros_core::{
    Descriptor, DurabilityPolicy, FloatingPointRange, HistoryPolicy, IntegerRange,
    LivelinessPolicy, Parameter, Profile, ReliabilityPolicy, TypeSupport, Value,
};

// Re-export error types for compatibility
pub use oxidros_core::error::{ActionError, RclError};

// Re-export selector callback result
pub use oxidros_core::selector::CallbackResult;

// Re-export parameter storage
pub use oxidros_core::parameter::Parameters;
