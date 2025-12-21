use std::time::Duration;

use oxidros::{
    context::Context, error::DynError, logger::Logger,
    msg::common_interfaces::std_msgs::msg::String, pr_info,
};

fn main() -> Result<(), DynError> {
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None, Default::default())?;
    let publisher = node.create_publisher::<String>("chatter", None)?;
    let mut msg = String::new().unwrap();
    let mut index = 0;
    let logger = Logger::new("simple");
    loop {
        msg.data.assign(&format!("Hello World: {index}"));
        pr_info!(logger, "{}", msg.data.get_string());
        index += 1;
        publisher.send(&msg)?;
        std::thread::sleep(Duration::from_secs(1));
    }
}
