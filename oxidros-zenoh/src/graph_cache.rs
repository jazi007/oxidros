//! Graph cache for entity discovery.
//!
//! Tracks nodes, publishers, subscribers, services, and clients in the ROS2 graph
//! using Zenoh liveliness tokens.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Graph Cache](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#graph-cache)

use crate::keyexpr::{EntityKind, LIVELINESS_PREFIX, unmangle_name};
use std::collections::HashMap;
use zenoh::sample::SampleKind;

/// Information about a discovered entity.
#[derive(Debug, Clone)]
pub struct EntityInfo {
    /// Domain ID
    pub domain_id: u32,
    /// Session ID (hex)
    pub session_id: String,
    /// Node ID
    pub node_id: u32,
    /// Entity ID
    pub entity_id: u32,
    /// Entity kind
    pub kind: EntityKind,
    /// SROS enclave
    pub enclave: String,
    /// Node namespace
    pub namespace: String,
    /// Node name
    pub node_name: String,
    /// Topic/service name (empty for nodes)
    pub topic_name: Option<String>,
    /// Type name (empty for nodes)
    pub type_name: Option<String>,
    /// Type hash (empty for nodes)
    pub type_hash: Option<String>,
}

/// Graph cache storing discovered entities.
#[derive(Debug, Clone, Default)]
pub struct GraphCache {
    /// All discovered entities, keyed by liveliness token.
    entities: HashMap<String, EntityInfo>,
}

impl GraphCache {
    /// Create a new empty graph cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle a liveliness token event.
    pub fn handle_liveliness_token(&mut self, key_expr: &str, kind: SampleKind) {
        match kind {
            SampleKind::Put => {
                if let Some(info) = Self::parse_liveliness_token(key_expr) {
                    self.entities.insert(key_expr.to_string(), info);
                }
            }
            SampleKind::Delete => {
                self.entities.remove(key_expr);
            }
        }
    }

    /// Parse a liveliness token key expression.
    ///
    /// Format for nodes:
    /// `@ros2_lv/<domain_id>/<session_id>/<node_id>/<node_id>/<entity_kind>/<enclave>/<namespace>/<node_name>`
    ///
    /// Format for entities:
    /// `@ros2_lv/<domain_id>/<session_id>/<node_id>/<entity_id>/<entity_kind>/<enclave>/<namespace>/<node_name>/<topic>/<type>/<hash>/<qos>`
    fn parse_liveliness_token(key_expr: &str) -> Option<EntityInfo> {
        let parts: Vec<&str> = key_expr.split('/').collect();

        // Minimum parts for a node: @ros2_lv + domain + session + node_id + entity_id + kind + enclave + ns + name
        if parts.len() < 9 {
            return None;
        }

        // Verify prefix
        if parts[0] != LIVELINESS_PREFIX {
            return None;
        }

        let domain_id: u32 = parts[1].parse().ok()?;
        let session_id = parts[2].to_string();
        let node_id: u32 = parts[3].parse().ok()?;
        let entity_id: u32 = parts[4].parse().ok()?;
        let kind = match parts[5] {
            "NN" => EntityKind::Node,
            "MP" => EntityKind::Publisher,
            "MS" => EntityKind::Subscriber,
            "SS" => EntityKind::ServiceServer,
            "SC" => EntityKind::ServiceClient,
            _ => return None,
        };
        let enclave = unmangle_name(parts[6]);
        let namespace = unmangle_name(parts[7]);
        let node_name = parts[8].to_string();

        // For non-node entities, extract topic/type/hash
        let (topic_name, type_name, type_hash) = if parts.len() >= 12 && kind != EntityKind::Node {
            (
                Some(unmangle_name(parts[9])),
                Some(parts[10].to_string()),
                Some(parts[11].to_string()),
            )
        } else {
            (None, None, None)
        };

        Some(EntityInfo {
            domain_id,
            session_id,
            node_id,
            entity_id,
            kind,
            enclave,
            namespace,
            node_name,
            topic_name,
            type_name,
            type_hash,
        })
    }

    /// Get all node names.
    pub fn get_node_names(&self) -> Vec<String> {
        self.entities
            .values()
            .filter(|e| e.kind == EntityKind::Node)
            .map(|e| {
                if e.namespace.is_empty() {
                    format!("/{}", e.node_name)
                } else {
                    format!("{}/{}", e.namespace, e.node_name)
                }
            })
            .collect()
    }

    /// Count publishers for a topic.
    pub fn count_publishers(&self, topic: &str) -> usize {
        self.entities
            .values()
            .filter(|e| e.kind == EntityKind::Publisher && e.topic_name.as_deref() == Some(topic))
            .count()
    }

    /// Count subscribers for a topic.
    pub fn count_subscribers(&self, topic: &str) -> usize {
        self.entities
            .values()
            .filter(|e| e.kind == EntityKind::Subscriber && e.topic_name.as_deref() == Some(topic))
            .count()
    }

    /// Get publishers info for a topic.
    pub fn get_publishers_info(&self, topic: &str) -> Vec<&EntityInfo> {
        self.entities
            .values()
            .filter(|e| e.kind == EntityKind::Publisher && e.topic_name.as_deref() == Some(topic))
            .collect()
    }

    /// Get subscribers info for a topic.
    pub fn get_subscribers_info(&self, topic: &str) -> Vec<&EntityInfo> {
        self.entities
            .values()
            .filter(|e| e.kind == EntityKind::Subscriber && e.topic_name.as_deref() == Some(topic))
            .collect()
    }

    /// Check if a service is available.
    pub fn is_service_available(&self, service_name: &str) -> bool {
        self.entities.values().any(|e| {
            e.kind == EntityKind::ServiceServer && e.topic_name.as_deref() == Some(service_name)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Parse Token Tests - Nodes
    // Based on: https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#graph-cache
    // =========================================================================

    #[test]
    fn test_parse_node_token() {
        let token = "@ros2_lv/0/abc123/0/0/NN/%/%/my_node";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.domain_id, 0);
        assert_eq!(info.session_id, "abc123");
        assert_eq!(info.node_id, 0);
        assert_eq!(info.kind, EntityKind::Node);
        assert_eq!(info.node_name, "my_node");
        assert!(info.topic_name.is_none());
    }

    #[test]
    fn test_parse_listener_node_from_design_doc() {
        // Example from design doc:
        // `listener` node (with `ROS_DOMAIN_ID=2`):
        // @ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/0/NN/%/%/listener
        let token = "@ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/0/NN/%/%/listener";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.domain_id, 2);
        assert_eq!(info.session_id, "aac3178e146ba6f1fc6e6a4085e77f21");
        assert_eq!(info.node_id, 0);
        assert_eq!(info.entity_id, 0);
        assert_eq!(info.kind, EntityKind::Node);
        assert_eq!(info.enclave, "");
        assert_eq!(info.namespace, "");
        assert_eq!(info.node_name, "listener");
    }

    #[test]
    fn test_parse_node_with_namespace() {
        let token = "@ros2_lv/0/session123/1/1/NN/%/%robot1/my_node";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.node_id, 1);
        assert_eq!(info.namespace, "/robot1");
        assert_eq!(info.node_name, "my_node");
    }

    #[test]
    fn test_parse_node_with_enclave() {
        let token = "@ros2_lv/0/sess/0/0/NN/%secure_enclave/%ns/secure_node";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.enclave, "/secure_enclave");
        assert_eq!(info.namespace, "/ns");
    }

    // =========================================================================
    // Parse Token Tests - Publishers
    // =========================================================================

    #[test]
    fn test_parse_publisher_token() {
        let token = "@ros2_lv/0/abc123/0/10/MP/%/%/my_node/%chatter/std_msgs::msg::dds_::String_/RIHS01_abc/::,10:,:,:,,";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.kind, EntityKind::Publisher);
        assert_eq!(info.topic_name, Some("/chatter".to_string()));
        assert_eq!(
            info.type_name,
            Some("std_msgs::msg::dds_::String_".to_string())
        );
    }

    #[test]
    fn test_parse_talker_publisher_from_design_doc() {
        // Example from design doc:
        // `talker` node's publisher on `chatter` topic:
        // @ros2_lv/2/8b20917502ee955ac4476e0266340d5c/0/10/MP/%/%/talker/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/::,7:,:,:,,
        let token = "@ros2_lv/2/8b20917502ee955ac4476e0266340d5c/0/10/MP/%/%/talker/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/::,7:,:,:,,";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.domain_id, 2);
        assert_eq!(info.session_id, "8b20917502ee955ac4476e0266340d5c");
        assert_eq!(info.node_id, 0);
        assert_eq!(info.entity_id, 10);
        assert_eq!(info.kind, EntityKind::Publisher);
        assert_eq!(info.enclave, "");
        assert_eq!(info.namespace, "");
        assert_eq!(info.node_name, "talker");
        assert_eq!(info.topic_name, Some("/chatter".to_string()));
        assert_eq!(info.type_name, Some("std_msgs::msg::dds_::String_".to_string()));
        assert_eq!(info.type_hash, Some("RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18".to_string()));
    }

    // =========================================================================
    // Parse Token Tests - Subscribers
    // =========================================================================

    #[test]
    fn test_parse_listener_subscription_from_design_doc() {
        // Example from design doc:
        // `listener` node's subscription on `chatter` topic:
        // @ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/10/MS/%/%/listener/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/::,10:,:,:,,
        let token = "@ros2_lv/2/aac3178e146ba6f1fc6e6a4085e77f21/0/10/MS/%/%/listener/%chatter/std_msgs::msg::dds_::String_/RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18/::,10:,:,:,,";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.domain_id, 2);
        assert_eq!(info.kind, EntityKind::Subscriber);
        assert_eq!(info.node_name, "listener");
        assert_eq!(info.topic_name, Some("/chatter".to_string()));
        assert_eq!(info.type_name, Some("std_msgs::msg::dds_::String_".to_string()));
    }

    // =========================================================================
    // Parse Token Tests - Service Servers
    // =========================================================================

    #[test]
    fn test_parse_service_server_from_design_doc() {
        // Example from design doc:
        // `add_two_ints_server` node's service server:
        // @ros2_lv/2/f9980ee0495eaafb3e38f0d19e2eae12/0/10/SS/%/%/add_two_ints_server/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/::,10:,:,:,,
        let token = "@ros2_lv/2/f9980ee0495eaafb3e38f0d19e2eae12/0/10/SS/%/%/add_two_ints_server/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/::,10:,:,:,,";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.domain_id, 2);
        assert_eq!(info.session_id, "f9980ee0495eaafb3e38f0d19e2eae12");
        assert_eq!(info.kind, EntityKind::ServiceServer);
        assert_eq!(info.node_name, "add_two_ints_server");
        assert_eq!(info.topic_name, Some("/add_two_ints".to_string()));
        assert_eq!(info.type_name, Some("example_interfaces::srv::dds_::AddTwoInts_".to_string()));
        assert_eq!(info.type_hash, Some("RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a".to_string()));
    }

    // =========================================================================
    // Parse Token Tests - Service Clients
    // =========================================================================

    #[test]
    fn test_parse_service_client_from_design_doc() {
        // Example from design doc:
        // `add_two_ints_client` node's service client:
        // @ros2_lv/2/e1dc8d1b45ae8717fce78689cc655685/0/10/SC/%/%/add_two_ints_client/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/::,10:,:,:,,
        let token = "@ros2_lv/2/e1dc8d1b45ae8717fce78689cc655685/0/10/SC/%/%/add_two_ints_client/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_e118de6bf5eeb66a2491b5bda11202e7b68f198d6f67922cf30364858239c81a/::,10:,:,:,,";
        let info = GraphCache::parse_liveliness_token(token).unwrap();

        assert_eq!(info.domain_id, 2);
        assert_eq!(info.session_id, "e1dc8d1b45ae8717fce78689cc655685");
        assert_eq!(info.kind, EntityKind::ServiceClient);
        assert_eq!(info.node_name, "add_two_ints_client");
        assert_eq!(info.topic_name, Some("/add_two_ints".to_string()));
        assert_eq!(info.type_name, Some("example_interfaces::srv::dds_::AddTwoInts_".to_string()));
    }

    // =========================================================================
    // Invalid Token Tests
    // =========================================================================

    #[test]
    fn test_parse_invalid_prefix() {
        let token = "@invalid/0/abc123/0/0/NN/%/%/my_node";
        assert!(GraphCache::parse_liveliness_token(token).is_none());
    }

    #[test]
    fn test_parse_too_short() {
        let token = "@ros2_lv/0/abc123";
        assert!(GraphCache::parse_liveliness_token(token).is_none());
    }

    #[test]
    fn test_parse_invalid_entity_kind() {
        let token = "@ros2_lv/0/abc123/0/0/XX/%/%/my_node";
        assert!(GraphCache::parse_liveliness_token(token).is_none());
    }

    #[test]
    fn test_parse_invalid_domain_id() {
        let token = "@ros2_lv/not_a_number/abc123/0/0/NN/%/%/my_node";
        assert!(GraphCache::parse_liveliness_token(token).is_none());
    }

    // =========================================================================
    // Graph Cache Operations Tests
    // =========================================================================

    #[test]
    fn test_handle_liveliness_put() {
        let mut cache = GraphCache::new();
        let token = "@ros2_lv/0/abc123/0/0/NN/%/%/my_node";

        cache.handle_liveliness_token(token, SampleKind::Put);

        assert_eq!(cache.get_node_names().len(), 1);
        assert!(cache.get_node_names().contains(&"/my_node".to_string()));
    }

    #[test]
    fn test_handle_liveliness_delete() {
        let mut cache = GraphCache::new();
        let token = "@ros2_lv/0/abc123/0/0/NN/%/%/my_node";

        cache.handle_liveliness_token(token, SampleKind::Put);
        assert_eq!(cache.get_node_names().len(), 1);

        cache.handle_liveliness_token(token, SampleKind::Delete);
        assert_eq!(cache.get_node_names().len(), 0);
    }

    #[test]
    fn test_count_publishers() {
        let mut cache = GraphCache::new();
        let token1 = "@ros2_lv/0/sess1/0/10/MP/%/%/node1/%chatter/std_msgs::msg::dds_::String_/RIHS01_abc/qos";
        let token2 = "@ros2_lv/0/sess2/0/10/MP/%/%/node2/%chatter/std_msgs::msg::dds_::String_/RIHS01_abc/qos";
        let token3 = "@ros2_lv/0/sess3/0/10/MP/%/%/node3/%other_topic/std_msgs::msg::dds_::String_/RIHS01_abc/qos";

        cache.handle_liveliness_token(token1, SampleKind::Put);
        cache.handle_liveliness_token(token2, SampleKind::Put);
        cache.handle_liveliness_token(token3, SampleKind::Put);

        assert_eq!(cache.count_publishers("/chatter"), 2);
        assert_eq!(cache.count_publishers("/other_topic"), 1);
        assert_eq!(cache.count_publishers("/nonexistent"), 0);
    }

    #[test]
    fn test_count_subscribers() {
        let mut cache = GraphCache::new();
        let token1 = "@ros2_lv/0/sess1/0/10/MS/%/%/node1/%chatter/std_msgs::msg::dds_::String_/RIHS01_abc/qos";
        let token2 = "@ros2_lv/0/sess2/0/10/MS/%/%/node2/%chatter/std_msgs::msg::dds_::String_/RIHS01_abc/qos";

        cache.handle_liveliness_token(token1, SampleKind::Put);
        cache.handle_liveliness_token(token2, SampleKind::Put);

        assert_eq!(cache.count_subscribers("/chatter"), 2);
    }

    #[test]
    fn test_is_service_available() {
        let mut cache = GraphCache::new();
        let token = "@ros2_lv/0/sess/0/10/SS/%/%/server_node/%add_two_ints/example_interfaces::srv::dds_::AddTwoInts_/RIHS01_abc/qos";

        assert!(!cache.is_service_available("/add_two_ints"));

        cache.handle_liveliness_token(token, SampleKind::Put);
        assert!(cache.is_service_available("/add_two_ints"));

        cache.handle_liveliness_token(token, SampleKind::Delete);
        assert!(!cache.is_service_available("/add_two_ints"));
    }

    #[test]
    fn test_get_publishers_info() {
        let mut cache = GraphCache::new();
        let token = "@ros2_lv/0/sess/0/10/MP/%/%/talker/%chatter/std_msgs::msg::dds_::String_/RIHS01_abc/qos";

        cache.handle_liveliness_token(token, SampleKind::Put);

        let pubs = cache.get_publishers_info("/chatter");
        assert_eq!(pubs.len(), 1);
        assert_eq!(pubs[0].node_name, "talker");
        assert_eq!(pubs[0].type_name, Some("std_msgs::msg::dds_::String_".to_string()));
    }

    #[test]
    fn test_get_node_names_with_namespace() {
        let mut cache = GraphCache::new();
        let token1 = "@ros2_lv/0/sess/0/0/NN/%/%/node1";
        let token2 = "@ros2_lv/0/sess/1/1/NN/%/%robot1/node2";
        let token3 = "@ros2_lv/0/sess/2/2/NN/%/%robot1%arm/node3";

        cache.handle_liveliness_token(token1, SampleKind::Put);
        cache.handle_liveliness_token(token2, SampleKind::Put);
        cache.handle_liveliness_token(token3, SampleKind::Put);

        let names = cache.get_node_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"/node1".to_string()));
        assert!(names.contains(&"/robot1/node2".to_string()));
        assert!(names.contains(&"/robot1/arm/node3".to_string()));
    }
}
