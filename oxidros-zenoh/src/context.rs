//! Zenoh context (session) management.
//!
//! A [`Context`] wraps a Zenoh session and provides the foundation for
//! creating ROS2 nodes and entities.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Contexts](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#contexts)

use crate::{
    error::{Error, Result, Ros2ArgsResultExt},
    graph_cache::GraphCache,
    node::Node,
};
use parking_lot::Mutex;
use ros2args::Ros2Args;
use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};
use zenoh::{Session, Wait};

/// Environment variable for custom Zenoh session config.
pub const ZENOH_SESSION_CONFIG_URI: &str = "ZENOH_SESSION_CONFIG_URI";

/// Environment variable for ROS domain ID.
pub const ROS_DOMAIN_ID: &str = "ROS_DOMAIN_ID";

/// Default Zenoh router endpoint.
pub const DEFAULT_ROUTER_ENDPOINT: &str = "tcp/localhost:7447";

/// Inner context data.
struct ContextInner {
    /// Zenoh session.
    session: Session,
    /// ROS domain ID.
    domain_id: u32,
    /// Session ID as hex string.
    session_id: String,
    /// Next node ID counter.
    next_node_id: AtomicU32,
    /// Graph cache for entity discovery.
    graph_cache: Arc<Mutex<GraphCache>>,
    /// Parsed ROS2 command-line arguments.
    ros2_args: Ros2Args,
    /// Liveliness subscriber for graph discovery (must be kept alive).
    _liveliness_subscriber: Mutex<Option<zenoh::pubsub::Subscriber<()>>>,
}

/// ROS2 context wrapping a Zenoh session.
///
/// A context represents a single Zenoh session and can contain multiple nodes.
/// All nodes within a context share the same session for communication.
///
/// # Example
///
/// ```ignore
/// let ctx = Context::new()?;
/// let node = ctx.create_node("my_node", None)?;
/// ```
#[derive(Clone)]
pub struct Context {
    inner: Arc<ContextInner>,
}

impl Context {
    /// Create a new context with default configuration.
    ///
    /// This will:
    /// 1. Parse ROS2 command-line arguments from `std::env::args()`
    /// 2. Read `ROS_DOMAIN_ID` from environment (default: 0)
    /// 3. Read `ZENOH_SESSION_CONFIG_URI` for custom config (optional)
    /// 4. Open a Zenoh session in peer mode connecting to localhost:7447
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - ROS2 arguments are malformed
    /// - The Zenoh session cannot be opened
    pub fn new() -> Result<Arc<Self>> {
        // Parse ROS2 arguments from environment
        let ros2_args = Ros2Args::from_env().map_name_err()?;

        Self::with_args(ros2_args)
    }

    /// Create a new context with pre-parsed ROS2 arguments.
    ///
    /// This is useful when you want to parse arguments yourself or
    /// provide custom arguments programmatically.
    pub fn with_args(ros2_args: Ros2Args) -> Result<Arc<Self>> {
        // Get domain ID from environment
        let domain_id = env::var(ROS_DOMAIN_ID)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Self::with_args_and_domain_id(ros2_args, domain_id)
    }

    /// Create a new context with pre-parsed ROS2 arguments and specific domain ID.
    pub fn with_args_and_domain_id(ros2_args: Ros2Args, domain_id: u32) -> Result<Arc<Self>> {
        // Build Zenoh config
        let mut config = zenoh::Config::default();

        // Check for custom config file
        if let Ok(config_uri) = env::var(ZENOH_SESSION_CONFIG_URI) {
            config = zenoh::Config::from_file(&config_uri)
                .map_err(|e| Error::InvalidConfig(format!("Failed to load config: {}", e)))?;
        } else {
            // Default config: peer mode, connect to local router
            config
                .connect
                .endpoints
                .set(vec![DEFAULT_ROUTER_ENDPOINT.parse().unwrap()])
                .map_err(|e| Error::InvalidConfig(format!("Failed to set endpoints: {:?}", e)))?;
        }

        // Enable timestamping for AdvancedPublisher with Sequencing::Timestamp
        config.insert_json5(
            "timestamping/enabled",
            &serde_json::json!({"router": true, "peer": true, "client": true}).to_string(),
        )?;

        Self::with_full_config(ros2_args, domain_id, config)
    }

    /// Create a new context with a specific domain ID (legacy API).
    ///
    /// This parses ROS2 arguments from `std::env::args()`.
    pub fn with_domain_id(domain_id: u32) -> Result<Arc<Self>> {
        let ros2_args = Ros2Args::from_env().map_name_err()?;
        Self::with_args_and_domain_id(ros2_args, domain_id)
    }

    /// Create a new context with custom Zenoh configuration.
    ///
    /// This parses ROS2 arguments from `std::env::args()`.
    pub fn with_config(domain_id: u32, config: zenoh::Config) -> Result<Arc<Self>> {
        let ros2_args = Ros2Args::from_env().map_name_err()?;
        Self::with_full_config(ros2_args, domain_id, config)
    }

    /// Create a new context with full configuration.
    ///
    /// This is the most flexible constructor that allows specifying all options.
    pub fn with_full_config(
        ros2_args: Ros2Args,
        domain_id: u32,
        config: zenoh::Config,
    ) -> Result<Arc<Self>> {
        // Open Zenoh session
        let session = zenoh::open(config).wait()?;

        // Get session ID (ZenohId Display provides hex format)
        let session_id = session.zid().to_string();

        // Create graph cache
        let graph_cache = GraphCache::new();

        let inner = Arc::new(ContextInner {
            session,
            domain_id,
            session_id,
            next_node_id: AtomicU32::new(0),
            graph_cache: Arc::new(Mutex::new(graph_cache)),
            ros2_args,
            _liveliness_subscriber: Mutex::new(None),
        });

        let ctx = Arc::new(Context { inner });

        // Start liveliness subscription for graph discovery
        ctx.start_graph_discovery()?;

        Ok(ctx)
    }

    /// Get the ROS domain ID.
    pub fn domain_id(&self) -> u32 {
        self.inner.domain_id
    }

    /// Get the Zenoh session ID as a hex string.
    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }

    /// Get a reference to the Zenoh session.
    pub fn session(&self) -> &Session {
        &self.inner.session
    }

    /// Get a reference to the parsed ROS2 command-line arguments.
    pub fn ros2_args(&self) -> &Ros2Args {
        &self.inner.ros2_args
    }

    /// Get the enclave from ROS2 arguments (for SROS2 security).
    pub fn enclave(&self) -> Option<&str> {
        self.inner.ros2_args.enclave.as_deref()
    }

    /// Create a new node.
    ///
    /// # Arguments
    ///
    /// * `name` - Node name (must be a valid ROS2 node name)
    /// * `namespace` - Optional namespace (must be a valid ROS2 namespace)
    ///
    /// # Errors
    ///
    /// Returns an error if the name or namespace is invalid.
    pub fn create_node(self: &Arc<Self>, name: &str, namespace: Option<&str>) -> Result<Arc<Node>> {
        // Get enclave from ROS2 args
        let enclave = self.inner.ros2_args.enclave.as_deref().unwrap_or("");

        // Allocate node ID
        let node_id = self.inner.next_node_id.fetch_add(1, Ordering::SeqCst);

        Node::new(
            Arc::clone(self),
            node_id,
            name,
            namespace.unwrap_or(""),
            enclave,
        )
    }

    /// Get a snapshot of the graph cache.
    pub fn graph_cache(&self) -> GraphCache {
        self.inner.graph_cache.lock().clone()
    }

    /// Create a new selector.
    ///
    /// The selector is used to wait on events and invoke callbacks
    /// for single-threaded execution.
    pub fn create_selector(&self) -> crate::selector::Selector {
        crate::selector::Selector::new()
    }

    /// Start graph discovery by subscribing to liveliness tokens.
    fn start_graph_discovery(&self) -> Result<()> {
        use crate::keyexpr::LIVELINESS_PREFIX;

        let key = format!("{}/**", LIVELINESS_PREFIX);
        let graph_cache = Arc::clone(&self.inner.graph_cache);

        // Subscribe to liveliness tokens
        let subscriber = self
            .inner
            .session
            .liveliness()
            .declare_subscriber(&key)
            .callback(move |sample| {
                let key_expr = sample.key_expr().as_str();
                let mut cache = graph_cache.lock();
                cache.handle_liveliness_token(key_expr, sample.kind());
            })
            .wait()?;

        // Store the subscriber to keep it alive for the lifetime of the context
        *self.inner._liveliness_subscriber.lock() = Some(subscriber);

        // Query existing liveliness tokens
        let replies = self.inner.session.liveliness().get(&key).wait()?;

        let mut cache = self.inner.graph_cache.lock();
        while let Ok(reply) = replies.recv() {
            if let Ok(sample) = reply.result() {
                cache.handle_liveliness_token(sample.key_expr().as_str(), sample.kind());
            }
        }

        Ok(())
    }
}

// ============================================================================
// RosContext trait implementation
// ============================================================================

impl oxidros_core::api::RosContext for Context {
    type Node = Node;
    type Selector = crate::selector::Selector;

    fn create_node(
        self: &Arc<Self>,
        name: &str,
        namespace: Option<&str>,
    ) -> crate::error::Result<Arc<Self::Node>> {
        Self::create_node(self, name, namespace)
    }

    fn create_selector(self: &Arc<Self>) -> crate::error::Result<Self::Selector> {
        Ok(crate::selector::Selector::new())
    }

    fn ros_domain_id(&self) -> u32 {
        self.domain_id()
    }
}
