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

use crate::{
    ActionMsg, Result, ServiceMsg, TypeDescription, TypeSupport, message::Message, qos::Profile,
};
use futures_core::Stream;
use std::{borrow::Cow, pin::Pin, sync::Arc, time::Duration};

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
    fn respond(self, response: &T::Response) -> Result<()>;
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
    fn create_node(
        self: &Arc<Self>,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<Arc<Self::Node>>;

    /// Create a new selector for event-driven execution.
    fn create_selector(self: &Arc<Self>) -> Result<Self::Selector>;

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
    fn create_publisher<T: TypeSupport + TypeDescription>(
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
    fn create_subscriber<T: TypeSupport + TypeDescription>(
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
    fn topic_name(&self) -> Result<Cow<'_, String>>;

    /// Publish a message.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the publish operation fails.
    fn send(&self, msg: &T) -> Result<()>;

    /// Publish raw serialized bytes directly (no serialization).
    ///
    /// Useful for message forwarding and bridge scenarios where data is already serialized.
    ///
    /// # Errors
    ///
    /// Returns an error if the publish operation fails.
    fn send_raw(&self, data: &[u8]) -> Result<()>;

    /// Publish multiple messages.
    ///
    /// Default implementation calls `send` for each message.
    /// Implementations may override for better efficiency.
    fn send_many<'a>(&self, messages: impl IntoIterator<Item = &'a T>) -> Result<()>
    where
        T: 'a,
    {
        for msg in messages {
            self.send(msg)?;
        }
        Ok(())
    }
}

// ============================================================================
// Subscriber Trait
// ============================================================================

/// Type alias for subscriber message streams.
pub type MessageStream<T> = Pin<Box<dyn Stream<Item = Result<Message<T>>> + Send>>;

/// A ROS2 subscriber that can receive messages from a topic.
///
/// Returns [`Message<T>`] which contains both the data and metadata.
pub trait RosSubscriber<T: TypeSupport>: Send {
    /// Get the topic name.
    fn topic_name(&self) -> Result<Cow<'_, String>>;

    /// Receive a message asynchronously.
    ///
    /// This method waits until a message is available.
    /// Returns [`Message<T>`] which contains the data (copied or loaned)
    /// and metadata (sequence number, timestamp, publisher GID).
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the subscription is closed.
    fn recv(&mut self) -> impl std::future::Future<Output = Result<Message<T>>> + Send;

    /// Try to receive a message without blocking.
    ///
    /// Returns `Ok(None)` if no message is currently available.
    /// Returns [`Message<T>`] which contains the data (copied or loaned)
    /// and metadata (sequence number, timestamp, publisher GID).
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the subscription is closed.
    fn try_recv(&mut self) -> Result<Option<Message<T>>>;

    /// Receive a raw CDR-encoded message asynchronously without deserializing.
    ///
    /// Returns the raw CDR bytes (including encapsulation header) and message metadata.
    /// Useful for message forwarding, recording, and dynamic decoding scenarios.
    fn recv_raw(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(Vec<u8>, crate::message::MessageInfo)>> + Send;

    /// Try to receive a raw CDR-encoded message without blocking or deserializing.
    ///
    /// Returns `Ok(None)` if no message is currently available.
    fn try_recv_raw(&mut self) -> Result<Option<(Vec<u8>, crate::message::MessageInfo)>>;

    /// Receive up to `limit` messages without blocking.
    ///
    /// Returns immediately with available messages, up to `limit`.
    /// Default implementation calls `try_recv` repeatedly.
    fn recv_many(&mut self, limit: usize) -> Result<Vec<Message<T>>> {
        let mut results = Vec::with_capacity(limit.min(64));
        while results.len() < limit {
            match self.try_recv()? {
                Some(msg) => results.push(msg),
                None => break,
            }
        }
        Ok(results)
    }

    /// Convert this subscriber into an async Stream.
    ///
    /// Consumes the subscriber and returns a stream that yields messages.
    fn into_stream(self) -> MessageStream<T>
    where
        Self: Sized + 'static;
}

// ============================================================================
// Client Trait
// ============================================================================

/// A ROS2 service client that can send requests and receive responses.
pub trait RosClient<T: ServiceMsg>: Send {
    /// Get the service name.
    fn service_name(&self) -> Result<Cow<'_, String>>;

    /// Check if the service is available.
    fn is_service_available(&self) -> bool;

    /// Send a request and wait for a response.
    ///
    /// Uses a default timeout (implementation-specific).
    fn call(
        &mut self,
        request: &T::Request,
    ) -> impl std::future::Future<Output = Result<Message<T::Response>>> + Send;

    /// Call with automatic retry on timeout, waits for service availability.
    ///
    /// This method first waits for the service to become available,
    /// then sends the request with the specified timeout. If the call
    /// times out, it will retry indefinitely until a response is received.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to send
    /// * `timeout` - Timeout for each call attempt
    fn call_with_retry(
        &mut self,
        request: &T::Request,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<Message<T::Response>>> + Send;
}

// ============================================================================
// Server Trait
// ============================================================================

/// A ROS2 service server that receives requests and sends responses.
pub trait RosServer<T: ServiceMsg>: Send {
    /// The request type returned by recv methods.
    type Request: ServiceRequest<T>;

    /// Get the service name.
    fn service_name(&self) -> Result<Cow<'_, String>>;

    /// Receive a request asynchronously.
    ///
    /// This method waits until a request is available.
    fn recv(&mut self) -> impl std::future::Future<Output = Result<Self::Request>> + Send;

    /// Try to receive a request without blocking.
    ///
    /// Returns `Ok(None)` if no request is currently available.
    fn try_recv(&mut self) -> Result<Option<Self::Request>>;

    /// Run a serving loop with the given handler.
    ///
    /// Continuously receives requests and invokes the handler to generate responses.
    /// The handler receives the request message and must return the response.
    ///
    /// # Arguments
    ///
    /// * `handler` - A function that takes a request and returns a response
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the server shuts down gracefully, or an error if
    /// receiving or sending fails.
    fn serve<F>(self, handler: F) -> impl std::future::Future<Output = Result<()>> + Send
    where
        Self: Sized,
        F: FnMut(Message<T::Request>) -> T::Response + Send;

    /// Run a serving loop with an async handler.
    ///
    /// Like [`serve`](RosServer::serve), but the handler returns a future,
    /// allowing async work (e.g. calling another service) while processing each request.
    ///
    /// # Arguments
    ///
    /// * `handler` - An async function that takes a request and returns a response
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the server shuts down gracefully, or an error if
    /// receiving or sending fails.
    fn serve_async<F, Fut>(
        self,
        handler: F,
    ) -> impl std::future::Future<Output = Result<()>> + Send
    where
        Self: Sized,
        F: FnMut(Message<T::Request>) -> Fut + Send,
        Fut: std::future::Future<Output = T::Response> + Send;
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
    fn add_subscriber<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Self::Subscriber<T>,
        handler: Box<dyn FnMut(Message<T>)>,
    ) -> bool;

    /// Register a server with a callback function.
    ///
    /// The callback receives requests and must return responses.
    /// Takes ownership of the server.
    ///
    /// # Returns
    ///
    /// `true` if successfully added, `false` if context mismatch.
    fn add_server<T: ServiceMsg + 'static>(
        &mut self,
        server: Self::Server<T>,
        handler: Box<dyn FnMut(Message<T::Request>) -> T::Response>,
    ) -> bool;

    /// Register a parameter server with a callback.
    ///
    /// The callback is invoked when parameters are updated.
    fn add_parameter_server(
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
    fn add_timer(&mut self, duration: Duration, handler: Box<dyn FnMut()>) -> u64;

    /// Add a repeating wall timer.
    ///
    /// The callback is invoked periodically at the specified interval.
    ///
    /// # Returns
    ///
    /// A timer ID that can be used to remove the timer.
    fn add_wall_timer(&mut self, name: &str, period: Duration, handler: Box<dyn FnMut()>) -> u64;

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
    fn add_action_server<T, GR, A, CR>(
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
    fn add_action_client<T: ActionMsg + 'static>(
        &mut self,
        client: Self::ActionClient<T>,
    ) -> Result<bool>;

    /// Wait for events and invoke registered callbacks.
    ///
    /// Blocks until at least one event occurs.
    fn wait(&mut self) -> Result<()>;

    /// Wait for events with a timeout.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - At least one event occurred
    /// - `Ok(false)` - Timeout elapsed with no events
    /// - `Err(_)` - An error occurred
    fn wait_timeout(&mut self, timeout: Duration) -> Result<bool>;
}
