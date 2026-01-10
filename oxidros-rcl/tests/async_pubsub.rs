#![cfg(feature = "rcl")]

pub mod common;

use oxidros_rcl::msg::common_interfaces::example_interfaces::msg::Int64;
use oxidros_rcl::{
    context::Context,
    topic::{publisher::Publisher, subscriber::Subscriber},
};
use std::{error::Error, time::Duration};

const TOPIC_NAME: &str = "test_async_pubsub";

#[tokio::test(flavor = "multi_thread")]
async fn test_async_pubsub() -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
    // create a context
    let ctx = Context::new()?;

    // create nodes
    let node_pub = ctx.create_node_with_opt("test_async_pub_node", None, Default::default())?;
    let node_sub = ctx.create_node_with_opt("test_async_sub_node", None, Default::default())?;

    // create a publisher
    let p = common::create_publisher(node_pub, TOPIC_NAME, true).unwrap();

    // create a subscriber
    let s = common::create_subscriber(node_sub, TOPIC_NAME, true).unwrap();

    // create tasks
    let p = tokio::task::spawn(run_publisher(p));
    let s = tokio::task::spawn(run_subscriber(s));
    p.await.unwrap();
    s.await.unwrap();
    println!("finished");

    Ok(())
}

/// The publisher
async fn run_publisher(p: Publisher<Int64>) {
    let dur = Duration::from_millis(100);
    for n in 0..3 {
        // publish a message periodically
        let msg = Int64 { data: n };
        if let Err(e) = p.send(&msg) {
            println!("error: {e}");
            return;
        }

        // sleep 100[ms]
        tokio::time::sleep(dur).await;
        println!("async publish: msg = {n}");
    }
}

/// The subscriber
async fn run_subscriber(mut s: Subscriber<Int64>) {
    let dur = Duration::from_millis(500);
    for n in 0.. {
        // receive a message specifying timeout of 500ms
        match tokio::time::timeout(dur, s.recv()).await {
            Ok(Ok(msg)) => {
                // received a message
                println!("async subscribe: msg = {}", msg.data);
                assert_eq!(msg.data, n);
            }
            Ok(Err(e)) => panic!("{}", e), // fatal error
            Err(_) => {
                // timeout
                println!("async subscribe: timeout");
                break;
            }
        }
    }
}
