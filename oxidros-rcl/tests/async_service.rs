#![cfg(feature = "rcl")]

pub mod common;

use oxidros_rcl::msg::common_interfaces::example_interfaces::srv::{
    AddTwoInts, AddTwoInts_Request, AddTwoInts_Response,
};
use oxidros_rcl::{
    context::Context,
    error::Result,
    msg::common_interfaces::std_srvs,
    service::{client::Client, server::Server},
};
use std::time::Duration;

const SERVICE_NAME: &str = "test_async_service";

#[tokio::test(flavor = "multi_thread")]
async fn test_async_service() -> Result<()> {
    // create a context
    let ctx = Context::new()?;

    // create nodes
    let node_server =
        ctx.create_node_with_opt("test_async_server_node", None, Default::default())?;
    let node_client =
        ctx.create_node_with_opt("test_async_client_node", None, Default::default())?;

    // create a server
    let server = common::create_server(node_server, SERVICE_NAME).unwrap();

    // create a client
    let client = common::create_client(node_client, SERVICE_NAME).unwrap();

    // create tasks
    let p = tokio::task::spawn(async {
        let _ = tokio::time::timeout(Duration::from_secs(3), run_server(server)).await;
    });
    let s = tokio::task::spawn(run_client(client));
    p.await.unwrap();
    s.await.unwrap().unwrap();

    println!("finished test_async_service");

    Ok(())
}

/// The server
async fn run_server(mut server: Server<AddTwoInts>) -> Result<()> {
    for _ in 0..3 {
        // receive a request
        let (sender, request) = server.recv().await?.split();
        println!("Server: request = {:?}", *request);

        let response = AddTwoInts_Response {
            sum: request.a + request.b,
        };

        // send a response
        // send returns a new server to receive the next request
        println!("Server: response = {:?}", response);
        match sender.send(&response) {
            Ok(()) => {}
            Err(_e) => {}
        }
    }

    Ok(())
}

/// The client
async fn run_client(mut client: Client<AddTwoInts>) -> Result<()> {
    let dur = Duration::from_millis(500);
    for n in 0..3 {
        let data = AddTwoInts_Request { a: n, b: n * 10 };

        // send a request
        println!("Client: request = {:?}", data);
        let receiver = client.send(&data)?.recv();

        // receive a response
        match tokio::time::timeout(dur, receiver).await {
            Ok(Ok(response)) => {
                tracing::info!(target: "test_async_service", "received: {:?}", *response);
                assert_eq!(response.sum, n + n * 10);
            }
            Ok(Err(e)) => {
                tracing::error!(target: "test_async_service", "error: {e}");
                break;
            }
            Err(_) => {
                continue;
            }
        }

        // sleep 500[ms]
        tokio::time::sleep(dur).await;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_client_rs() {
    // Create a context.
    let ctx = Context::new().unwrap();

    // Create a server node.
    let node = ctx
        .create_node_with_opt("service_test_client_rs", None, Default::default())
        .unwrap();

    // Create a client.
    let client = node
        .create_client::<std_srvs::srv::Empty>("service_test_client_rs", None)
        .unwrap();

    async fn run_client(mut client: Client<std_srvs::srv::Empty>) {
        let dur = Duration::from_millis(100);
        let mut n_timeout = 0;

        loop {
            let request = std_srvs::srv::Empty_Request::new().unwrap();
            let receiver = client.send(&request).unwrap().recv();

            tracing::info!(target: "test_client_rs", "receiving");
            match tokio::time::timeout(dur, receiver).await {
                Ok(Ok(response)) => {
                    tracing::info!(target: "test_client_rs", "received: {:?}", *response);
                }
                Ok(Err(e)) => {
                    tracing::error!(target: "test_client_rs", "error: {e}");
                    break;
                }
                Err(_) => {
                    n_timeout += 1;
                    tracing::info!(target: "test_client_rs", "timeout: n = {n_timeout}");
                    if n_timeout > 10 {
                        return;
                    }
                }
            }
        }
    }

    run_client(client).await; // Spawn an asynchronous task.

    println!("finished test_client_rs");
}
