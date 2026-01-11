//! Async Service Client/Server integration test.
//!
//! Tests async service functionality using the unified API.
//! Works with both RCL and Zenoh backends.

mod common;

use oxidros::prelude::*;
use oxidros_msg::common_interfaces::example_interfaces::srv::{
    AddTwoInts_Request, AddTwoInts_Response,
};
use std::error::Error;
use std::time::Duration;

const SERVICE_NAME: &str = "test_async_unified_service";

#[tokio::test(flavor = "multi_thread")]
async fn test_async_service() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create context and nodes
    let ctx = Context::new()?;
    let node_server = ctx.new_node("test_async_server", None)?;
    let node_client = ctx.new_node("test_async_client", None)?;

    // Create server and client
    let mut server = common::create_server(node_server.clone(), SERVICE_NAME)?;
    let mut client = common::create_client(node_client.clone(), SERVICE_NAME)?;

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let timeout = Duration::from_secs(3);
        for _ in 0..3 {
            match tokio::time::timeout(timeout, server.recv_request()).await {
                Ok(Ok(request)) => {
                    let req = request.request();
                    println!("Server received: a={}, b={}", req.a, req.b);
                    let response = AddTwoInts_Response { sum: req.a + req.b };
                    if let Err(e) = request.respond(&response) {
                        eprintln!("Server respond error: {e}");
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Server recv error: {e}");
                    break;
                }
                Err(_) => {
                    println!("Server timeout");
                    break;
                }
            }
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn client task
    let client_handle = tokio::spawn(async move {
        for n in 0..3i64 {
            let request = AddTwoInts_Request { a: n, b: n * 10 };
            println!("Client sending: a={}, b={}", request.a, request.b);

            match tokio::time::timeout(Duration::from_secs(2), client.call_service(&request)).await
            {
                Ok(Ok(response)) => {
                    println!("Client received: sum={}", response.sum);
                    assert_eq!(response.sum, n + n * 10);
                }
                Ok(Err(e)) => {
                    eprintln!("Client call error: {e}");
                    break;
                }
                Err(_) => {
                    eprintln!("Client timeout");
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // Wait for both tasks
    let _ = tokio::time::timeout(Duration::from_secs(5), server_handle).await;
    client_handle.await?;

    Ok(())
}
