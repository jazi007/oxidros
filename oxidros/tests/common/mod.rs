#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(deref_nullptr)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(unused_imports)]
#![allow(clippy::upper_case_acronyms)]

// Use oxidros-msg generated messages instead of custom ones
use oxidros::msg::common_interfaces::example_interfaces::{
    action::Fibonacci, msg::Int64, srv::AddTwoInts,
};

use oxidros::{
    self,
    error::{DynError, RCLResult},
    msg::{ServiceMsg, TypeSupport},
    node::Node,
    rcl,
    service::{client::Client, server::Server},
    topic::{publisher::Publisher, subscriber::Subscriber},
};
use std::{error::Error, sync::Arc};

pub fn create_publisher(
    node: Arc<Node>,
    topic_name: &str,
    disable_loaned_message: bool,
) -> RCLResult<Publisher<Int64>> {
    let _ = disable_loaned_message;
    node.create_publisher(topic_name, Default::default())
}

pub fn create_subscriber(
    node: Arc<Node>,
    topic_name: &str,
    disable_loaned_message: bool,
) -> RCLResult<Subscriber<Int64>> {
    let _ = disable_loaned_message;
    node.create_subscriber(topic_name, Default::default())
}

pub fn create_server(node: Arc<Node>, service_name: &str) -> RCLResult<Server<AddTwoInts>> {
    node.create_server(service_name, None)
}

pub fn create_client(node: Arc<Node>, service_name: &str) -> RCLResult<Client<AddTwoInts>> {
    node.create_client(service_name, None)
}
