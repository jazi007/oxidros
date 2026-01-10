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

use crate::{ActionMsg, Result, ServiceMsg, TypeSupport, message::TakenMsg, qos::Profile};
use std::{borrow::Cow, sync::Arc, time::Duration};

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

    /// The selector type created by this context.
    type Selector: RosSelector;

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
    fn new_node(self: &Arc<Self>, name: &str, namespace: Option<&str>) -> Result<Arc<Self::Node>>;

    /// Create a new selector for event-driven execution.
    fn new_selector(self: &Arc<Self>) -> Result<Self::Selector>;

    /// Get the domain ID.
    fn ros_domain_id(&self) -> u32;
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
    fn name(&self) -> Result<String>;

    /// Get the node namespace.
    fn namespace(&self) -> Result<String>;

    /// Get the fully qualified node name (namespace + name).
    fn fully_qualified_name(&self) -> Result<String>;

    /// Create a publisher.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Topic name (can be relative or absolute)
    /// * `qos` - Optional QoS profile (uses default if None)
    fn new_publisher<T: TypeSupport>(
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
    fn new_subscriber<T: TypeSupport>(
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
    fn new_client<T: ServiceMsg>(
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
    fn new_server<T: ServiceMsg>(
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
    fn topic_name(&self) -> Result<Cow<'_, String>>;

    /// Publish a message.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the publish operation fails.
    fn publish(&self, msg: &T) -> Result<()>;
}

// ============================================================================
// Subscriber Trait
// ============================================================================

/// A ROS2 subscriber that can receive messages from a topic.
///
/// Returns [`TakenMsg<T>`] which supports both copied and zero-copy loaned messages.
pub trait RosSubscriber<T: TypeSupport>: Send {
    /// Get the topic name.
    fn topic_name(&self) -> Result<Cow<'_, String>>;

    /// Receive a message asynchronously.
    ///
    /// This method waits until a message is available.
    /// Returns [`TakenMsg<T>`] which may be either a copied message or a
    /// zero-copy loaned message from shared memory.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the subscription is closed.
    fn recv_msg(&mut self) -> impl std::future::Future<Output = Result<TakenMsg<T>>> + Send;

    /// Try to receive a message without blocking.
    ///
    /// Returns `Ok(None)` if no message is currently available.
    /// Returns [`TakenMsg<T>`] which may be either a copied message or a
    /// zero-copy loaned message from shared memory.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the subscription is closed.
    fn try_recv_msg(&mut self) -> Result<Option<TakenMsg<T>>>;
}

// ============================================================================
// Client Trait
// ============================================================================

/// A ROS2 service client that can send requests and receive responses.
pub trait RosClient<T: ServiceMsg>: Send {
    /// Get the service name.
    fn service_name(&self) -> Cow<'_, str>;

    /// Check if the service is available.
    fn service_available(&self) -> bool;

    /// Send a request and wait for a response.
    ///
    /// Uses a default timeout (implementation-specific).
    fn call_service(
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
    fn recv_request(&mut self) -> impl std::future::Future<Output = Result<Self::Request>> + Send;

    /// Try to receive a request without blocking.
    ///
    /// Returns `Ok(None)` if no request is currently available.
    fn try_recv_request(&mut self) -> Result<Option<Self::Request>>;
}

// ============================================================================
// Selector Trait
// ============================================================================

type ParameterHandler =
    Box<dyn FnMut(&mut crate::parameter::Parameters, std::collections::BTreeSet<String>)>;

/// A ROS2 selector for event-driven single-threaded execution.
///
/// The selector allows registering callbacks for subscribers, servers, timers,
/// and other event sources, then waiting for events in a loop.
///
/// # Example
///
/// ```ignore
/// use oxidros_core::api::RosSelector;
///
/// fn spin<S: RosSelector>(mut selector: S) -> Result<(), oxidros_core::Error> {
///     loop {
///         selector.wait()?;
///     }
/// }
/// ```
pub trait RosSelector: Sized {
    /// The subscriber type this selector accepts.
    type Subscriber<T: TypeSupport>;

    /// The server type this selector accepts.
    type Server<T: ServiceMsg>;

    /// The action server type this selector accepts.
    type ActionServer<T: ActionMsg>;

    /// The action client type this selector accepts.
    type ActionClient<T: ActionMsg>;

    /// The parameter server type this selector accepts.
    type ParameterServer;

    /// Register a subscriber with a callback function.
    ///
    /// The callback is invoked when messages arrive on the topic.
    /// Takes ownership of the subscriber.
    ///
    /// # Returns
    ///
    /// `true` if successfully added, `false` if context mismatch.
    fn add_subscriber_handler<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Self::Subscriber<T>,
        handler: Box<dyn FnMut(TakenMsg<T>)>,
    ) -> bool;

    /// Register a server with a callback function.
    ///
    /// The callback receives requests and must return responses.
    /// Takes ownership of the server.
    ///
    /// # Returns
    ///
    /// `true` if successfully added, `false` if context mismatch.
    fn add_server_handler<T: ServiceMsg + 'static>(
        &mut self,
        server: Self::Server<T>,
        handler: Box<dyn FnMut(T::Request) -> T::Response>,
    ) -> bool;

    /// Register a parameter server with a callback.
    ///
    /// The callback is invoked when parameters are updated.
    fn add_parameter_server_handler(
        &mut self,
        param_server: Self::ParameterServer,
        handler: ParameterHandler,
    );

    /// Add a one-shot timer.
    ///
    /// The callback is invoked once after the specified duration.
    ///
    /// # Returns
    ///
    /// A timer ID that can be used to remove the timer.
    fn add_timer_handler(&mut self, duration: Duration, handler: Box<dyn FnMut()>) -> u64;

    /// Add a repeating wall timer.
    ///
    /// The callback is invoked periodically at the specified interval.
    ///
    /// # Returns
    ///
    /// A timer ID that can be used to remove the timer.
    fn add_wall_timer_handler(
        &mut self,
        name: &str,
        period: Duration,
        handler: Box<dyn FnMut()>,
    ) -> u64;

    /// Remove a timer by its ID.
    fn delete_timer(&mut self, id: u64);

    /// Register an action server with handlers.
    ///
    /// # Arguments
    ///
    /// * `server` - The action server
    /// * `goal_handler` - Called when a new goal arrives, returns `true` to accept
    /// * `accept_handler` - Called after goal is accepted, receives the goal handle
    /// * `cancel_handler` - Called when cancel is requested, returns `true` to cancel
    ///
    /// # Returns
    ///
    /// `Ok(true)` if successfully added, `Ok(false)` if context mismatch,
    /// `Err(NotSupported)` if actions are not supported by this backend.
    fn add_action_server_handler<T, GR, A, CR>(
        &mut self,
        server: Self::ActionServer<T>,
        goal_handler: GR,
        accept_handler: A,
        cancel_handler: CR,
    ) -> Result<bool>
    where
        T: ActionMsg + 'static,
        GR: Fn(&<T::Goal as crate::ActionGoal>::Request) -> bool + 'static,
        A: Fn(Self::ActionGoalHandle<T>) + 'static,
        CR: Fn(&[u8; 16]) -> bool + 'static;

    /// The goal handle type for action servers.
    type ActionGoalHandle<T: ActionMsg>;

    /// Register an action client for async operations.
    ///
    /// This is primarily for internal use by the async action client.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if successfully added, `Ok(false)` if context mismatch,
    /// `Err(NotSupported)` if actions are not supported by this backend.
    fn add_action_client_handler<T: ActionMsg + 'static>(
        &mut self,
        client: Self::ActionClient<T>,
    ) -> Result<bool>;

    /// Wait for events and invoke registered callbacks.
    ///
    /// Blocks until at least one event occurs.
    fn spin_once(&mut self) -> Result<()>;

    /// Wait for events with a timeout.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - At least one event occurred
    /// - `Ok(false)` - Timeout elapsed with no events
    /// - `Err(_)` - An error occurred
    fn spin_timeout(&mut self, timeout: Duration) -> Result<bool>;
}
