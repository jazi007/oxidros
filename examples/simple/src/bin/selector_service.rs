//! Selector-based service server example
//!
//! Demonstrates callback-based service handling using the Selector pattern.
//!
//! Run with:
//! ```bash
//! cargo run -p simple --bin selector_service --features jazzy
//! ```
//!
//! Test with (in another terminal):
//! ```bash
//! ros2 service call /add_two_ints example_interfaces/srv/AddTwoInts "{a: 5, b: 3}"
//! ```

use oxidros::error::Result;
use oxidros::oxidros_msg::common_interfaces::example_interfaces;
use oxidros::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

type AddTwoInts = example_interfaces::srv::AddTwoInts;

fn main() -> Result<()> {
    // Initialize logging
    init_ros_logging("selector_service");

    let ctx = Context::new()?;
    let node = ctx.create_node("selector_service_demo", None)?;

    // Create service server
    let server = node.create_server::<AddTwoInts>("add_two_ints", None)?;

    // Request counter
    let request_count = Arc::new(AtomicU64::new(0));
    let request_count_cb = Arc::clone(&request_count);

    // Create selector
    let mut selector = ctx.create_selector()?;

    // Add server callback
    selector.add_server(
        server,
        Box::new(move |request| {
            let count = request_count_cb.fetch_add(1, Ordering::SeqCst) + 1;
            let a = request.sample.a;
            let b = request.sample.b;
            let sum = a + b;
            tracing::info!("Request #{}: {} + {} = {}", count, a, b, sum);

            // Create response
            let mut response = example_interfaces::srv::AddTwoInts_Response::new().unwrap();
            response.sum = sum;
            response
        }),
    );

    // Add status timer
    let request_count_timer = Arc::clone(&request_count);
    selector.add_wall_timer(
        "status_timer",
        Duration::from_secs(10),
        Box::new(move || {
            let count = request_count_timer.load(Ordering::SeqCst);
            tracing::info!("Status: {} requests processed", count);
        }),
    );

    tracing::info!("Selector-based service demo started");
    tracing::info!("Serving 'add_two_ints' service");
    tracing::info!(
        "Test with: ros2 service call /add_two_ints example_interfaces/srv/AddTwoInts \"{{a: 5, b: 3}}\""
    );

    // Main event loop
    loop {
        if let Err(e) = selector.wait() {
            tracing::error!("Selector error: {}", e);
            break;
        }
    }

    Ok(())
}
