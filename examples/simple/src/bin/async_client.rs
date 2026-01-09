use std::{sync::atomic::AtomicUsize, time::Duration};

use oxidros::oxidros_msg::common_interfaces::example_interfaces::srv::AddTwoInts_Request;
use oxidros::prelude::*;
use oxidros::{error::Result, oxidros_msg::common_interfaces::example_interfaces::srv::AddTwoInts};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn client_handler(mut client: Client<AddTwoInts>) -> Result<()> {
    let client_n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    // if client_n > 0 {
    //     tokio::time::sleep(Duration::from_secs(3)).await;
    // }
    let name = format!("client{client_n}");
    while !client.service_available() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut req = AddTwoInts_Request::new().unwrap();
    let mut index = client_n as i64 * 1000;
    loop {
        req.a = index;
        req.b = index + 1;
        index += 1;
        let resp = client.call_service(&req).await?;
        println!("{name}: REQ {:?}", resp);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = Context::new()?;
    let node = ctx.new_node("simple", None)?;
    let mut set = tokio::task::JoinSet::new();
    for _ in 0..2 {
        let client = node.new_client::<AddTwoInts>("add_two_ints", None)?;
        set.spawn(client_handler(client));
    }
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        res = set.join_all() => {
            res.into_iter().collect::<Result<Vec<()>>>()?;
        },
        _ = ctrl_c => {},
    }
    Ok(())
}
