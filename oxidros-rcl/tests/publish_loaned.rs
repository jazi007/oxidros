pub mod common;

use oxidros_rcl::context::Context;
use oxidros_rcl::msg::common_interfaces::example_interfaces::msg::Int64;
use std::error::Error;

const TOPIC_NAME: &str = "test_publish_loaned";

#[test]
fn test_publish_loaned() -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
    let ctx = Context::new()?;
    let node = ctx
        .create_node("test_publish_node", None, Default::default())
        .unwrap();

    let publisher = node.create_publisher::<Int64>(TOPIC_NAME, Default::default())?;

    let mut loaned = publisher.borrow_loaned_message()?;
    *loaned = Int64 { data: 100 };

    publisher.send_loaned(loaned)?;

    Ok(())
}
