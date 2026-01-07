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
}
