//! ROS2 Node abstraction.
//!
//! A [`Node`] represents a ROS2 node and can create publishers, subscribers,
//! service clients, and service servers.

use crate::{
    attachment::generate_gid,
    context::Context,
    error::Result,
    keyexpr::{EntityKind, liveliness_node_keyexpr},
    service::{client::Client, server::Server},
    topic::{publisher::Publisher, subscriber::Subscriber},
};
use oxidros_core::qos::Profile;
use parking_lot::Mutex;
use ros2_types::TypeSupport;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use zenoh::Wait;
use zenoh::liveliness::LivelinessToken;

/// Inner node data.
struct NodeInner {
    /// Parent context.
    context: Context,
    /// Node ID within the context.
    node_id: u32,
    /// Node name.
    name: String,
    /// Node namespace.
    namespace: String,
    /// SROS enclave (empty if not set).
    enclave: String,
    /// Node GID.
    gid: [u8; 16],
    /// Next entity ID counter.
    next_entity_id: AtomicU32,
    /// Liveliness token for this node.
    _liveliness_token: Mutex<Option<LivelinessToken>>,
}

/// ROS2 Node.
///
/// A node is the fundamental unit of computation in ROS2. It can create
/// publishers, subscribers, service clients, and service servers.
///
/// # Example
///
/// ```ignore
/// let ctx = Context::new()?;
/// let node = ctx.create_node("my_node", Some("/robot1"))?;
///
/// let publisher = node.create_publisher::<std_msgs::msg::String>("chatter", None)?;
/// let subscriber = node.create_subscriber::<std_msgs::msg::String>("chatter", None)?;
/// ```
pub struct Node {
    inner: Arc<NodeInner>,
}

impl Node {
    /// Create a new node.
    pub(crate) fn new(
        context: Context,
        node_id: u32,
        name: &str,
        namespace: &str,
    ) -> Result<Arc<Self>> {
        let gid = generate_gid();

        // Create liveliness token key
        let token_key = liveliness_node_keyexpr(
            context.domain_id(),
            context.session_id(),
            node_id,
            "", // enclave
            namespace,
            name,
        );

        // Declare liveliness token
        let token = context
            .session()
            .liveliness()
            .declare_token(&token_key)
            .wait()?;

        let inner = Arc::new(NodeInner {
            context,
            node_id,
            name: name.to_string(),
            namespace: namespace.to_string(),
            enclave: String::new(),
            gid,
            next_entity_id: AtomicU32::new(10), // Start at 10 to match rmw_zenoh
            _liveliness_token: Mutex::new(Some(token)),
        });

        Ok(Arc::new(Node { inner }))
    }

    /// Get the node name.
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get the node namespace.
    pub fn namespace(&self) -> &str {
        &self.inner.namespace
    }

    /// Get the fully qualified node name.
    pub fn fully_qualified_name(&self) -> String {
        if self.inner.namespace.is_empty() {
            format!("/{}", self.inner.name)
        } else {
            format!("{}/{}", self.inner.namespace, self.inner.name)
        }
    }

    /// Get the node GID.
    pub fn gid(&self) -> &[u8; 16] {
        &self.inner.gid
    }

    /// Get the parent context.
    pub fn context(&self) -> &Context {
        &self.inner.context
    }

    /// Get the node ID.
    pub fn node_id(&self) -> u32 {
        self.inner.node_id
    }

    /// Get the enclave.
    pub fn enclave(&self) -> &str {
        &self.inner.enclave
    }

    /// Allocate a new entity ID.
    pub(crate) fn allocate_entity_id(&self) -> u32 {
        self.inner.next_entity_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Create a publisher.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Topic name (must be a valid ROS2 topic name)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Message type implementing `TypeSupport`
    pub fn create_publisher<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Publisher<T>> {
        // Validate topic name
        ros2args::names::validate_topic_name(topic_name)?;

        Publisher::new(
            self.clone(),
            topic_name,
            qos.unwrap_or_default(),
            EntityKind::Publisher,
        )
    }

    /// Create a subscriber.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Topic name (must be a valid ROS2 topic name)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Message type implementing `TypeSupport`
    pub fn create_subscriber<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Subscriber<T>> {
        // Validate topic name
        ros2args::names::validate_topic_name(topic_name)?;

        Subscriber::new(
            self.clone(),
            topic_name,
            qos.unwrap_or_default(),
            EntityKind::Subscriber,
        )
    }

    /// Create a service client.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name (must be a valid ROS2 service name)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Service type implementing `ServiceMsg`
    pub fn create_client<T: oxidros_core::ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Client<T>>
    where
        T::Request: TypeSupport,
        T::Response: TypeSupport,
    {
        // Validate service name
        ros2args::names::validate_topic_name(service_name)?;

        Client::new(
            self.clone(),
            service_name,
            qos.unwrap_or_else(Profile::services_default),
        )
    }

    /// Create a service server.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name (must be a valid ROS2 service name)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Service type implementing `ServiceMsg`
    pub fn create_server<T: oxidros_core::ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Server<T>>
    where
        T::Request: TypeSupport,
        T::Response: TypeSupport,
    {
        // Validate service name
        ros2args::names::validate_topic_name(service_name)?;

        Server::new(
            self.clone(),
            service_name,
            qos.unwrap_or_else(Profile::services_default),
        )
    }
}
