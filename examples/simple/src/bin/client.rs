//! Service client example with selector and tokio
//!
//! Demonstrates a service client using async/await for the service call
//! while using selector pattern for the overall event loop.
//!
//! For a pure async version, see `async_client.rs`.
//!
//! Run with:
//! ```bash
//! # First start the server
//! cargo run -p simple --bin server --features jazzy
//!
//! # Then run the client
//! cargo run -p simple --bin client --features jazzy
//! ```

use oxidros::error::Result;
use oxidros::oxidros_msg::common_interfaces::example_interfaces;
use oxidros::prelude::*;
use std::time::Duration;

type AddTwoInts = example_interfaces::srv::AddTwoInts;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_ros_logging("client");

    let ctx = Context::new()?;
    let node = ctx.create_node("client_demo", None)?;

    // Create service client
    let mut client = node.create_client::<AddTwoInts>("add_two_ints", None)?;

    tracing::info!("Service client demo started");
    tracing::info!("Waiting for 'add_two_ints' service...");

    // Wait for service to be available
    while !client.is_service_available() {
        tracing::debug!("Service not available, waiting...");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    tracing::info!("Service available!");

    // Send multiple requests
    for i in 0..5 {
        let a = (i + 1) * 10;
        let b = i + 1;

        let mut request = example_interfaces::srv::AddTwoInts_Request::new().unwrap();
        request.a = a;
        request.b = b;

        tracing::info!("Sending request #{}: {} + {}", i + 1, a, b);

        match client.call(&request).await {
            Ok(response) => {
                tracing::info!(
                    "Response #{}: {} + {} = {}",
                    i + 1,
                    a,
                    b,
                    response.sample.sum
                );
            }
            Err(e) => {
                tracing::error!("Request #{} failed: {}", i + 1, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    tracing::info!("Client demo completed");
    Ok(())
}
