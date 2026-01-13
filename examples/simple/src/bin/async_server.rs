use oxidros::{
    error::Result,
    oxidros_msg::common_interfaces::example_interfaces::srv::{AddTwoInts, AddTwoInts_Response},
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = Context::new()?;
    let node = ctx.create_node("simple", None)?;
    let mut server = node.create_server::<AddTwoInts>("add_two_ints", None)?;
    loop {
        let request = server.recv().await?;
        let req = request.request();
        println!("Received {req:?}");
        let response = AddTwoInts_Response { sum: req.a + req.b };
        request.respond(&response)?;
    }
}
