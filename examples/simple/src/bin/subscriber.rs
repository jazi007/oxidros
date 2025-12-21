use oxidros::{
    context::Context, error::DynError, logger::Logger,
    msg::common_interfaces::std_msgs::msg::String, pr_info,
};

fn main() -> Result<(), DynError> {
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None, Default::default())?;
    let sub = node.create_subscriber::<String>("chatter", None)?;
    let mut selector = ctx.create_selector()?;
    let logger = Logger::new("simple");
    selector.add_subscriber(
        sub,
        Box::new(move |msg| {
            pr_info!(logger, "{:?}", msg.data.get_string());
        }),
    );
    loop {
        selector.wait()?;
    }
}
