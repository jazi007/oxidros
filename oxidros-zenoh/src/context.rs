//! Zenoh context (session) management.
//!
//! A [`Context`] wraps a Zenoh session and provides the foundation for
//! creating ROS2 nodes and entities.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Contexts](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#contexts)

use crate::{
    error::{Error, Result},
    graph_cache::GraphCache,
    node::Node,
};
use parking_lot::Mutex;
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
    /// 1. Read `ROS_DOMAIN_ID` from environment (default: 0)
    /// 2. Read `ZENOH_SESSION_CONFIG_URI` for custom config (optional)
    /// 3. Open a Zenoh session in peer mode connecting to localhost:7447
    ///
    /// # Errors
    ///
    /// Returns an error if the Zenoh session cannot be opened.
    pub fn new() -> Result<Self> {
        // Get domain ID from environment
        let domain_id = env::var(ROS_DOMAIN_ID)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Self::with_domain_id(domain_id)
    }

    /// Create a new context with a specific domain ID.
    pub fn with_domain_id(domain_id: u32) -> Result<Self> {
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

        Self::with_config(domain_id, config)
    }

    /// Create a new context with custom Zenoh configuration.
    pub fn with_config(domain_id: u32, config: zenoh::Config) -> Result<Self> {
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
        });

        let ctx = Context { inner };

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
    pub fn create_node(&self, name: &str, namespace: Option<&str>) -> Result<Arc<Node>> {
        // Validate node name
        ros2args::names::validate_node_name(name)?;

        // Validate namespace if provided
        if let Some(ns) = namespace {
            if !ns.is_empty() {
                ros2args::names::validate_namespace(ns)?;
            }
        }

        // Allocate node ID
        let node_id = self.inner.next_node_id.fetch_add(1, Ordering::SeqCst);

        Node::new(self.clone(), node_id, name, namespace.unwrap_or(""))
    }

    /// Get a snapshot of the graph cache.
    pub fn graph_cache(&self) -> GraphCache {
        self.inner.graph_cache.lock().clone()
    }

    /// Allocate a new entity ID.
    #[allow(dead_code)]
    pub(crate) fn allocate_entity_id(&self) -> u32 {
        // For simplicity, use a global counter
        // In a more complete implementation, this would be per-node
        static ENTITY_COUNTER: AtomicU32 = AtomicU32::new(10); // Start at 10 to match rmw_zenoh
        ENTITY_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Start graph discovery by subscribing to liveliness tokens.
    fn start_graph_discovery(&self) -> Result<()> {
        use crate::keyexpr::LIVELINESS_PREFIX;

        let key = format!("{}/**", LIVELINESS_PREFIX);
        let graph_cache = Arc::clone(&self.inner.graph_cache);

        // Subscribe to liveliness tokens
        let _subscriber = self
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
