//! Selector-based pub/sub example
//!
//! Demonstrates callback-based message handling using the Selector pattern.
//!
//! Run with:
//! ```bash
//! cargo run -p simple --bin selector_pubsub --features jazzy
//! ```

use oxidros::error::Result;
use oxidros::oxidros_msg::common_interfaces::std_msgs;
use oxidros::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

fn main() -> Result<()> {
    // Initialize logging
    init_ros_logging("selector_pubsub");

    let ctx = Context::new()?;
    let node = ctx.create_node("selector_pubsub_demo", None)?;

    // Create publisher
    let publisher = node.create_publisher::<std_msgs::msg::String>("chatter", None)?;
    let publisher = Arc::new(publisher);

    // Create subscriber
    let subscriber = node.create_subscriber::<std_msgs::msg::String>("chatter", None)?;

    // Message counter
    let counter = Arc::new(AtomicU32::new(0));

    // Create selector for event handling
    let mut selector = ctx.create_selector()?;

    // Add subscriber callback - processes incoming messages
    selector.add_subscriber(
        subscriber,
        Box::new(move |msg| {
            let data = msg.sample.data.get_string();
            tracing::info!("Received: {}", data);
        }),
    );

    // Add timer to publish messages every second
    let pub_clone = Arc::clone(&publisher);
    let counter_clone = Arc::clone(&counter);
    selector.add_wall_timer(
        "publish_timer",
        Duration::from_secs(1),
        Box::new(move || {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            let mut msg = std_msgs::msg::String::new().unwrap();
            msg.data.assign(&format!("Hello, World! #{}", count));
            if let Err(e) = pub_clone.send(&msg) {
                tracing::error!("Failed to publish: {}", e);
            } else {
                tracing::info!("Published message #{}", count);
            }
        }),
    );

    // Add status timer every 5 seconds
    let counter_clone = Arc::clone(&counter);
    selector.add_wall_timer(
        "status_timer",
        Duration::from_secs(5),
        Box::new(move || {
            let count = counter_clone.load(Ordering::SeqCst);
            tracing::info!("Status: {} messages published so far", count);
        }),
    );

    tracing::info!("Selector-based pub/sub demo started");
    tracing::info!("Publishing and subscribing on topic 'chatter'");

    // Main event loop - selector.wait() processes events
    loop {
        if let Err(e) = selector.wait() {
            tracing::error!("Selector error: {}", e);
            break;
        }
    }

    Ok(())
}
