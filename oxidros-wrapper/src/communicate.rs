//! A trait for simplifying cmmunication
//!
use oxidros::{
    msg::{ServiceMsg, TypeSupport},
    node::Node,
    topic::{publisher::Publisher, subscriber::Subscriber},
};
use std::sync::Arc;

use crate::{
    clientserver::{Client, Server},
    common::{Attributes, Result},
};

/// Communicate traits to define all needed methods
pub trait Communicate: Send + Sync {
    /// Create a publisher
    fn new_publisher<T: TypeSupport>(&self, attributes: &Attributes) -> Result<Publisher<T>>;

    /// create a Subscriber
    fn new_subscriber<T: TypeSupport>(&self, attributes: &Attributes) -> Result<Subscriber<T>>;
    /// create an RPC Server
    fn new_server<T: ServiceMsg>(&self, attributes: &Attributes) -> Result<Server<T>>;
    /// create an RPC Client
    fn new_client<T: ServiceMsg>(&self, attributes: &Attributes) -> Result<Client<T>>;
}

impl Communicate for Arc<Node> {
    fn new_publisher<T: TypeSupport>(&self, attributes: &Attributes) -> Result<Publisher<T>> {
        Ok(self.create_publisher::<T>(attributes.name, attributes.qos.clone())?)
    }
    fn new_subscriber<T: TypeSupport>(&self, attributes: &Attributes) -> Result<Subscriber<T>> {
        Ok(self.create_subscriber(attributes.name, attributes.qos.clone())?)
    }
    fn new_server<T: ServiceMsg>(&self, attributes: &Attributes) -> Result<Server<T>> {
        let server = self.create_server(attributes.name, attributes.qos.clone())?;
        Ok(Server(Some(server)))
    }
    fn new_client<T: ServiceMsg>(&self, attributes: &Attributes) -> Result<Client<T>> {
        let client = self.create_client(attributes.name, attributes.qos.clone())?;
        Ok(Client(Some(client)))
    }
}
