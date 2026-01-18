//! Async Publish/Subscribe integration test.
//!
//! Tests async pub/sub functionality using the unified API.
//! Works with both RCL and Zenoh backends.

mod common;

use oxidros::prelude::*;
use oxidros_msg::common_interfaces::example_interfaces::msg::Int64;
use std::error::Error;
use std::ops::Deref;
use std::time::Duration;

const TOPIC_NAME: &str = "test_async_unified_pubsub";

#[tokio::test(flavor = "multi_thread")]
async fn test_async_pubsub() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create context and nodes
    let ctx = Context::new()?;
    let node_pub = ctx.create_node("test_async_unified_pub", None)?;
    let node_sub = ctx.create_node("test_async_unified_sub", None)?;

    // Create publisher and subscriber
    let publisher = common::create_publisher(node_pub.clone(), TOPIC_NAME)?;
    let mut subscriber = common::create_subscriber(node_sub.clone(), TOPIC_NAME)?;

    // Spawn publisher task
    let pub_handle = tokio::spawn(async move {
        for n in 0..3i64 {
            let msg = Int64 { data: n };
            if let Err(e) = publisher.send(&msg) {
                eprintln!("Publish error: {e}");
                return;
            }
            println!("Published: {n}");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // Spawn subscriber task
    let sub_handle = tokio::spawn(async move {
        let timeout = Duration::from_millis(500);
        for expected in 0..3i64 {
            match tokio::time::timeout(timeout, subscriber.recv()).await {
                Ok(Ok(msg)) => {
                    // TakenMsg implements Deref, so we can directly access fields
                    let data = msg.deref().data;
                    println!("Received: {data}");
                    assert_eq!(data, expected);
                }
                Ok(Err(e)) => {
                    eprintln!("Receive error: {e}");
                    break;
                }
                Err(_) => {
                    println!("Timeout waiting for message");
                    break;
                }
            }
        }
    });

    // Wait for both tasks
    pub_handle.await?;
    sub_handle.await?;

    Ok(())
}
