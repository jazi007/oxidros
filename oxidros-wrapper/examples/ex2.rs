use std::time::Duration;

use oxidros::oxidros_msg::common_interfaces::std_msgs;
use oxidros::prelude::Context;
use oxidros_wrapper::prelude::*;
use tokio::runtime::Builder;
use tokio::time::interval;

static NAME: &str = "EX2";

async fn ros2_main() -> Result<()> {
    // Create a context.
    let ctx = Context::new()?;
    // Create a node.
    let node = ctx.create_node(NAME, None, Default::default())?;
    let publisher = node.new_publisher::<std_msgs::msg::String>(&Attributes::new("ex2_pub"))?;
    let subscriber = node.new_subscriber::<std_msgs::msg::String>(&Attributes::new("ex1_pub"))?;
    let mut interval = interval(Duration::from_secs(1));
    let mut message = std_msgs::msg::String::new().unwrap();
    for ii in 0.. {
        message.data.assign(&format!("{}: {}", NAME, ii));
        println!("Sending: {:?}", message.data.get_string());
        publisher.send(&message)?;
        for msg in subscriber.recv_many(usize::MAX)? {
            println!("Received: {:?}", msg.data.get_string());
        }
        interval.tick().await;
    }
    Ok(())
}

fn main() -> Result<()> {
    let rt = Builder::new_multi_thread()
        .thread_name(NAME)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(ros2_main())
}
