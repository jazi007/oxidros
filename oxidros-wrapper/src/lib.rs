//! Ergonomic ROS2 API implementing oxidros-core traits for the RCL backend.
//!
//! This crate provides newtype wrappers around `oxidros-rcl` types and implements
//! the core API traits, making it easy to work with ROS2 using async/await patterns.
//!
//! The newtype pattern is used to satisfy Rust's orphan rules while keeping
//! `oxidros-rcl` minimal (FFI only).
//!
//! # Example
//!
//! ```ignore
//! use oxidros_wrapper::prelude::*;
//! use oxidros_wrapper::msg::common_interfaces::std_msgs;
//! use tokio::signal::ctrl_c;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let ctx = Context::new()?;
//!     let node = ctx.create_node("my_node", None)?;
//!
//!     // Create publisher using core trait
//!     let publisher = node.create_publisher::<std_msgs::msg::String>("topic", None)?;
//!
//!     // Create subscriber and convert to stream
//!     let mut subscriber = node
//!         .create_subscriber::<std_msgs::msg::String>("topic", None)?
//!         .into_stream();
//!
//!     loop {
//!         tokio::select! {
//!             Some(Ok(msg)) = subscriber.next() => {
//!                 println!("Received: {:?}", msg.sample.data.get_string());
//!             }
//!             _ = ctrl_c() => break,
//!         }
//!     }
//!     Ok(())
//! }
//! ```

#![deny(
    missing_docs,
    bad_style,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    unconditional_recursion,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    clippy::unwrap_used,
    clippy::expect_used
)]

use futures_core::Stream;
use std::task::{Context as TaskContext, Poll};
use std::{
    borrow::Cow,
    collections::BTreeSet,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

// Re-export core traits and types
pub use oxidros_core::{
    ActionGoal,
    ActionMsg,
    DurabilityPolicy,
    Error,
    HistoryPolicy,
    LivelinessPolicy,
    // Types
    Message,
    MessageStream,
    Profile,
    ReliabilityPolicy,
    Result,
    // Traits
    RosClient,
    RosContext,
    RosNode,
    RosPublisher,
    RosSelector,
    RosServer,
    RosSubscriber,
    ServiceMsg,
    ServiceRequest as ServiceRequestTrait,
    TypeSupport,
};

// Re-export callback result from core
pub use oxidros_core::selector::CallbackResult;

// Re-export message types
pub use oxidros_msg as msg;

// Re-export clock from rcl
pub use oxidros_rcl::clock::{self, Clock};

// Re-export logger from rcl
pub use oxidros_rcl::logger;

// Re-export parameter types
pub use oxidros_rcl::parameter::{self, ParameterServer};

// Re-export action types
use oxidros_msg::interfaces::action_msgs::msg::GoalInfo;
pub use oxidros_rcl::{action, service, topic};

// ============================================================================
// Newtype Wrappers
// ============================================================================

/// A ROS2 context wrapper implementing [`RosContext`].
///
/// Wraps an `Arc<oxidros_rcl::context::Context>`.
pub struct Context(pub Arc<oxidros_rcl::context::Context>);

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").finish_non_exhaustive()
    }
}

impl Context {
    /// Create a new context with default options.
    pub fn new() -> Result<Arc<Self>> {
        let inner = oxidros_rcl::context::Context::new()?;
        Ok(Arc::new(Self(inner)))
    }

    /// Get the inner RCL context.
    pub fn inner(&self) -> &Arc<oxidros_rcl::context::Context> {
        &self.0
    }
}

impl Deref for Context {
    type Target = oxidros_rcl::context::Context;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A ROS2 node wrapper implementing [`RosNode`].
///
/// Wraps an `Arc<oxidros_rcl::node::Node>`.
pub struct Node(pub Arc<oxidros_rcl::node::Node>);

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").finish_non_exhaustive()
    }
}

impl Node {
    /// Get the inner RCL node.
    pub fn inner(&self) -> &Arc<oxidros_rcl::node::Node> {
        &self.0
    }

    /// Create a parameter server for this node.
    pub fn create_parameter_server(self: &Arc<Self>) -> Result<ParameterServer> {
        self.0.create_parameter_server()
    }
}

impl Deref for Node {
    type Target = oxidros_rcl::node::Node;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A ROS2 publisher wrapper implementing [`RosPublisher`].
pub struct Publisher<T>(pub oxidros_rcl::topic::publisher::Publisher<T>);

impl<T> std::fmt::Debug for Publisher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Publisher").finish_non_exhaustive()
    }
}

impl<T> Publisher<T> {
    /// Get the inner RCL publisher.
    pub fn inner(&self) -> &oxidros_rcl::topic::publisher::Publisher<T> {
        &self.0
    }
}

impl<T> Deref for Publisher<T> {
    type Target = oxidros_rcl::topic::publisher::Publisher<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A ROS2 subscriber wrapper implementing [`RosSubscriber`].
pub struct Subscriber<T>(pub oxidros_rcl::topic::subscriber::Subscriber<T>);

impl<T> std::fmt::Debug for Subscriber<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber").finish_non_exhaustive()
    }
}

impl<T> Subscriber<T> {
    /// Get the inner RCL subscriber.
    pub fn inner(&self) -> &oxidros_rcl::topic::subscriber::Subscriber<T> {
        &self.0
    }
}

impl<T> Deref for Subscriber<T> {
    type Target = oxidros_rcl::topic::subscriber::Subscriber<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Subscriber<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A ROS2 service client wrapper implementing [`RosClient`].
pub struct Client<T>(pub oxidros_rcl::service::client::Client<T>);

impl<T> std::fmt::Debug for Client<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl<T> Client<T> {
    /// Get the inner RCL client.
    pub fn inner(&self) -> &oxidros_rcl::service::client::Client<T> {
        &self.0
    }
}

impl<T> Deref for Client<T> {
    type Target = oxidros_rcl::service::client::Client<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Client<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A ROS2 service server wrapper implementing [`RosServer`].
pub struct Server<T>(pub oxidros_rcl::service::server::Server<T>);

impl<T> std::fmt::Debug for Server<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server").finish_non_exhaustive()
    }
}

impl<T> Server<T> {
    /// Get the inner RCL server.
    pub fn inner(&self) -> &oxidros_rcl::service::server::Server<T> {
        &self.0
    }
}

impl<T> Deref for Server<T> {
    type Target = oxidros_rcl::service::server::Server<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Server<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A ROS2 selector wrapper implementing [`RosSelector`].
pub struct Selector(pub oxidros_rcl::selector::Selector);

impl std::fmt::Debug for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Selector").finish_non_exhaustive()
    }
}

impl Selector {
    /// Get the inner RCL selector.
    pub fn inner(&self) -> &oxidros_rcl::selector::Selector {
        &self.0
    }
}

impl Deref for Selector {
    type Target = oxidros_rcl::selector::Selector;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Selector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A service request wrapper implementing [`ServiceRequestTrait`].
pub struct ServiceRequest<T: ServiceMsg> {
    inner: oxidros_rcl::service::server::ServiceRequest<T>,
}

impl<T: ServiceMsg> std::fmt::Debug for ServiceRequest<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceRequest").finish_non_exhaustive()
    }
}

impl<T: ServiceMsg> ServiceRequest<T>
where
    T::Response: TypeSupport,
{
    /// Create from inner RCL service request.
    pub fn new(inner: oxidros_rcl::service::server::ServiceRequest<T>) -> Self {
        Self { inner }
    }

    /// Get the inner RCL service request.
    pub fn into_inner(self) -> oxidros_rcl::service::server::ServiceRequest<T> {
        self.inner
    }
}

// ============================================================================
// SubscriberStream - Stream wrapper for async subscription
// ============================================================================

use futures_util::ready;
use tokio_util::sync::ReusableBoxFuture;

/// A stream of messages from a subscriber.
pub struct SubscriberStream<T: TypeSupport + Send + 'static> {
    inner: ReusableBoxFuture<'static, (Result<Message<T>>, Subscriber<T>)>,
}

impl<T: TypeSupport + Send + 'static> std::fmt::Debug for SubscriberStream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubscriberStream").finish_non_exhaustive()
    }
}

async fn make_subscriber_future<T: TypeSupport + Send + 'static>(
    mut subscriber: Subscriber<T>,
) -> (Result<Message<T>>, Subscriber<T>) {
    let result = subscriber.0.recv().await;
    (result, subscriber)
}

impl<T: TypeSupport + Send + 'static> SubscriberStream<T> {
    /// Create a new stream from a subscriber.
    pub fn new(subscriber: Subscriber<T>) -> Self {
        Self {
            inner: ReusableBoxFuture::new(make_subscriber_future(subscriber)),
        }
    }
}

impl<T: TypeSupport + Send + 'static> Stream for SubscriberStream<T> {
    type Item = Result<Message<T>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let (result, subscriber) = ready!(self.inner.poll(cx));
        self.inner.set(make_subscriber_future(subscriber));
        Poll::Ready(Some(result))
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl RosContext for Context {
    type Node = Node;
    type Selector = Selector;

    fn create_node(
        self: &Arc<Self>,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<Arc<Self::Node>> {
        let inner = self.0.create_node(name, namespace)?;
        Ok(Arc::new(Node(inner)))
    }

    fn create_selector(self: &Arc<Self>) -> Result<Self::Selector> {
        let inner = self.0.create_selector()?;
        Ok(Selector(inner))
    }

    fn ros_domain_id(&self) -> u32 {
        // RCL uses the ROS_DOMAIN_ID environment variable, default is 0
        std::env::var("ROS_DOMAIN_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }
}

impl RosNode for Node {
    type Publisher<T: TypeSupport> = Publisher<T>;
    type Subscriber<T: TypeSupport> = Subscriber<T>;
    type Client<T: ServiceMsg> = Client<T>;
    type Server<T: ServiceMsg> = Server<T>;

    fn name(&self) -> Result<String> {
        self.0.name()
    }

    fn namespace(&self) -> Result<String> {
        self.0.namespace()
    }

    fn fully_qualified_name(&self) -> Result<String> {
        self.0.fully_qualified_name()
    }

    fn create_publisher<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Publisher<T>> {
        let inner = self.0.create_publisher(topic_name, qos)?;
        Ok(Publisher(inner))
    }

    fn create_subscriber<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Subscriber<T>> {
        let inner = self.0.create_subscriber(topic_name, qos)?;
        Ok(Subscriber(inner))
    }

    fn create_client<T: ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Client<T>> {
        let inner = self.0.create_client(service_name, qos)?;
        Ok(Client(inner))
    }

    fn create_server<T: ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Server<T>> {
        let inner = self.0.create_server(service_name, qos)?;
        Ok(Server(inner))
    }
}

impl<T: TypeSupport> RosPublisher<T> for Publisher<T> {
    fn topic_name(&self) -> Result<Cow<'_, String>> {
        self.0.topic_name()
    }

    fn send(&self, msg: &T) -> Result<()> {
        self.0.send(msg)
    }

    fn send_raw(&self, data: &[u8]) -> Result<()> {
        // SAFETY: The raw bytes are passed directly to RCL
        unsafe { self.0.send_raw(data) }
    }
}

impl<T: TypeSupport + Send + 'static> RosSubscriber<T> for Subscriber<T> {
    fn topic_name(&self) -> Result<Cow<'_, String>> {
        self.0.topic_name()
    }

    async fn recv(&mut self) -> Result<Message<T>> {
        self.0.recv().await
    }

    fn try_recv(&mut self) -> Result<Option<Message<T>>> {
        self.0.try_recv()
    }

    async fn recv_raw(&mut self) -> Result<(Vec<u8>, oxidros_core::message::MessageInfo)> {
        self.0.recv_raw().await
    }

    fn try_recv_raw(&mut self) -> Result<Option<(Vec<u8>, oxidros_core::message::MessageInfo)>> {
        self.0.try_recv_raw()
    }

    fn into_stream(self) -> MessageStream<T>
    where
        Self: Sized + 'static,
    {
        Box::pin(SubscriberStream::new(self))
    }
}

impl<T: ServiceMsg> RosClient<T> for Client<T> {
    fn service_name(&self) -> Result<Cow<'_, String>> {
        self.0.service_name()
    }

    fn is_service_available(&self) -> bool {
        self.0.is_service_available()
    }

    async fn call(&mut self, request: &T::Request) -> Result<Message<T::Response>> {
        self.0.call(request).await
    }

    async fn call_with_retry(
        &mut self,
        request: &T::Request,
        timeout: Duration,
    ) -> Result<Message<T::Response>> {
        use tokio::time;

        // Wait for service availability
        while !self.0.is_service_available() {
            time::sleep(Duration::from_millis(100)).await;
        }

        // Retry loop with timeout
        loop {
            match time::timeout(timeout, self.0.call(request)).await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    tracing::warn!("Service call timeout, retrying...");
                }
            }
        }
    }
}

impl<T: ServiceMsg> ServiceRequestTrait<T> for ServiceRequest<T>
where
    T::Response: TypeSupport,
{
    fn request(&self) -> &T::Request {
        self.inner.request()
    }

    fn respond(self, response: &T::Response) -> Result<()> {
        self.inner.send(response)
    }
}

impl<T: ServiceMsg> RosServer<T> for Server<T>
where
    T::Response: TypeSupport,
{
    type Request = ServiceRequest<T>;

    fn service_name(&self) -> Result<Cow<'_, String>> {
        self.0.service_name()
    }

    async fn recv(&mut self) -> Result<Self::Request> {
        let inner = self.0.recv().await?;
        Ok(ServiceRequest::new(inner))
    }

    fn try_recv(&mut self) -> Result<Option<Self::Request>> {
        match self.0.try_recv()? {
            Some(inner) => Ok(Some(ServiceRequest::new(inner))),
            None => Ok(None),
        }
    }

    async fn serve<F>(mut self, mut handler: F) -> Result<()>
    where
        Self: Sized,
        F: FnMut(Message<T::Request>) -> T::Response + Send,
    {
        loop {
            match self.0.recv().await {
                Ok(service_req) => {
                    let (sender, request) = service_req.split();
                    let response = handler(request);
                    if let Err(e) = sender.send(&response) {
                        tracing::error!("Failed to send response: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Error receiving request: {:?}", e);
                    return Err(e);
                }
            }
        }
    }

    async fn serve_async<F, Fut>(mut self, mut handler: F) -> Result<()>
    where
        Self: Sized,
        F: FnMut(Message<T::Request>) -> Fut + Send,
        Fut: std::future::Future<Output = T::Response> + Send,
    {
        loop {
            match self.0.recv().await {
                Ok(service_req) => {
                    let (sender, request) = service_req.split();
                    let response = handler(request).await;
                    if let Err(e) = sender.send(&response) {
                        tracing::error!("Failed to send response: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Error receiving request: {:?}", e);
                    return Err(e);
                }
            }
        }
    }
}

impl RosSelector for Selector {
    type Subscriber<T: TypeSupport> = Subscriber<T>;
    type Server<T: ServiceMsg> = Server<T>;
    type ActionServer<T: ActionMsg> = action::server::Server<T>;
    type ActionClient<T: ActionMsg> = action::client::Client<T>;
    type ActionGoalHandle<T: ActionMsg> = action::handle::GoalHandle<T>;
    type ParameterServer = ParameterServer;

    fn add_subscriber<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Self::Subscriber<T>,
        handler: Box<dyn FnMut(Message<T>)>,
    ) -> bool {
        self.0.add_subscriber(subscriber.0, handler)
    }

    fn add_server<T: ServiceMsg + 'static>(
        &mut self,
        server: Self::Server<T>,
        handler: Box<dyn FnMut(Message<T::Request>) -> T::Response>,
    ) -> bool {
        self.0.add_server(server.0, handler)
    }

    fn add_parameter_server(
        &mut self,
        param_server: Self::ParameterServer,
        handler: Box<dyn FnMut(&mut oxidros_core::parameter::Parameters, BTreeSet<String>)>,
    ) {
        self.0.add_parameter_server(param_server, handler)
    }

    fn add_timer(&mut self, duration: Duration, handler: Box<dyn FnMut()>) -> u64 {
        self.0.add_timer(duration, handler)
    }

    fn add_wall_timer(&mut self, name: &str, period: Duration, handler: Box<dyn FnMut()>) -> u64 {
        self.0.add_wall_timer(name, period, handler)
    }

    fn delete_timer(&mut self, id: u64) {
        self.0.remove_timer(id)
    }

    fn add_action_server<T, GR, A, CR>(
        &mut self,
        server: Self::ActionServer<T>,
        goal_handler: GR,
        accept_handler: A,
        cancel_handler: CR,
    ) -> Result<bool>
    where
        T: ActionMsg + 'static,
        GR: Fn(&<T::Goal as ActionGoal>::Request) -> bool + 'static,
        A: Fn(Self::ActionGoalHandle<T>) + 'static,
        CR: Fn(&[u8; 16]) -> bool + 'static,
    {
        type SendGoalServiceRequest<T> = <<T as ActionMsg>::Goal as ActionGoal>::Request;

        let wrapped_goal = move |req: SendGoalServiceRequest<T>| -> bool { goal_handler(&req) };
        let wrapped_cancel = move |info: &GoalInfo| -> bool { cancel_handler(&info.goal_id.uuid) };
        Ok(self
            .0
            .add_action_server(server, wrapped_goal, accept_handler, wrapped_cancel))
    }

    fn add_action_client<T: ActionMsg + 'static>(
        &mut self,
        _client: Self::ActionClient<T>,
    ) -> Result<bool> {
        // Action client registration is handled internally by the client
        // The selector only needs to know about it for polling
        // For now, action clients are not fully supported via selector
        Err(Error::NotImplemented {
            feature: "RosSelector::add_action_client".to_string(),
            reason: "Action clients use internal registration".to_string(),
        })
    }

    fn wait(&mut self) -> Result<()> {
        self.0.wait()
    }

    fn wait_timeout(&mut self, timeout: Duration) -> Result<bool> {
        self.0.wait_timeout(timeout)
    }
}

// ============================================================================
// Prelude
// ============================================================================

/// Prelude module for convenient imports.
pub mod prelude {
    pub use super::{
        CallbackResult,
        Client,
        Clock,
        // Wrapper types
        Context,
        DurabilityPolicy,
        Error,
        HistoryPolicy,
        LivelinessPolicy,
        // Other types
        Message,
        MessageStream,
        Node,
        Profile,
        Publisher,
        ReliabilityPolicy,
        Result,
        // Core traits
        RosClient,
        RosContext,
        RosNode,
        RosPublisher,
        RosSelector,
        RosServer,
        RosSubscriber,
        Selector,
        Server,
        ServiceRequestTrait as ServiceRequest,
        Subscriber,
        SubscriberStream,
    };
    pub use futures_core::Stream;
    pub use futures_util::StreamExt;
    pub use std::sync::Arc;
}
