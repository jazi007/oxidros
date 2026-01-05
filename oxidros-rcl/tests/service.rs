pub mod common;

use oxidros_rcl::{
    RecvResult,
    context::Context,
    error::DynError,
    msg::common_interfaces::example_interfaces::srv::{AddTwoInts_Request, AddTwoInts_Response},
};
use std::time::Duration;

const SERVICE_NAME1: &str = "test_service1";

#[test]
fn test_service() -> Result<(), DynError> {
    // create a context
    let ctx = Context::new()?;

    // create a server node
    let node_server = ctx.create_node("test_service_server_node", None, Default::default())?;

    // create a client node
    let node_client = ctx.create_node("test_service_client_node", None, Default::default())?;

    // create a server and a client
    let server = common::create_server(node_server, SERVICE_NAME1)?;
    let mut client = common::create_client(node_client, SERVICE_NAME1)?;

    // create a selector
    let mut selector = ctx.create_selector()?;

    // Client: send a request
    let req = AddTwoInts_Request { a: 1, b: 7 };
    let rcv_client = match client.send_ret_seq(&req) {
        Ok((c, seq)) => {
            println!("Client: seq = {seq}");
            c
        }
        Err(e) => {
            return Err(e.into());
        }
    };

    // Server: wait the request
    selector.add_server(
        server,
        Box::new(move |request, header| {
            println!(
                "Server: received: data = {:?}, header = {:?}",
                request, header
            );
            AddTwoInts_Response {
                sum: request.a + request.b,
            }
        }),
    );
    selector.wait()?;

    std::thread::sleep(Duration::from_millis(1));

    // Client: receive the response
    match rcv_client.try_recv() {
        RecvResult::Ok((data, header)) => {
            println!("Client: sum = {}, header = {:?}", data.sum, header);
            assert_eq!(data.sum, 8);
            Ok(())
        }
        RecvResult::RetryLater => {
            println!("should retry");
            Ok(())
        }
        RecvResult::Err(e) => Err(e),
    }
}
