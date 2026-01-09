//! Abstract API traits for ROS2 implementations.
//!
//! This module defines the common traits that both `oxidros-rcl` and `oxidros-zenoh`
//! implementations must satisfy. Users can write code against these traits to be
//! agnostic of the underlying implementation.
//!
//! # Traits
//!
//! - [`RosContext`] - Factory for creating nodes
//! - [`RosNode`] - Creates publishers, subscribers, clients, and servers
//! - [`RosPublisher`] - Publishes messages to a topic
//! - [`RosSubscriber`] - Receives messages from a topic
//! - [`RosClient`] - Sends service requests and receives responses
//! - [`RosServer`] - Receives service requests and sends responses
//!
//! # Example
//!
//! ```ignore
//! use oxidros_core::api::*;
//!
//! fn setup<C: RosContext>(ctx: &C) -> Result<(), oxidros_core::Error> {
//!     let node = ctx.create_node("my_node", Some("/robot"))?;
//!     let publisher = node.create_publisher::<MyMsg>("topic", None)?;
//!     Ok(())
//! }
//! ```

use crate::{Result, ServiceMsg, TypeSupport, message::TakenMsg, qos::Profile};
use std::{borrow::Cow, sync::Arc};

// ============================================================================
// Common Types
// ============================================================================

/// A service request that can be responded to.
///
/// The `respond` method must be called to send a response back to the client.
pub trait ServiceRequest<T: ServiceMsg>: Send {
    /// Get the request data.
    fn request(&self) -> &T::Request;

    /// Send a response back to the client.
    ///
    /// Consumes self to ensure only one response is sent.
    fn respond(self, response: T::Response) -> Result<()>;
}

// ============================================================================
// Context Trait
// ============================================================================

/// A ROS2 context that can create nodes.
///
/// The context represents a connection to the ROS2 middleware (whether RCL/DDS or Zenoh)
/// and can create multiple nodes.
pub trait RosContext: Send + Sync + Sized {
    /// The node type created by this context.
    type Node: RosNode;

    /// Create a new node.
    ///
    /// # Arguments
    ///
    /// * `name` - The node name (must be a valid ROS2 name)
    /// * `namespace` - Optional namespace (defaults to "/")
    ///
    /// # Errors
    ///
    /// Returns an error if the name is invalid or node creation fails.
    fn create_node(
        self: &Arc<Self>,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<Arc<Self::Node>>;

    /// Get the domain ID.
    fn domain_id(&self) -> u32;
}

// ============================================================================
// Node Trait
// ============================================================================

/// A ROS2 node that can create publishers, subscribers, clients, and servers.
pub trait RosNode: Send + Sync + Sized {
    /// The publisher type created by this node.
    type Publisher<T: TypeSupport>: RosPublisher<T>;

    /// The subscriber type created by this node.
    /// The metadata type is implementation-specific.
    type Subscriber<T: TypeSupport>;

    /// The client type created by this node.
    type Client<T: ServiceMsg>: RosClient<T>;

    /// The server type created by this node.
    type Server<T: ServiceMsg>: RosServer<T>;

    /// Get the node name.
    fn name(&self) -> Cow<'_, str>;

    /// Get the node namespace.
    fn namespace(&self) -> Cow<'_, str>;

    /// Get the fully qualified node name (namespace + name).
    fn fully_qualified_name(&self) -> Cow<'_, str>;

    /// Create a publisher.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Topic name (can be relative or absolute)
    /// * `qos` - Optional QoS profile (uses default if None)
    fn create_publisher<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Publisher<T>>;

    /// Create a subscriber.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Topic name (can be relative or absolute)
    /// * `qos` - Optional QoS profile (uses default if None)
    fn create_subscriber<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Subscriber<T>>;

    /// Create a service client.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name (can be relative or absolute)
    /// * `qos` - Optional QoS profile (uses default if None)
    fn create_client<T: ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Client<T>>;

    /// Create a service server.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name (can be relative or absolute)
    /// * `qos` - Optional QoS profile (uses default if None)
    fn create_server<T: ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Server<T>>;
}

// ============================================================================
// Publisher Trait
// ============================================================================

/// A ROS2 publisher that can send messages to a topic.
pub trait RosPublisher<T: TypeSupport>: Send + Sync {
    /// Get the topic name.
    fn topic_name(&self) -> &str;

    /// Publish a message.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the publish operation fails.
    fn send(&self, msg: &T) -> Result<()>;
}

// ============================================================================
// Subscriber Trait
// ============================================================================

/// A ROS2 subscriber that can receive messages from a topic.
///
/// Returns [`TakenMsg<T>`] which supports both copied and zero-copy loaned messages.
pub trait RosSubscriber<T: TypeSupport>: Send {
    /// Get the topic name.
    fn topic_name(&self) -> &str;

    /// Receive a message asynchronously.
    ///
    /// This method waits until a message is available.
    /// Returns [`TakenMsg<T>`] which may be either a copied message or a
    /// zero-copy loaned message from shared memory.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the subscription is closed.
    fn recv(&mut self) -> impl std::future::Future<Output = Result<TakenMsg<T>>> + Send;

    /// Try to receive a message without blocking.
    ///
    /// Returns `Ok(None)` if no message is currently available.
    /// Returns [`TakenMsg<T>`] which may be either a copied message or a
    /// zero-copy loaned message from shared memory.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the subscription is closed.
    fn try_recv(&mut self) -> Result<Option<TakenMsg<T>>>;
}

// ============================================================================
// Client Trait
// ============================================================================

/// A ROS2 service client that can send requests and receive responses.
pub trait RosClient<T: ServiceMsg>: Send {
    /// Get the service name.
    fn service_name(&self) -> Cow<'_, str>;

    /// Check if the service is available.
    fn is_service_available(&self) -> bool;

    /// Send a request and wait for a response.
    ///
    /// Uses a default timeout (implementation-specific).
    fn call(
        &mut self,
        request: &T::Request,
    ) -> impl std::future::Future<Output = Result<T::Response>> + Send;
}

// ============================================================================
// Server Trait
// ============================================================================

/// A ROS2 service server that receives requests and sends responses.
pub trait RosServer<T: ServiceMsg>: Send {
    /// The request type returned by recv methods.
    type Request: ServiceRequest<T>;

    /// Get the service name.
    fn service_name(&self) -> Cow<'_, str>;

    /// Receive a request asynchronously.
    ///
    /// This method waits until a request is available.
    fn recv(&mut self) -> impl std::future::Future<Output = Result<Self::Request>> + Send;

    /// Try to receive a request without blocking.
    ///
    /// Returns `Ok(None)` if no request is currently available.
    fn try_recv(&mut self) -> Result<Option<Self::Request>>;
}
