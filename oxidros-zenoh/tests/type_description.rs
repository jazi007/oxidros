//! Integration tests for z_get_type_description service.

use oxidros_core::types::TypeDescriptionMsg;
use oxidros_msg::common_interfaces::std_msgs::msg::String as StdString;
use oxidros_zenoh::Context;
use std::sync::Arc;
use std::time::Duration;

/// Test that creating a publisher registers its type description,
/// and that it can be queried via z_get_type_description.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_z_get_type_description_after_publish() {
    let ctx = Arc::new(Context::new().expect("Failed to create context"));
    let node = ctx
        .z_create_node("test_type_desc_node", None)
        .expect("Failed to create node");

    // Create a publisher — this should register the type description
    let _pub = node
        .z_create_publisher::<StdString>("chatter", None)
        .expect("Failed to create publisher");

    // Small delay to let the queryable be ready
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Query z_get_type_description with the ROS type name
    let domain_id = ctx.domain_id();
    let key_expr = format!("{domain_id}/**/z_get_type_description");
    let ros_type_name = "std_msgs/msg/String";

    let replies = ctx
        .session()
        .get(&key_expr)
        .payload(ros_type_name.as_bytes())
        .timeout(Duration::from_secs(2))
        .await
        .expect("get failed");

    let reply = replies.recv_async().await.expect("No reply received");
    let sample = reply.result().expect("Reply was an error");
    let json_bytes = sample.payload().to_bytes();
    let desc: TypeDescriptionMsg =
        serde_json::from_slice(&json_bytes).expect("Failed to deserialize TypeDescriptionMsg");

    assert_eq!(desc.type_description.type_name, "std_msgs/msg/String");
    assert!(
        !desc.type_description.fields.is_empty(),
        "Expected at least one field"
    );
    assert_eq!(desc.type_description.fields[0].name, "data");
}

/// Test that an unknown type returns no reply (queryable silently ignores it).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_z_get_type_description_unknown_type() {
    let ctx = Arc::new(Context::new().expect("Failed to create context"));
    let _node = ctx
        .z_create_node("test_unknown_node", None)
        .expect("Failed to create node");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let domain_id = ctx.domain_id();
    let key_expr = format!("{domain_id}/**/z_get_type_description");

    let replies = ctx
        .session()
        .get(&key_expr)
        .payload("nonexistent/msg/Type".as_bytes())
        .timeout(Duration::from_millis(500))
        .await
        .expect("get failed");

    // Should timeout with no replies since the type isn't registered
    let result = tokio::time::timeout(Duration::from_secs(1), replies.recv_async()).await;
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Expected no reply for unknown type"
    );
}

/// Test that the RosNode trait's create_publisher also registers the type.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_ros_node_trait_registers_type() {
    use oxidros_core::api::RosNode;

    let ctx = Arc::new(Context::new().expect("Failed to create context"));
    let node = ctx
        .z_create_node("test_trait_node", None)
        .expect("Failed to create node");

    // Use the RosNode trait method (what the simple example uses)
    let _pub = node
        .create_publisher::<StdString>("chatter", None)
        .expect("Failed to create publisher");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let domain_id = ctx.domain_id();
    let key_expr = format!("{domain_id}/**/z_get_type_description");

    let replies = ctx
        .session()
        .get(&key_expr)
        .payload("std_msgs/msg/String".as_bytes())
        .timeout(Duration::from_secs(2))
        .await
        .expect("get failed");

    let reply = replies
        .recv_async()
        .await
        .expect("No reply — RosNode create_publisher didn't register type");
    let sample = reply.result().expect("Reply was an error");
    let json_bytes = sample.payload().to_bytes();
    let desc: TypeDescriptionMsg =
        serde_json::from_slice(&json_bytes).expect("Failed to deserialize");

    assert_eq!(desc.type_description.type_name, "std_msgs/msg/String");
}
