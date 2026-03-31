//! Zenoh-specific type description service (`z_get_type_description`).
//!
//! Registers a Zenoh queryable that serves `TypeDescriptionMsg` for types
//! registered on the node. The protocol is Zenoh-specific (not ROS2 service):
//!
//! - Key: `<domain_id>/<node_fqn>/z_get_type_description`
//! - Request payload: UTF-8 ROS type name (e.g. `"std_msgs/msg/String"`)
//! - Response payload: JSON-encoded `TypeDescriptionMsg`

use oxidros_core::types::TypeDescriptionMsg;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use zenoh::Wait;

/// Shared type registry: maps ROS2 type name (e.g. "std_msgs/msg/String") to its description.
pub(crate) type TypeRegistry = Arc<Mutex<HashMap<String, TypeDescriptionMsg>>>;

/// Set up the `z_get_type_description` Zenoh queryable on a node.
pub(crate) fn setup_queryable(
    session: &zenoh::Session,
    domain_id: u32,
    node_fqn: &str,
    registry: TypeRegistry,
) -> crate::error::Result<zenoh::query::Queryable<()>> {
    let node_fqn_stripped = node_fqn.strip_prefix('/').unwrap_or(node_fqn);
    let key_expr = format!("{domain_id}/{node_fqn_stripped}/z_get_type_description");

    let reg = registry.clone();
    let queryable = session
        .declare_queryable(&key_expr)
        .complete(true)
        .callback(move |query| {
            handle_query(&query, &reg);
        })
        .wait()?;

    tracing::debug!(
        target: oxidros_core::targets::ZENOH,
        key = %key_expr,
        "z_get_type_description service registered"
    );

    Ok(queryable)
}

fn handle_query(query: &zenoh::query::Query, registry: &TypeRegistry) {
    let type_name = match query.payload() {
        Some(p) => match std::str::from_utf8(&p.to_bytes()) {
            Ok(s) => s.to_string(),
            Err(_) => return,
        },
        None => return,
    };

    let reg = registry.lock();
    if let Some(desc) = reg.get(&type_name)
        && let Ok(json) = serde_json::to_vec(desc)
    {
        let _ = query.reply(query.key_expr().clone(), json).wait();
    }
}
