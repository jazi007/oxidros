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
use ros2_types::{TypeDescription, TypeSupport};
use ros2args::names::NameKind;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use zenoh::Wait;
use zenoh::liveliness::LivelinessToken;

/// Inner node data.
struct NodeInner {
    /// Parent context.
    context: Arc<Context>,
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
        context: Arc<Context>,
        node_id: u32,
        name: &str,
        namespace: &str,
        enclave: &str,
    ) -> Result<Arc<Self>> {
        let gid = generate_gid();

        // Create liveliness token key
        let token_key = liveliness_node_keyexpr(
            context.domain_id(),
            context.session_id(),
            node_id,
            enclave,
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
            enclave: enclave.to_string(),
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

    /// Get the node name (alias for `name()` for API compatibility with oxidros-rcl).
    pub fn get_name(&self) -> &str {
        &self.inner.name
    }

    /// Get the node namespace.
    pub fn namespace(&self) -> &str {
        &self.inner.namespace
    }

    /// Get the node namespace (alias for `namespace()` for API compatibility with oxidros-rcl).
    pub fn get_namespace(&self) -> &str {
        &self.inner.namespace
    }

    /// Get the fully qualified node name.
    pub fn fully_qualified_name(&self) -> String {
        ros2args::names::build_node_fqn(
            if self.inner.namespace.is_empty() {
                "/"
            } else {
                &self.inner.namespace
            },
            &self.inner.name,
        )
    }

    /// Get the fully qualified node name (alias for API compatibility with oxidros-rcl).
    pub fn get_fully_qualified_name(&self) -> String {
        self.fully_qualified_name()
    }

    /// Get the node GID.
    pub fn gid(&self) -> &[u8; 16] {
        &self.inner.gid
    }

    /// Get the parent context.
    pub fn context(&self) -> &Arc<Context> {
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

    /// Expand a topic/service name to its fully qualified form and apply remapping rules.
    ///
    /// This function:
    /// 1. Validates the input name
    /// 2. Expands `~` (private) and relative names to fully qualified names
    /// 3. Applies any remapping rules from command-line arguments
    ///
    /// # Arguments
    ///
    /// * `name` - The topic or service name to expand
    /// * `kind` - The kind of name (Topic or Service)
    ///
    /// # Returns
    ///
    /// The fully qualified and potentially remapped name.
    pub fn expand_and_remap_name(&self, name: &str, kind: NameKind) -> Result<String> {
        // Validate the input name
        ros2args::names::validate_topic_name(name)?;

        // Get the effective namespace (use "/" if empty)
        let namespace = if self.inner.namespace.is_empty() {
            "/"
        } else {
            &self.inner.namespace
        };

        // Expand the name (handles ~, relative, and absolute names)
        let expanded = ros2args::names::expand_topic_name(namespace, &self.inner.name, name)?;

        // Apply remapping rules
        let ros2_args = self.inner.context.ros2_args();
        let remapped = self.apply_remap_rules(&expanded, kind, ros2_args);

        Ok(remapped)
    }

    /// Apply remapping rules to a fully qualified name.
    fn apply_remap_rules(
        &self,
        fq_name: &str,
        _kind: NameKind,
        ros2_args: &ros2args::Ros2Args,
    ) -> String {
        // Get remapping rules that apply to this node
        let node_name = &self.inner.name;

        for rule in &ros2_args.remap_rules {
            // Check if rule applies to this node
            if !rule.applies_to_node(node_name) {
                continue;
            }

            // Check for exact match
            if rule.from == fq_name {
                return rule.to.clone();
            }

            // Check for relative match (rule.from without leading /)
            if !rule.from.starts_with('/') {
                // Expand the rule's from field
                let namespace = if self.inner.namespace.is_empty() {
                    "/"
                } else {
                    &self.inner.namespace
                };
                if let Ok(expanded_from) =
                    ros2args::names::expand_topic_name(namespace, node_name, &rule.from)
                {
                    if expanded_from == fq_name {
                        // Expand the rule's to field as well
                        if rule.to.starts_with('/') {
                            return rule.to.clone();
                        }
                        if let Ok(expanded_to) =
                            ros2args::names::expand_topic_name(namespace, node_name, &rule.to)
                        {
                            return expanded_to;
                        }
                        return rule.to.clone();
                    }
                }
            }
        }

        fq_name.to_string()
    }

    /// Create a publisher.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Topic name (can be absolute, relative, or private `~`)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Message type implementing `TypeSupport`
    ///
    /// # Name Resolution
    ///
    /// The topic name is expanded and remapped:
    /// - Absolute names (starting with `/`) are used as-is
    /// - Relative names are prefixed with the node's namespace
    /// - Private names (starting with `~`) are prefixed with the node's FQN
    /// - Remapping rules from command-line arguments are applied
    pub fn create_publisher<T: TypeSupport + TypeDescription>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Publisher<T>> {
        // Expand and remap the topic name
        let fq_topic_name = self.expand_and_remap_name(topic_name, NameKind::Topic)?;

        Publisher::new(
            self.clone(),
            topic_name,
            &fq_topic_name,
            qos.unwrap_or_default(),
            EntityKind::Publisher,
        )
    }

    /// Create a subscriber.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Topic name (can be absolute, relative, or private `~`)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Message type implementing `TypeSupport`
    ///
    /// # Name Resolution
    ///
    /// The topic name is expanded and remapped (see `create_publisher`).
    pub fn create_subscriber<T: TypeSupport + TypeDescription>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Subscriber<T>> {
        // Expand and remap the topic name
        let fq_topic_name = self.expand_and_remap_name(topic_name, NameKind::Topic)?;

        Subscriber::new(
            self.clone(),
            topic_name,
            &fq_topic_name,
            qos.unwrap_or_default(),
            EntityKind::Subscriber,
        )
    }

    /// Create a service client.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name (can be absolute, relative, or private `~`)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Service type implementing `ServiceMsg`
    ///
    /// # Name Resolution
    ///
    /// The service name is expanded and remapped (see `create_publisher`).
    pub fn create_client<T: oxidros_core::ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Client<T>>
    where
        T::Request: TypeSupport + TypeDescription,
        T::Response: TypeSupport + TypeDescription,
    {
        // Expand and remap the service name (services use Topic naming rules)
        let fq_service_name = self.expand_and_remap_name(service_name, NameKind::Topic)?;

        Client::new(
            self.clone(),
            service_name,
            &fq_service_name,
            qos.unwrap_or_else(Profile::services_default),
        )
    }

    /// Create a service server.
    ///
    /// # Arguments
    ///
    /// * `service_name` - Service name (can be absolute, relative, or private `~`)
    /// * `qos` - Optional QoS profile (uses default if None)
    ///
    /// # Type Parameters
    ///
    /// * `T` - Service type implementing `ServiceMsg`
    ///
    /// # Name Resolution
    ///
    /// The service name is expanded and remapped (see `create_publisher`).
    pub fn create_server<T: oxidros_core::ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Server<T>>
    where
        T::Request: TypeSupport + TypeDescription,
        T::Response: TypeSupport + TypeDescription,
    {
        // Expand and remap the service name (services use Topic naming rules)
        let fq_service_name = self.expand_and_remap_name(service_name, NameKind::Topic)?;

        Server::new(
            self.clone(),
            service_name,
            &fq_service_name,
            qos.unwrap_or_else(Profile::services_default),
        )
    }

    /// Create a parameter server for this node.
    ///
    /// The parameter server provides the standard ROS2 parameter services:
    /// - `~/list_parameters`
    /// - `~/get_parameters`
    /// - `~/set_parameters`
    /// - `~/set_parameters_atomically`
    /// - `~/describe_parameters`
    /// - `~/get_parameter_types`
    ///
    /// # Arguments
    ///
    /// # Returns
    ///
    /// A `ParameterServer` that must be spun to handle parameter requests.
    ///
    pub fn create_parameter_server(self: &Arc<Self>) -> Result<crate::parameter::ParameterServer> {
        crate::parameter::ParameterServer::new(self.clone())
    }
}
