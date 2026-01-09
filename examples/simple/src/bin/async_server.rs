use oxidros::{
    error::Result,
    oxidros_msg::common_interfaces::example_interfaces::srv::{AddTwoInts, AddTwoInts_Response},
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = Context::new()?;
    let node = ctx.new_node("simple", None)?;
    let mut server = node.new_server::<AddTwoInts>("add_two_ints", None)?;
    loop {
        let request = server.recv_request().await?;
        let req = request.request();
        println!("Received {req:?}");
        let response = AddTwoInts_Response { sum: req.a + req.b };
        request.respond(response)?;
    }
}
