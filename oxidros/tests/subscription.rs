pub mod common;

use oxidros::msg::common_interfaces::example_interfaces::msg::Int64;
use oxidros::{context::Context, RecvResult};
use std::error::Error;

#[test]
fn test_subscription() -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
    let ctx = Context::new()?;
    let node = ctx
        .create_node("test_subscription_node", None, Default::default())
        .unwrap();

    let subscription = node.create_subscriber::<Int64>("test_subscription", Default::default())?;

    match subscription.try_recv() {
        RecvResult::RetryLater => Ok(()), // must fail because there is no publisher
        _ => panic!(),
    }
}
