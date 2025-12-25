pub mod common;

use oxidros::msg::common_interfaces::example_interfaces::srv::{
    AddTwoInts_Request, AddTwoInts_Response,
};
use oxidros::{context::Context, error::DynError, RecvResult};
use std::time::Duration;

const SERVICE_NAME3: &str = "test_service3";

#[test]
fn test_no_server() -> Result<(), DynError> {
    // create a context
    let ctx = Context::new()?;

    // create a client and a server node
    let node_client = ctx.create_node("test_client_no_server_node", None, Default::default())?;
    let node_server = ctx.create_node("test_server_no_server_node", None, Default::default())?;

    // create a server and a client
    let mut client = common::create_client(node_client, SERVICE_NAME3)?;
    let mut server = common::create_server(node_server, SERVICE_NAME3)?;

    std::thread::sleep(Duration::from_millis(500));

    let req = AddTwoInts_Request { a: 1, b: 7 };
    let (_receiver, seq) = client.send_ret_seq(&req).unwrap();
    println!("client: send: seq = {seq}");

    std::thread::sleep(Duration::from_millis(500));

    let srv;
    let request;
    match server.try_recv() {
        RecvResult::Ok((s, req, header)) => {
            println!("server:recv: seq = {:?}", header.get_sequence());
            srv = s;
            request = req;
        }
        RecvResult::RetryLater => panic!("server:try_recv: retry later"),
        RecvResult::Err(e) => panic!("server:try_recv:error: {e}"),
    }

    std::thread::sleep(Duration::from_millis(50));
    println!("client: gave up!");

    let req = AddTwoInts_Request { a: 4, b: 18 };
    let (receiver, seq) = client.send_ret_seq(&req).unwrap();
    println!("clinet:send: seq = {seq}");

    std::thread::sleep(Duration::from_millis(50));

    let resp = AddTwoInts_Response {
        sum: request.a + request.b,
    };
    let _ = srv.send(&resp);

    std::thread::sleep(Duration::from_millis(50));

    match receiver.try_recv() {
        RecvResult::Ok((msg, header)) => {
            panic!(
                "try_recv: msg = {:?}, seq = {:?}",
                msg,
                header.get_sequence()
            );
        }
        RecvResult::Err(_e) => {
            panic!("try_recv: error");
        }
        RecvResult::RetryLater => {
            println!("try_recv: retry later");
        }
    }

    Ok(())
}
