use oxidros::{
    context::Context, error::Result, logger::Logger, msg::common_interfaces::std_msgs::msg::String,
    pr_info,
};

fn main() -> Result<()> {
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None, Default::default())?;
    let sub1 = node.create_subscriber::<String>("chatter", None)?;
    let sub2 = node.create_subscriber::<String>("chatter", None)?;
    let logger = Logger::new("simple");
    loop {
        let msg1 = sub1.recv_blocking()?;
        let msg2 = sub2.recv_blocking()?;
        pr_info!(logger, "MSG1 {:?}", msg1.data.get_string());
        pr_info!(logger, "MSG2 {:?}", msg2.data.get_string());
    }
}
