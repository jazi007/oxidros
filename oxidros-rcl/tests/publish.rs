#![cfg(feature = "rcl")]

pub mod common;

use oxidros_rcl::{context::Context, msg::common_interfaces::example_interfaces::msg::Int64};
use std::error::Error;

#[test]
fn test_publish() -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
    let ctx = Context::new()?;
    let node = ctx
        .create_node("test_publish_node", None, Default::default())
        .unwrap();

    let publisher = node.create_publisher::<Int64>("test_publish", Default::default())?;

    let msg = Int64 { data: 100 };
    publisher.send(&msg)?;

    Ok(())
}
