use oxidros::oxidros_msg::common_interfaces::std_msgs::msg::String;
use oxidros::{error::Result, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = Context::new()?;
    let node = ctx.new_node("simple", None)?;
    let mut sub1 = node.create_subscriber::<String>("chatter", None)?;
    let mut sub2 = node.create_subscriber::<String>("chatter", None)?;
    loop {
        let msg1 = sub1.recv().await?;
        let msg2 = sub2.recv().await?;
        println!("MSG1 {:?}", msg1.data.get_string());
        println!("MSG2 {:?}", msg2.data.get_string());
    }
}
