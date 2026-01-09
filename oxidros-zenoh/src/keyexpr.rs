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
/// Format: `<Reliability>:<Durability>:<History>,<Depth>:<DeadlineSec>,<DeadlineNSec>:<LifespanSec>,<LifespanNSec>:<Liveliness>,<LivelinessSec>,<LivelinessNSec>`
///
/// Where:
/// - `:` is the QOS_DELIMITER separating major QoS components
/// - `,` is the QOS_COMPONENT_DELIMITER separating sub-components
/// - Values are only included if they differ from the default QoS profile
///
/// RMW default QoS values (from rmw_zenoh_cpp):
/// - History: KeepLast (1)
/// - Depth: 42
/// - Reliability: Reliable (1)
/// - Durability: Volatile (2)
/// - Deadline: infinite (max u64 seconds)
/// - Lifespan: infinite (max u64 seconds)
/// - Liveliness: Automatic (1)
/// - Liveliness lease duration: infinite (max u64 seconds)
///
/// # Reference
///
/// See `qos_to_keyexpr` function in rmw_zenoh_cpp liveliness_utils.cpp
pub fn qos_to_keyexpr(qos: &Profile) -> String {
    use oxidros_core::qos::{DurabilityPolicy, HistoryPolicy, LivelinessPolicy, ReliabilityPolicy};

    // Default QoS values as defined in rmw_zenoh_cpp/src/detail/qos.cpp
    const DEFAULT_RELIABILITY: u8 = 1; // RMW_QOS_POLICY_RELIABILITY_RELIABLE
    const DEFAULT_DURABILITY: u8 = 2; // RMW_QOS_POLICY_DURABILITY_VOLATILE
    const DEFAULT_HISTORY: u8 = 1; // RMW_QOS_POLICY_HISTORY_KEEP_LAST
    const DEFAULT_DEPTH: usize = 42;
    const DEFAULT_LIVELINESS: u8 = 1; // RMW_QOS_POLICY_LIVELINESS_AUTOMATIC
    // Default deadline, lifespan, liveliness_lease_duration are all "infinite"
    // which is represented as max u64 values. We treat Duration::ZERO as unset/default.

    let mut keyexpr = String::new();

    // Reliability (enum values: 0=SystemDefault, 1=Reliable, 2=BestEffort, 3=Unknown, 4=BestAvailable)
    let reliability_val = match qos.reliability {
        ReliabilityPolicy::SystemDefault => 0u8,
        ReliabilityPolicy::Reliable => 1,
        ReliabilityPolicy::BestEffort => 2,
        ReliabilityPolicy::Unknown => 3,
        ReliabilityPolicy::BestAvailable => 4,
    };
    if reliability_val != DEFAULT_RELIABILITY {
        keyexpr.push_str(&reliability_val.to_string());
    }
    keyexpr.push(':');

    // Durability (enum values: 0=SystemDefault, 1=TransientLocal, 2=Volatile, 3=Unknown, 4=BestAvailable)
    let durability_val = match qos.durability {
        DurabilityPolicy::SystemDefault => 0u8,
        DurabilityPolicy::TransientLocal => 1,
        DurabilityPolicy::Volatile => 2,
        DurabilityPolicy::Unknown => 3,
        DurabilityPolicy::BestAvailable => 4,
    };
    if durability_val != DEFAULT_DURABILITY {
        keyexpr.push_str(&durability_val.to_string());
    }
    keyexpr.push(':');

    // History (enum values: 0=SystemDefault, 1=KeepLast, 2=KeepAll, 3=Unknown)
    let history_val = match qos.history {
        HistoryPolicy::SystemDefault => 0u8,
        HistoryPolicy::KeepLast => 1,
        HistoryPolicy::KeepAll => 2,
        HistoryPolicy::Unknown => 3,
    };
    if history_val != DEFAULT_HISTORY {
        keyexpr.push_str(&history_val.to_string());
    }
    keyexpr.push(',');

    // Depth
    if qos.depth != DEFAULT_DEPTH {
        keyexpr.push_str(&qos.depth.to_string());
    }
    keyexpr.push(':');

    // Deadline (sec, nsec) - Duration::ZERO means use default (infinite)
    if !qos.deadline.is_zero() {
        keyexpr.push_str(&qos.deadline.as_secs().to_string());
    }
    keyexpr.push(',');
    if !qos.deadline.is_zero() {
        keyexpr.push_str(&qos.deadline.subsec_nanos().to_string());
    }
    keyexpr.push(':');

    // Lifespan (sec, nsec) - Duration::ZERO means use default (infinite)
    if !qos.lifespan.is_zero() {
        keyexpr.push_str(&qos.lifespan.as_secs().to_string());
    }
    keyexpr.push(',');
    if !qos.lifespan.is_zero() {
        keyexpr.push_str(&qos.lifespan.subsec_nanos().to_string());
    }
    keyexpr.push(':');

    // Liveliness (enum values: 0=SystemDefault, 1=Automatic, 2=ManualByTopic, 3=Unknown, 4=BestAvailable)
    let liveliness_val = match qos.liveliness {
        LivelinessPolicy::SystemDefault => 0u8,
        LivelinessPolicy::Automatic => 1,
        LivelinessPolicy::ManualByTopic => 2,
        LivelinessPolicy::Unknown => 3,
        LivelinessPolicy::BestAvailable => 4,
    };
    if liveliness_val != DEFAULT_LIVELINESS {
        keyexpr.push_str(&liveliness_val.to_string());
    }
    keyexpr.push(',');

    // Liveliness lease duration (sec, nsec) - Duration::ZERO means use default (infinite)
    if !qos.liveliness_lease_duration.is_zero() {
        keyexpr.push_str(&qos.liveliness_lease_duration.as_secs().to_string());
    }
    keyexpr.push(',');
    if !qos.liveliness_lease_duration.is_zero() {
        keyexpr.push_str(&qos.liveliness_lease_duration.subsec_nanos().to_string());
    }

    keyexpr
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxidros_core::qos::{DurabilityPolicy, HistoryPolicy, ReliabilityPolicy};

    // =========================================================================
    // Mangle/Unmangle Tests
    // =========================================================================

    #[test]
    fn test_mangle_unmangle() {
        assert_eq!(mangle_name("/robot1/cmd_vel"), "%robot1%cmd_vel");
        assert_eq!(mangle_name(""), "%");
        assert_eq!(mangle_name("simple"), "simple");

        assert_eq!(unmangle_name("%robot1%cmd_vel"), "/robot1/cmd_vel");
        assert_eq!(unmangle_name("%"), "");
    }

    #[test]
    fn test_mangle_namespace_with_leading_slash() {
        // From design doc: namespace "/robot1" becomes "%robot1"
        assert_eq!(mangle_name("/robot1"), "%robot1");
    }

    #[test]
    fn test_mangle_topic_with_namespace() {
        // /robot1/chatter -> %robot1%chatter
        assert_eq!(mangle_name("/robot1/chatter"), "%robot1%chatter");
    }

    // =========================================================================
    // Topic Key Expression Tests
    // Based on: https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#topic-and-service-name-mapping-to-zenoh-key-expressions
    // =========================================================================

    #[test]
    fn test_topic_keyexpr_chatter() {
        // Example from design doc:
        // `chatter` topic (with `ROS_DOMAIN_ID=0` by default):
        // 0/chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18
        let key = topic_keyexpr(
            0,
            "/chatter",
            "std_msgs::msg::dds_::String_",
            "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18",
        );
        assert_eq!(
            key,
            "0/chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18"
        );
    }

    #[test]
    fn test_topic_keyexpr_with_namespace() {
        // Example from design doc:
        // `chatter` topic, using `/robot1` as a namespace:
        // 0/robot1/chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18
        let key = topic_keyexpr(
            0,
            "/robot1/chatter",
            "std_msgs::msg::dds_::String_",
            "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18",
        );
        assert_eq!(
            key,
            "0/robot1/chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18"
        );
    }

    #[test]
    fn test_service_keyexpr_add_two_ints() {
        // Example from design doc:
        // `add_two_ints` service with `DOMAIN_ID=2`:
        // 2/add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a
        let key = topic_keyexpr(
            2,
            "/add_two_ints",
            "example_interfaces::srv::dds_::AddTwoInts_",
            "RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a",
        );
        assert_eq!(
            key,
            "2/add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a"
        );
    }

    #[test]
    fn test_topic_keyexpr_different_domain() {
        let key = topic_keyexpr(
            42,
            "/my_topic",
            "my_pkg::msg::dds_::MyMsg_",
            "RIHS01_abc123",
        );
        assert_eq!(key, "42/my_topic/my_pkg::msg::dds_::MyMsg_/RIHS01_abc123");
    }

    // =========================================================================
    // Entity Kind Tests
    // =========================================================================

    #[test]
    fn test_entity_kind() {
        assert_eq!(EntityKind::Node.as_str(), "NN");
        assert_eq!(EntityKind::Publisher.as_str(), "MP");
        assert_eq!(EntityKind::Subscriber.as_str(), "MS");
        assert_eq!(EntityKind::ServiceServer.as_str(), "SS");
        assert_eq!(EntityKind::ServiceClient.as_str(), "SC");
    }

    // =========================================================================
    // Liveliness Token Tests - Nodes
    // Based on: https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#graph-cache
    // =========================================================================

    #[test]
    fn test_liveliness_node_listener() {
        // Example from design doc:
        // `listener` node (with `ROS_DOMAIN_ID=0` by default):
        // @ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/0/NN/%/%/listener
        // Note: The doc example uses domain_id=2, session_id=aac3178e146ba6f1fc6e6a4085e77f21
        let key = liveliness_node_keyexpr(
            2,
            "aac3178e146ba6f1fc6e6a4085e77f21",
            0,
            "",        // no enclave
            "",        // no namespace
            "listener",
        );
        assert_eq!(
            key,
            "@ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/0/NN/%/%/listener"
        );
    }

    #[test]
    fn test_liveliness_node_with_namespace() {
        let key = liveliness_node_keyexpr(
            0,
            "abcd1234",
            1,
            "",
            "/robot1",
            "my_node",
        );
        assert_eq!(
            key,
            "@ros2_lv/0/abcd1234/1/1/NN/%/%robot1/my_node"
        );
    }

    #[test]
    fn test_liveliness_node_with_enclave() {
        let key = liveliness_node_keyexpr(
            0,
            "session123",
            0,
            "/my_enclave",
            "/ns",
            "secure_node",
        );
        assert_eq!(
            key,
            "@ros2_lv/0/session123/0/0/NN/%my_enclave/%ns/secure_node"
        );
    }

    // =========================================================================
    // Liveliness Token Tests - Publishers/Subscribers
    // =========================================================================

    #[test]
    fn test_liveliness_subscriber_chatter() {
        // Example from design doc:
        // `listener` node's subscription on `chatter` topic:
        // @ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/10/MS/%/%/listener/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/<qos>
        // QoS format: <Reliability>:<Durability>:<History>,<Depth>:<DeadlineSec>,<DeadlineNSec>:<LifespanSec>,<LifespanNSec>:<Liveliness>,<LivelinessSec>,<LivelinessNSec>
        // With depth=10 (differs from rmw_zenoh default of 42), liveliness=SystemDefault(0) differs from default Automatic(1)
        let qos = Profile {
            depth: 10,
            history: HistoryPolicy::KeepLast,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            ..Default::default()
        };

        let key = liveliness_entity_keyexpr(
            2,
            "aac3178e146ba6f1fc6e6a4085e77f21",
            0,
            10,
            EntityKind::Subscriber,
            "",
            "",
            "listener",
            "/chatter",
            "std_msgs::msg::dds_::String_",
            "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18",
            &qos,
        );

        // QoS: reliable(1=default), volatile(2=default), keep_last(1=default), depth=10(!=42), liveliness=system_default(0!=1)
        // Format: ::,10:,:,:0,,
        assert_eq!(
            key,
            "@ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/10/MS/%/%/listener/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/::,10:,:,:0,,"
        );
    }

    #[test]
    fn test_liveliness_publisher_chatter() {
        // Example from design doc:
        // `talker` node's publisher on `chatter` topic:
        // @ros2_lv/2/8b20917502ee955ac4476e0266340d5c/0/10/MP/%/%/talker/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/<qos>
        let qos = Profile {
            depth: 7,
            history: HistoryPolicy::KeepLast,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            ..Default::default()
        };

        let key = liveliness_entity_keyexpr(
            2,
            "8b20917502ee955ac4476e0266340d5c",
            0,
            10,
            EntityKind::Publisher,
            "",
            "",
            "talker",
            "/chatter",
            "std_msgs::msg::dds_::String_",
            "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18",
            &qos,
        );

        // QoS: reliable(1=default), volatile(2=default), keep_last(1=default), depth=7(!=42), liveliness=system_default(0!=1)
        // Format: ::,7:,:,:0,,
        assert_eq!(
            key,
            "@ros2_lv/2/8b20917502ee955ac4476e0266340d5c/0/10/MP/%/%/talker/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/::,7:,:,:0,,"
        );
    }

    // =========================================================================
    // Liveliness Token Tests - Services
    // =========================================================================

    #[test]
    fn test_liveliness_service_server() {
        // Example from design doc:
        // `add_two_ints_server` node's service server:
        // @ros2_lv/2/f9980ee0495eaafb3e38f0d19e2eae12/0/10/SS/%/%/add_two_ints_server/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/<qos>
        let qos = Profile {
            depth: 10,
            history: HistoryPolicy::KeepLast,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            ..Default::default()
        };

        let key = liveliness_entity_keyexpr(
            2,
            "f9980ee0495eaafb3e38f0d19e2eae12",
            0,
            10,
            EntityKind::ServiceServer,
            "",
            "",
            "add_two_ints_server",
            "/add_two_ints",
            "example_interfaces::srv::dds_::AddTwoInts_",
            "RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a",
            &qos,
        );

        // QoS: reliable(1=default), volatile(2=default), keep_last(1=default), depth=10(!=42), liveliness=system_default(0!=1)
        // Format: ::,10:,:,:0,,
        assert_eq!(
            key,
            "@ros2_lv/2/f9980ee0495eaafb3e38f0d19e2eae12/0/10/SS/%/%/add_two_ints_server/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/::,10:,:,:0,,"
        );
    }

    #[test]
    fn test_liveliness_service_client() {
        // Example from design doc:
        // `add_two_ints_client` node's service client:
        // @ros2_lv/2/e1dc8d1b45ae8717fce78689cc655685/0/10/SC/%/%/add_two_ints_client/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/::,10:,:,:,,
        let qos = Profile {
            depth: 10,
            history: HistoryPolicy::KeepLast,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            ..Default::default()
        };

        let key = liveliness_entity_keyexpr(
            2,
            "e1dc8d1b45ae8717fce78689cc655685",
            0,
            10,
            EntityKind::ServiceClient,
            "",
            "",
            "add_two_ints_client",
            "/add_two_ints",
            "example_interfaces::srv::dds_::AddTwoInts_",
            "RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a",
            &qos,
        );

        // QoS: reliable(1=default), volatile(2=default), keep_last(1=default), depth=10(!=42), liveliness=system_default(0!=1)
        // Format: ::,10:,:,:0,,
        assert_eq!(
            key,
            "@ros2_lv/2/e1dc8d1b45ae8717fce78689cc655685/0/10/SC/%/%/add_two_ints_client/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/::,10:,:,:0,,"
        );
    }

    // =========================================================================
    // QoS to Keyexpr Tests
    // Format: <Reliability>:<Durability>:<History>,<Depth>:<DeadlineSec>,<DeadlineNSec>:<LifespanSec>,<LifespanNSec>:<Liveliness>,<LivelinessSec>,<LivelinessNSec>
    // RMW defaults: reliability=1(Reliable), durability=2(Volatile), history=1(KeepLast), depth=42, liveliness=1(Automatic)
    // =========================================================================

    #[test]
    fn test_qos_to_keyexpr_default() {
        let qos = Profile::default();
        let keyexpr = qos_to_keyexpr(&qos);
        // Profile::default() has: reliable(1), volatile(2), keep_last(1), depth=10, liveliness=system_default(0)
        // Differs from RMW defaults: depth(10!=42), liveliness(0!=1)
        // Format: ::,10:,:,:0,,
        assert_eq!(keyexpr, "::,10:,:,:0,,");
    }

    #[test]
    fn test_qos_to_keyexpr_rmw_defaults() {
        // Test with rmw_zenoh default values - should produce all empty fields
        use oxidros_core::qos::LivelinessPolicy;
        let qos = Profile {
            history: HistoryPolicy::KeepLast,
            depth: 42,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            liveliness: LivelinessPolicy::Automatic,
            ..Default::default()
        };
        let keyexpr = qos_to_keyexpr(&qos);
        // All values match RMW defaults, so all fields are empty
        assert_eq!(keyexpr, "::,:,:,:,,");
    }

    #[test]
    fn test_qos_to_keyexpr_with_depth() {
        let qos = Profile {
            depth: 10,
            history: HistoryPolicy::KeepLast,
            ..Default::default()
        };
        let keyexpr = qos_to_keyexpr(&qos);
        // depth=10 differs from default 42, liveliness=system_default(0) differs from automatic(1)
        assert_eq!(keyexpr, "::,10:,:,:0,,");
    }

    #[test]
    fn test_qos_to_keyexpr_best_effort() {
        let qos = Profile {
            reliability: ReliabilityPolicy::BestEffort,
            ..Default::default()
        };
        let keyexpr = qos_to_keyexpr(&qos);
        // reliability=BestEffort(2) differs from Reliable(1), depth=10, liveliness=0
        assert_eq!(keyexpr, "2::,10:,:,:0,,");
    }

    #[test]
    fn test_qos_to_keyexpr_keep_all() {
        let qos = Profile {
            history: HistoryPolicy::KeepAll,
            ..Default::default()
        };
        let keyexpr = qos_to_keyexpr(&qos);
        // history=KeepAll(2) differs from KeepLast(1), depth=10, liveliness=0
        assert_eq!(keyexpr, "::2,10:,:,:0,,");
    }

    #[test]
    fn test_qos_to_keyexpr_transient_local() {
        use oxidros_core::qos::LivelinessPolicy;
        let qos = Profile {
            durability: DurabilityPolicy::TransientLocal,
            depth: 42,
            liveliness: LivelinessPolicy::Automatic,
            ..Default::default()
        };
        let keyexpr = qos_to_keyexpr(&qos);
        // durability=TransientLocal(1) differs from Volatile(2)
        assert_eq!(keyexpr, ":1:,:,:,:,,");
    }

    #[test]
    fn test_qos_to_keyexpr_with_deadline() {
        use std::time::Duration;
        use oxidros_core::qos::LivelinessPolicy;
        let qos = Profile {
            deadline: Duration::new(5, 123456789),
            depth: 42,
            liveliness: LivelinessPolicy::Automatic,
            ..Default::default()
        };
        let keyexpr = qos_to_keyexpr(&qos);
        // deadline=5s,123456789ns differs from default (infinite/zero)
        assert_eq!(keyexpr, "::,:5,123456789:,:,,");
    }

    // =========================================================================
    // Integration Tests - Full Key Expression Parsing
    // =========================================================================

    #[test]
    fn test_parse_topic_keyexpr_parts() {
        let key = topic_keyexpr(
            0,
            "/chatter",
            "std_msgs::msg::dds_::String_",
            "RIHS01_abc123",
        );
        let parts: Vec<&str> = key.split('/').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "0"); // domain_id
        assert_eq!(parts[1], "chatter"); // topic name (no leading slash)
        assert_eq!(parts[2], "std_msgs::msg::dds_::String_"); // type_name
        assert_eq!(parts[3], "RIHS01_abc123"); // type_hash
    }

    #[test]
    fn test_parse_liveliness_node_keyexpr_parts() {
        let key = liveliness_node_keyexpr(0, "session123", 5, "", "/ns", "my_node");
        let parts: Vec<&str> = key.split('/').collect();
        assert_eq!(parts[0], "@ros2_lv");
        assert_eq!(parts[1], "0"); // domain_id
        assert_eq!(parts[2], "session123"); // session_id
        assert_eq!(parts[3], "5"); // node_id
        assert_eq!(parts[4], "5"); // entity_id (same as node_id for nodes)
        assert_eq!(parts[5], "NN"); // entity_kind
        assert_eq!(parts[6], "%"); // mangled enclave (empty)
        assert_eq!(parts[7], "%ns"); // mangled namespace
        assert_eq!(parts[8], "my_node"); // node_name
    }

    #[test]
    fn test_wildcard_subscription() {
        // Subscribers use "*" as wildcard for type_hash to match any compatible publisher
        let key = topic_keyexpr(
            0,
            "/chatter",
            "std_msgs::msg::dds_::String_",
            "*", // wildcard
        );
        assert_eq!(key, "0/chatter/std_msgs::msg::dds_::String_/*");
    }
}
