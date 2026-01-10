//! ROS2 Node abstraction.
//!
//! A [`Node`] represents a ROS2 node and can create publishers, subscribers,
//! service clients, and service servers.

use crate::{
    attachment::generate_gid,
    context::Context,
    error::{Result, Ros2ArgsResultExt},
    keyexpr::{EntityKind, liveliness_node_keyexpr},
    service::{client::Client, server::Server},
    topic::{publisher::Publisher, subscriber::Subscriber},
};
use oxidros_core::{TypeSupport, qos::Profile};
use parking_lot::Mutex;
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

/// Compute the effective node name by applying `__node` remapping rules.
///
/// This looks for a remapping rule where `from` is `__node` and returns
/// the `to` value if found, otherwise returns the original name.
pub(crate) fn compute_effective_node_name(
    original_name: &str,
    ros2_args: &ros2args::Ros2Args,
) -> String {
    for rule in &ros2_args.remap_rules {
        // Only consider rules that apply to this node (or global rules)
        if !rule.applies_to_node(original_name) {
            continue;
        }
        if rule.from == "__node" {
            return rule.to.clone();
        }
    }
    original_name.to_string()
}

/// Compute the effective namespace by applying `__ns` remapping rules.
///
/// This looks for a remapping rule where `from` is `__ns` and returns
/// the `to` value if found, otherwise returns the original namespace.
pub(crate) fn compute_effective_namespace(
    original_name: &str,
    original_namespace: &str,
    ros2_args: &ros2args::Ros2Args,
) -> String {
    for rule in &ros2_args.remap_rules {
        // Only consider rules that apply to this node (or global rules)
        if !rule.applies_to_node(original_name) {
            continue;
        }
        if rule.from == "__ns" {
            return rule.to.clone();
        }
    }
    original_namespace.to_string()
}

impl Node {
    /// Create a new node.
    ///
    /// The `name` and `namespace` parameters are the original (requested) values.
    /// The effective (remapped) values are computed using ros2_args when needed.
    /// The liveliness token uses the effective name/namespace.
    pub(crate) fn new(
        context: Arc<Context>,
        node_id: u32,
        name: &str,
        namespace: &str,
        enclave: &str,
    ) -> Result<Arc<Self>> {
        let gid = generate_gid();
        // Compute effective name/namespace for liveliness token
        let ros2_args = context.ros2_args();
        let effective_name = compute_effective_node_name(name, ros2_args);
        let effective_namespace = compute_effective_namespace(name, namespace, ros2_args);
        // Validate node name
        ros2args::names::validate_node_name(&effective_name).map_name_err()?;
        // Validate namespace if non-empty
        if !effective_namespace.is_empty() {
            ros2args::names::validate_namespace(&effective_namespace).map_name_err()?;
        }
        // Create liveliness token key using effective name/namespace
        let token_key = liveliness_node_keyexpr(
            context.domain_id(),
            context.session_id(),
            node_id,
            enclave,
            &effective_namespace,
            &effective_name,
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

    /// Get the effective node name (after applying `__node` remapping).
    pub fn name(&self) -> String {
        compute_effective_node_name(&self.inner.name, self.inner.context.ros2_args())
    }

    /// Get the effective node namespace (after applying `__ns` remapping).
    pub fn namespace(&self) -> String {
        compute_effective_namespace(
            &self.inner.name,
            &self.inner.namespace,
            self.inner.context.ros2_args(),
        )
    }

    /// Get the fully qualified node name (using effective name/namespace).
    pub fn fully_qualified_name(&self) -> String {
        let effective_ns = self.namespace();
        let effective_name = self.name();
        ros2args::names::build_node_fqn(
            if effective_ns.is_empty() {
                "/"
            } else {
                &effective_ns
            },
            &effective_name,
        )
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

    /// Get the original (pre-remapping) node name.
    ///
    /// This is used internally for matching node-specific rules (params, topics).
    pub(crate) fn original_name(&self) -> &str {
        &self.inner.name
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
        ros2args::names::validate_topic_name(name).map_name_err()?;

        // Get the effective namespace and name for expansion
        // (topic names should expand using the remapped node identity)
        let effective_ns = self.namespace();
        let effective_name = self.name();
        let namespace = if effective_ns.is_empty() {
            "/"
        } else {
            &effective_ns
        };

        // Expand the name (handles ~, relative, and absolute names)
        let expanded =
            ros2args::names::expand_topic_name(namespace, &effective_name, name).map_name_err()?;

        // Apply remapping rules
        let ros2_args = self.inner.context.ros2_args();
        let remapped =
            self.apply_remap_rules(&expanded, &effective_ns, &effective_name, kind, ros2_args);

        Ok(remapped)
    }

    /// Apply remapping rules to a fully qualified name.
    ///
    /// Uses the original node name for matching node-specific rules,
    /// but the effective namespace/name for expanding relative rules.
    fn apply_remap_rules(
        &self,
        fq_name: &str,
        effective_ns: &str,
        effective_name: &str,
        _kind: NameKind,
        ros2_args: &ros2args::Ros2Args,
    ) -> String {
        // Use original node name for matching node-specific rules
        let original_node_name = &self.inner.name;

        for rule in &ros2_args.remap_rules {
            // Check if rule applies to this node (using original name)
            if !rule.applies_to_node(original_node_name) {
                continue;
            }

            // Check for exact match
            if rule.from == fq_name {
                return rule.to.clone();
            }

            // Check for relative match (rule.from without leading /)
            if !rule.from.starts_with('/') {
                // Expand the rule's from field using effective namespace/name
                let namespace = if effective_ns.is_empty() {
                    "/"
                } else {
                    effective_ns
                };
                if let Ok(expanded_from) =
                    ros2args::names::expand_topic_name(namespace, effective_name, &rule.from)
                    && expanded_from == fq_name
                {
                    // Expand the rule's to field as well
                    if rule.to.starts_with('/') {
                        return rule.to.clone();
                    }
                    if let Ok(expanded_to) =
                        ros2args::names::expand_topic_name(namespace, effective_name, &rule.to)
                    {
                        return expanded_to;
                    }
                    return rule.to.clone();
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
    pub fn create_publisher<T: TypeSupport>(
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
    pub fn create_subscriber<T: TypeSupport>(
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
        T::Request: TypeSupport,
        T::Response: TypeSupport,
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
        T::Request: TypeSupport,
        T::Response: TypeSupport,
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

// ============================================================================
// RosNode trait implementation
// ============================================================================

impl oxidros_core::api::RosNode for Node {
    type Publisher<T: TypeSupport> = Publisher<T>;
    type Subscriber<T: TypeSupport> = Subscriber<T>;
    type Client<T: oxidros_core::ServiceMsg> = Client<T>;
    type Server<T: oxidros_core::ServiceMsg> = Server<T>;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Owned(Node::name(self))
    }

    fn namespace(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Owned(Node::namespace(self))
    }

    fn fully_qualified_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Owned(Node::fully_qualified_name(self))
    }

    fn new_publisher<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Publisher<T>> {
        self.create_publisher(topic_name, qos)
    }

    fn new_subscriber<T: TypeSupport>(
        self: &Arc<Self>,
        topic_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Subscriber<T>> {
        self.create_subscriber(topic_name, qos)
    }

    fn new_client<T: oxidros_core::ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Client<T>> {
        self.create_client(service_name, qos)
    }

    fn new_server<T: oxidros_core::ServiceMsg>(
        self: &Arc<Self>,
        service_name: &str,
        qos: Option<Profile>,
    ) -> Result<Self::Server<T>> {
        self.create_server(service_name, qos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros2args::{RemapRule, Ros2Args};

    /// Helper to create Ros2Args with remap rules
    fn args_with_remaps(rules: Vec<RemapRule>) -> Ros2Args {
        Ros2Args {
            remap_rules: rules,
            ..Default::default()
        }
    }

    // ========================================================================
    // Tests for compute_effective_node_name
    // ========================================================================

    #[test]
    fn test_node_name_no_remapping() {
        let args = Ros2Args::default();
        let result = compute_effective_node_name("my_node", &args);
        assert_eq!(result, "my_node");
    }

    #[test]
    fn test_node_name_global_remapping() {
        let args = args_with_remaps(vec![RemapRule::new_global(
            "__node".to_string(),
            "renamed_node".to_string(),
        )]);
        let result = compute_effective_node_name("my_node", &args);
        assert_eq!(result, "renamed_node");
    }

    #[test]
    fn test_node_name_node_specific_remapping_matches() {
        let args = args_with_remaps(vec![RemapRule::new_node_specific(
            "my_node".to_string(),
            "__node".to_string(),
            "renamed_node".to_string(),
        )]);
        let result = compute_effective_node_name("my_node", &args);
        assert_eq!(result, "renamed_node");
    }

    #[test]
    fn test_node_name_node_specific_remapping_no_match() {
        let args = args_with_remaps(vec![RemapRule::new_node_specific(
            "other_node".to_string(),
            "__node".to_string(),
            "renamed_node".to_string(),
        )]);
        let result = compute_effective_node_name("my_node", &args);
        assert_eq!(result, "my_node"); // No change, rule doesn't apply
    }

    #[test]
    fn test_node_name_ignores_other_rules() {
        let args = args_with_remaps(vec![
            RemapRule::new_global("topic_a".to_string(), "topic_b".to_string()),
            RemapRule::new_global("__ns".to_string(), "/new_ns".to_string()),
        ]);
        let result = compute_effective_node_name("my_node", &args);
        assert_eq!(result, "my_node"); // Only __node rules should apply
    }

    // ========================================================================
    // Tests for compute_effective_namespace
    // ========================================================================

    #[test]
    fn test_namespace_no_remapping() {
        let args = Ros2Args::default();
        let result = compute_effective_namespace("my_node", "/original_ns", &args);
        assert_eq!(result, "/original_ns");
    }

    #[test]
    fn test_namespace_global_remapping() {
        let args = args_with_remaps(vec![RemapRule::new_global(
            "__ns".to_string(),
            "/new_namespace".to_string(),
        )]);
        let result = compute_effective_namespace("my_node", "/original_ns", &args);
        assert_eq!(result, "/new_namespace");
    }

    #[test]
    fn test_namespace_node_specific_remapping_matches() {
        let args = args_with_remaps(vec![RemapRule::new_node_specific(
            "my_node".to_string(),
            "__ns".to_string(),
            "/node_specific_ns".to_string(),
        )]);
        let result = compute_effective_namespace("my_node", "/original_ns", &args);
        assert_eq!(result, "/node_specific_ns");
    }

    #[test]
    fn test_namespace_node_specific_remapping_no_match() {
        let args = args_with_remaps(vec![RemapRule::new_node_specific(
            "other_node".to_string(),
            "__ns".to_string(),
            "/other_ns".to_string(),
        )]);
        let result = compute_effective_namespace("my_node", "/original_ns", &args);
        assert_eq!(result, "/original_ns"); // No change
    }

    #[test]
    fn test_namespace_empty_original() {
        let args = args_with_remaps(vec![RemapRule::new_global(
            "__ns".to_string(),
            "/new_ns".to_string(),
        )]);
        let result = compute_effective_namespace("my_node", "", &args);
        assert_eq!(result, "/new_ns");
    }

    // ========================================================================
    // Tests for combined node + namespace remapping
    // ========================================================================

    #[test]
    fn test_both_node_and_namespace_remapping() {
        let args = args_with_remaps(vec![
            RemapRule::new_global("__node".to_string(), "new_node".to_string()),
            RemapRule::new_global("__ns".to_string(), "/new_ns".to_string()),
        ]);

        let effective_name = compute_effective_node_name("original_node", &args);
        let effective_ns = compute_effective_namespace("original_node", "/original_ns", &args);

        assert_eq!(effective_name, "new_node");
        assert_eq!(effective_ns, "/new_ns");
    }

    #[test]
    fn test_first_matching_rule_wins() {
        let args = args_with_remaps(vec![
            RemapRule::new_global("__node".to_string(), "first_name".to_string()),
            RemapRule::new_global("__node".to_string(), "second_name".to_string()),
        ]);

        let result = compute_effective_node_name("my_node", &args);
        assert_eq!(result, "first_name"); // First rule should win
    }

    #[test]
    fn test_node_specific_rule_priority() {
        // Global rule comes first, but node-specific should still apply
        // when node name matches
        let args = args_with_remaps(vec![
            RemapRule::new_global("__node".to_string(), "global_name".to_string()),
            RemapRule::new_node_specific(
                "my_node".to_string(),
                "__node".to_string(),
                "specific_name".to_string(),
            ),
        ]);

        // Since global comes first and applies to all nodes, it wins
        let result = compute_effective_node_name("my_node", &args);
        assert_eq!(result, "global_name");
    }
}
