use std::time::Duration;

use oxidros_wrapper::msg::common_interfaces::std_msgs;
use oxidros_wrapper::prelude::*;
use tokio::{runtime::Builder, signal::ctrl_c, time::interval};

static NAME: &str = "EX1";

async fn ros2_main() -> Result<()> {
    // Create a context.
    let ctx = Context::new()?;
    // Create a node.
    let node = ctx.create_node(NAME, None)?;
    let publisher = node.create_publisher::<std_msgs::msg::String>("ex1_pub", None)?;
    let mut subscriber = node
        .create_subscriber::<std_msgs::msg::String>("ex2_pub", None)?
        .into_stream();
    let mut counter: usize = 0;
    let mut interval = interval(Duration::from_millis(100));
    loop {
        tokio::select! {
            msg = subscriber.next() => {
                let Some(Ok(v)) = msg else {
                    continue;
                };
                println!("Received message {:?}", v.sample.data.get_string());
                let mut message = std_msgs::msg::String::new().unwrap();
                message.data.assign(&format!("{} -> {}", NAME, counter));
                println!("Sending: {:?}", message.data.get_string());
                publisher.send_many([&message, &message])?;
                counter = counter.wrapping_add(1);
            },
            elapsed = interval.tick() => {
                println!("elapsed : {elapsed:?}");
            },
            _ = ctrl_c() => {
                break;
            }
        }
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
