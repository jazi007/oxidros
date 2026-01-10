#![cfg(feature = "rcl")]

pub mod common;

use oxidros_rcl::context::Context;
use oxidros_rcl::msg::common_interfaces::example_interfaces::msg::Int64;
use std::error::Error;

#[test]
fn test_subscription() -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
    let ctx = Context::new()?;
    let node = ctx
        .create_node("test_subscription_node", None, Default::default())
        .unwrap();

    let subscription = node.create_subscriber::<Int64>("test_subscription", Default::default())?;

    match subscription.try_recv() {
        Ok(None) => Ok(()), // must fail because there is no publisher
        _ => panic!(),
    }
}
