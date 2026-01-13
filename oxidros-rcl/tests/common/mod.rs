#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(deref_nullptr)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(unused_imports)]
#![allow(clippy::upper_case_acronyms)]

use once_cell::sync::OnceCell;
// Use oxidros-msg generated messages instead of custom ones
use oxidros_rcl::msg::common_interfaces::example_interfaces::{
    action::Fibonacci, msg::Int64, srv::AddTwoInts,
};

use oxidros_rcl::{
    self,
    error::Result,
    msg::{ServiceMsg, TypeSupport},
    node::Node,
    service::{client::Client, server::Server},
    topic::{publisher::Publisher, subscriber::Subscriber},
};
use std::future::Future;
use std::{error::Error, sync::Arc};

pub fn create_publisher(
    node: Arc<Node>,
    topic_name: &str,
    disable_loaned_message: bool,
) -> Result<Publisher<Int64>> {
    let _ = disable_loaned_message;
    node.create_publisher(topic_name, Default::default())
}

pub fn create_subscriber(
    node: Arc<Node>,
    topic_name: &str,
    disable_loaned_message: bool,
) -> Result<Subscriber<Int64>> {
    let _ = disable_loaned_message;
    node.create_subscriber(topic_name, Default::default())
}

pub fn create_server(node: Arc<Node>, service_name: &str) -> Result<Server<AddTwoInts>> {
    node.create_server(service_name, None)
}

pub fn create_client(node: Arc<Node>, service_name: &str) -> Result<Client<AddTwoInts>> {
    node.create_client(service_name, None)
}
