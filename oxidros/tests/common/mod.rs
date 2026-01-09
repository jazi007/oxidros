//! Common utilities for integration tests.
//!
//! This module provides helper functions that work with both RCL and Zenoh backends.

#![allow(dead_code)]

use oxidros::prelude::*;
use oxidros_msg::common_interfaces::example_interfaces::{msg::Int64, srv::AddTwoInts};
use std::sync::Arc;

/// Create a publisher for Int64 messages.
pub fn create_publisher(
    node: Arc<Node>,
    topic_name: &str,
) -> oxidros_core::Result<Publisher<Int64>> {
    node.new_publisher(topic_name, None)
}

/// Create a subscriber for Int64 messages.
pub fn create_subscriber(
    node: Arc<Node>,
    topic_name: &str,
) -> oxidros_core::Result<Subscriber<Int64>> {
    node.new_subscriber(topic_name, None)
}

/// Create a service server for AddTwoInts.
pub fn create_server(
    node: Arc<Node>,
    service_name: &str,
) -> oxidros_core::Result<Server<AddTwoInts>> {
    node.new_server(service_name, None)
}

/// Create a service client for AddTwoInts.
pub fn create_client(
    node: Arc<Node>,
    service_name: &str,
) -> oxidros_core::Result<Client<AddTwoInts>> {
    node.new_client(service_name, None)
}
