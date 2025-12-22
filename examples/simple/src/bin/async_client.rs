use std::time::Duration;

use oxidros::{
    context::Context,
    error::DynError,
    logger::Logger,
    msg::common_interfaces::example_interfaces::srv::{AddTwoInts, AddTwoInts_Request},
    pr_info,
};

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let logger = Logger::new("simple");
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None, Default::default())?;
    let mut client = node.create_client::<AddTwoInts>("add_two_ints", None)?;
    while !client.is_service_available()? {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut req = AddTwoInts_Request::new().unwrap();
    let mut index = 0;
    loop {
        req.a = index;
        req.b = index + 1;
        index += 1;
        let crcv = client.send(&req)?;
        let resp = crcv.recv().await?;
        client = resp.0;
        pr_info!(logger, "{:?}", resp.1);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
