//! Type resolution with Zenoh service fallback.
//!
//! Resolves a DDS type name to a `TypeDescriptionMsg` by:
//! 1. Checking the build-time `phf` registry (O(1), no I/O)
//! 2. Falling back to a Zenoh `z_get_type_description` query on discovered nodes

use crate::type_registry;
use oxidros_zenoh::Context;
use ros2_types::types::TypeDescriptionMsg;
use std::time::Duration;

/// Resolve a DDS type name to its `TypeDescriptionMsg`.
///
/// Tries the build-time registry first (instant). If not found, queries nodes
/// via the Zenoh-specific `z_get_type_description` service.
pub async fn resolve(dds_name: &str, ctx: &Context) -> Option<TypeDescriptionMsg> {
    // 1. Try build-time registry
    if let Some(desc) = type_registry::lookup_dds(dds_name) {
        return Some(desc.clone());
    }

    // 2. Fall back to z_get_type_description query
    tracing::debug!("Type {dds_name} not in build-time registry, trying z_get_type_description");
    query_type_description(dds_name, ctx).await
}

/// Convert a DDS type name to the ROS2 fully-qualified type name.
/// `"my_pkg::msg::dds_::MyType_"` → `"my_pkg/msg/MyType"`
pub(crate) fn dds_to_ros_type_name(dds_name: &str) -> Option<String> {
    let parts: Vec<&str> = dds_name.split("::").collect();
    if parts.len() < 4 || parts[2] != "dds_" {
        return None;
    }
    let pkg = parts[0];
    let iface = parts[1];
    let type_name = parts[3].strip_suffix('_').unwrap_or(parts[3]);
    Some(format!("{}/{}/{}", pkg, iface, type_name))
}

/// Query `z_get_type_description` on all nodes via Zenoh wildcard.
///
/// Key pattern: `<domain_id>/*/z_get_type_description`
/// Request: UTF-8 ROS type name
/// Response: JSON-encoded `TypeDescriptionMsg`
async fn query_type_description(dds_name: &str, ctx: &Context) -> Option<TypeDescriptionMsg> {
    let ros_type_name = dds_to_ros_type_name(dds_name)?;

    // Wildcard query across all nodes in this domain
    let key_expr = format!("{}/**/z_get_type_description", ctx.domain_id());

    tracing::debug!(key = %key_expr, type_name = %ros_type_name, "querying z_get_type_description");

    let replies = ctx
        .session()
        .get(&key_expr)
        .payload(ros_type_name.as_bytes())
        .timeout(Duration::from_secs(2))
        .await
        .ok()?;

    // Take the first successful reply
    while let Ok(reply) = replies.recv_async().await {
        if let Ok(sample) = reply.result() {
            let json_bytes = sample.payload().to_bytes();
            if let Ok(desc) = serde_json::from_slice::<TypeDescriptionMsg>(&json_bytes) {
                return Some(desc);
            }
        }
    }

    tracing::warn!("No z_get_type_description reply for {dds_name}");
    None
}
