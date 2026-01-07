//! Key expression builders for Zenoh.
//!
//! This module provides functions to build Zenoh key expressions that are
//! compatible with rmw_zenoh_cpp.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Topic and Service name mapping](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#topic-and-service-name-mapping-to-zenoh-key-expressions)

use oxidros_core::Profile;

/// Prefix for ROS2 liveliness tokens (hermetic namespace).
pub const LIVELINESS_PREFIX: &str = "@ros2_lv";

/// Entity kinds for liveliness tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// Node entity
    Node,
    /// Message publisher
    Publisher,
    /// Message subscriber
    Subscriber,
    /// Service server
    ServiceServer,
    /// Service client
    ServiceClient,
}

impl EntityKind {
    /// Returns the two-character code for this entity kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Node => "NN",
            Self::Publisher => "MP",
            Self::Subscriber => "MS",
            Self::ServiceServer => "SS",
            Self::ServiceClient => "SC",
        }
    }
}

/// Build a topic/service key expression.
///
/// Format: `<domain_id>/<fully_qualified_name>/<type_name>/<type_hash>`
///
/// # Arguments
///
/// * `domain_id` - ROS domain ID
/// * `fq_name` - Fully qualified topic/service name (e.g., "/chatter" or "/robot1/cmd_vel")
/// * `type_name` - DDS type name (e.g., "std_msgs::msg::dds_::String_")
/// * `type_hash` - RIHS01 type hash
///
/// # Example
///
/// ```ignore
/// let key = topic_keyexpr(0, "/chatter", "std_msgs::msg::dds_::String_", "RIHS01_abc...");
/// // Returns: "0/chatter/std_msgs::msg::dds_::String_/RIHS01_abc..."
/// ```
pub fn topic_keyexpr(domain_id: u32, fq_name: &str, type_name: &str, type_hash: &str) -> String {
    // Remove leading slash from fq_name for key expression
    let name = fq_name.strip_prefix('/').unwrap_or(fq_name);
    format!("{}/{}/{}/{}", domain_id, name, type_name, type_hash)
}

/// Build a liveliness token key expression for a node.
///
/// Format: `@ros2_lv/<domain_id>/<session_id>/<node_id>/<node_id>/<entity_kind>/<mangled_enclave>/<mangled_namespace>/<node_name>`
///
/// # Arguments
///
/// * `domain_id` - ROS domain ID
/// * `session_id` - Zenoh session ID (hex string)
/// * `node_id` - Node ID within the context
/// * `enclave` - SROS enclave name (empty string if not set)
/// * `namespace` - Node namespace (empty string for root namespace)
/// * `node_name` - Node name
pub fn liveliness_node_keyexpr(
    domain_id: u32,
    session_id: &str,
    node_id: u32,
    enclave: &str,
    namespace: &str,
    node_name: &str,
) -> String {
    let mangled_enclave = mangle_name(enclave);
    let mangled_namespace = mangle_name(namespace);

    format!(
        "{}/{}/{}/{}/{}/{}/{}/{}/{}",
        LIVELINESS_PREFIX,
        domain_id,
        session_id,
        node_id,
        node_id, // entity_id same as node_id for nodes
        EntityKind::Node.as_str(),
        mangled_enclave,
        mangled_namespace,
        node_name
    )
}

/// Build a liveliness token key expression for an entity (publisher, subscriber, service, client).
///
/// Format: `@ros2_lv/<domain_id>/<session_id>/<node_id>/<entity_id>/<entity_kind>/<mangled_enclave>/<mangled_namespace>/<node_name>/<mangled_qualified_name>/<type_name>/<type_hash>/<qos>`
///
/// # Arguments
///
/// * `domain_id` - ROS domain ID
/// * `session_id` - Zenoh session ID (hex string)
/// * `node_id` - Node ID within the context
/// * `entity_id` - Entity ID within the node
/// * `entity_kind` - Kind of entity (Publisher, Subscriber, etc.)
/// * `enclave` - SROS enclave name
/// * `namespace` - Node namespace
/// * `node_name` - Node name
/// * `fq_name` - Fully qualified topic/service name
/// * `type_name` - DDS type name
/// * `type_hash` - RIHS01 type hash
/// * `qos` - QoS profile
#[allow(clippy::too_many_arguments)]
pub fn liveliness_entity_keyexpr(
    domain_id: u32,
    session_id: &str,
    node_id: u32,
    entity_id: u32,
    entity_kind: EntityKind,
    enclave: &str,
    namespace: &str,
    node_name: &str,
    fq_name: &str,
    type_name: &str,
    type_hash: &str,
    qos: &Profile,
) -> String {
    let mangled_enclave = mangle_name(enclave);
    let mangled_namespace = mangle_name(namespace);
    let mangled_fq_name = mangle_name(fq_name);
    let qos_str = qos_to_keyexpr(qos);

    format!(
        "{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}/{}",
        LIVELINESS_PREFIX,
        domain_id,
        session_id,
        node_id,
        entity_id,
        entity_kind.as_str(),
        mangled_enclave,
        mangled_namespace,
        node_name,
        mangled_fq_name,
        type_name,
        type_hash,
        qos_str
    )
}

/// Mangle a name by replacing `/` with `%`.
///
/// Empty names become just `%`.
///
/// # Example
///
/// ```ignore
/// assert_eq!(mangle_name("/robot1/cmd_vel"), "%robot1%cmd_vel");
/// assert_eq!(mangle_name(""), "%");
/// ```
pub fn mangle_name(name: &str) -> String {
    if name.is_empty() {
        "%".to_string()
    } else {
        name.replace('/', "%")
    }
}

/// Unmangle a name by replacing `%` with `/`.
///
/// A single `%` becomes an empty string.
pub fn unmangle_name(mangled: &str) -> String {
    if mangled == "%" {
        String::new()
    } else {
        mangled.replace('%', "/")
    }
}

/// Encode QoS profile to a compact key expression string.
///
/// Format: `<reliability>:<depth>:<durability>:<deadline>:<lifespan>:<liveliness>`
///
/// # Reference
///
/// See `qos_to_keyexpr` function in rmw_zenoh_cpp.
pub fn qos_to_keyexpr(qos: &Profile) -> String {
    use oxidros_core::qos::{DurabilityPolicy, HistoryPolicy, ReliabilityPolicy};

    // Reliability: empty for default, number for specific
    let reliability = match qos.reliability {
        ReliabilityPolicy::SystemDefault => String::new(),
        ReliabilityPolicy::Reliable => String::new(), // Default
        ReliabilityPolicy::BestEffort => "1".to_string(),
        _ => String::new(),
    };

    // History depth
    let depth = match qos.history {
        HistoryPolicy::KeepLast => {
            if qos.depth == 0 {
                String::new()
            } else {
                qos.depth.to_string()
            }
        }
        HistoryPolicy::KeepAll => "0".to_string(), // 0 means keep all
        _ => String::new(),
    };

    // Durability
    let durability = match qos.durability {
        DurabilityPolicy::SystemDefault | DurabilityPolicy::Volatile => String::new(),
        DurabilityPolicy::TransientLocal => "1".to_string(),
        _ => String::new(),
    };

    // Deadline, lifespan, liveliness - empty for now (not fully implemented)
    let deadline = String::new();
    let lifespan = String::new();
    let liveliness = String::new();

    format!(
        "{}:,{}:,{}:,{}:,{}:,{}",
        reliability, depth, durability, deadline, lifespan, liveliness
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mangle_unmangle() {
        assert_eq!(mangle_name("/robot1/cmd_vel"), "%robot1%cmd_vel");
        assert_eq!(mangle_name(""), "%");
        assert_eq!(mangle_name("simple"), "simple");

        assert_eq!(unmangle_name("%robot1%cmd_vel"), "/robot1/cmd_vel");
        assert_eq!(unmangle_name("%"), "");
    }

    #[test]
    fn test_topic_keyexpr() {
        let key = topic_keyexpr(
            0,
            "/chatter",
            "std_msgs::msg::dds_::String_",
            "RIHS01_abc123",
        );
        assert_eq!(key, "0/chatter/std_msgs::msg::dds_::String_/RIHS01_abc123");
    }

    #[test]
    fn test_entity_kind() {
        assert_eq!(EntityKind::Node.as_str(), "NN");
        assert_eq!(EntityKind::Publisher.as_str(), "MP");
        assert_eq!(EntityKind::Subscriber.as_str(), "MS");
        assert_eq!(EntityKind::ServiceServer.as_str(), "SS");
        assert_eq!(EntityKind::ServiceClient.as_str(), "SC");
    }
}
