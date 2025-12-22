use std::time::Duration;

use oxidros::{
    context::Context,
    error::DynError,
    logger::Logger,
    msg::common_interfaces::example_interfaces::srv::{AddTwoInts, AddTwoInts_Request},
    pr_info, RecvResult,
};

fn main() -> Result<(), DynError> {
    let logger = Logger::new("simple");
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None, Default::default())?;
    let mut client = node.create_client::<AddTwoInts>("add_two_ints", None)?;
    while !client.is_service_available()? {
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut selector = ctx.create_selector()?;
    let mut req = AddTwoInts_Request::new().unwrap();
    let mut index = 0;
    loop {
        req.a = index;
        req.b = index + 1;
        index += 1;
        loop {
            let crcv = client.send(&req)?;
            let resp = crcv.recv_timeout(Duration::from_secs(1), &mut selector);
            match resp {
                RecvResult::Ok((c, v, _)) => {
                    pr_info!(logger, "{v:?}");
                    client = c;
                    break;
                }
                RecvResult::Err(e) => return Err(e),
                RecvResult::RetryLater(c) => {
                    pr_info!(logger, "server unavailabe");
                    client = c.give_up();
                }
            };
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
