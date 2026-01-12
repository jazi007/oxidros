//! Publish/Subscribe integration test.
//!
//! Tests basic pub/sub functionality using the unified API.
//! Works with both RCL and Zenoh backends.

mod common;

use oxidros::prelude::*;
use oxidros_msg::common_interfaces::example_interfaces::msg::Int64;
use std::error::Error;
use std::ops::Deref;
use std::time::Duration;

const TOPIC_NAME: &str = "test_unified_pubsub";

#[test]
fn test_pubsub() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create a context
    let ctx = Context::new()?;

    // Create nodes
    let node_pub = ctx.new_node("test_unified_pub_node", None)?;
    let node_sub = ctx.new_node("test_unified_sub_node", None)?;

    // Create publisher and subscriber
    let publisher = common::create_publisher(node_pub.clone(), TOPIC_NAME)?;
    let subscriber = common::create_subscriber(node_sub.clone(), TOPIC_NAME)?;

    // Publish a message
    let n = 42i64;
    let msg = Int64 { data: n };
    publisher.send(&msg)?;

    // Wait for message using selector
    let mut selector = ctx.new_selector()?;
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    selector.add_subscriber(
        subscriber,
        Box::new(move |msg: Message<Int64>| {
            // Message implements Deref, so we can directly access fields
            let data = msg.deref().data;
            assert_eq!(data, n);
            COUNT.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        }),
    );

    // Wait with timeout to avoid hanging
    selector.wait_timeout(Duration::from_secs(2))?;

    assert_ne!(COUNT.load(std::sync::atomic::Ordering::Relaxed), 0);
    Ok(())
}

#[test]
fn test_pubsub_multiple_messages() -> Result<(), Box<dyn Error + Send + Sync>> {
    let ctx = Context::new()?;

    let node_pub = ctx.new_node("test_multi_pub_node", None)?;
    let node_sub = ctx.new_node("test_multi_sub_node", None)?;

    let publisher: Publisher<Int64> = node_pub.create_publisher("test_multi_pubsub", None)?;
    let subscriber: Subscriber<Int64> = node_sub.create_subscriber("test_multi_pubsub", None)?;

    // Publish multiple messages
    for i in 0..3 {
        let msg = Int64 { data: i };
        publisher.send(&msg)?;
    }

    // Receive messages
    let mut selector = ctx.new_selector()?;
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    selector.add_subscriber(
        subscriber,
        Box::new(|_msg: Message<Int64>| {
            COUNT.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        }),
    );
    // Wait a few times to receive all messages
    for _ in 0..3 {
        let _ = selector.wait_timeout(Duration::from_millis(500));
    }
    assert_ne!(COUNT.load(std::sync::atomic::Ordering::Relaxed), 0);
    Ok(())
}
