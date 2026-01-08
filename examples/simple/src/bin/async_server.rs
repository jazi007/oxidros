use oxidros::{
    context::Context,
    error::Result,
    logger::Logger,
    msg::common_interfaces::example_interfaces::srv::{AddTwoInts, AddTwoInts_Response},
    pr_info,
};

#[tokio::main]
async fn main() -> Result<()> {
    let logger = Logger::new("simple");
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None, Default::default())?;
    let mut server = node.create_server::<AddTwoInts>("add_two_ints", None)?;
    loop {
        let (sender, req, header) = server.recv().await?;
        pr_info!(logger, "Received {req:?} with {header:?}");
        sender.send(&AddTwoInts_Response { sum: req.a + req.b })?;
    }
}
