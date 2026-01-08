use oxidros::{
    context::Context,
    error::Result,
    logger::Logger,
    msg::common_interfaces::example_interfaces::srv::{AddTwoInts, AddTwoInts_Response},
    pr_info,
};

fn main() -> Result<()> {
    let logger = Logger::new("simple");
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None, Default::default())?;
    let server = node.create_server::<AddTwoInts>("add_two_ints", None)?;
    let mut selector = ctx.create_selector()?;
    selector.add_server(
        server,
        Box::new(move |req, header| {
            pr_info!(logger, "{header:?} => {req:?}");
            AddTwoInts_Response { sum: req.a + req.b }
        }),
    );
    loop {
        selector.wait()?;
    }
}
