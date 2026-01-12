use oxidros::oxidros_msg::common_interfaces::std_msgs::msg::String;
use oxidros::{error::Result, prelude::*};
use std::time::Duration;

fn main() -> Result<()> {
    let ctx = Context::new()?;
    let node = ctx.new_node("simple", None)?;
    let publisher = node.create_publisher::<String>("chatter", None)?;
    let mut msg = String::new().unwrap();
    let mut index = 0;
    loop {
        msg.data.assign(&format!("Hello World: {index}"));
        println!("{}", msg.data.get_string());
        index += 1;
        publisher.send(&msg)?;
        std::thread::sleep(Duration::from_secs(1));
    }
}
