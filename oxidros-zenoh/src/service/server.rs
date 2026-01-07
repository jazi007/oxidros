//! Service server.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Service Servers](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#service-servers)

use crate::{
    attachment::{Attachment, GID_SIZE, generate_gid},
    error::{Error, Result},
    keyexpr::{EntityKind, liveliness_entity_keyexpr, topic_keyexpr},
    node::Node,
};
use oxidros_core::qos::Profile;
use ros2_types::TypeSupport;
use std::{marker::PhantomData, sync::Arc};
use zenoh::{Wait, bytes::ZBytes, query::Query};

/// Incoming service request with sender for response.
pub struct ServiceRequest<T: oxidros_core::ServiceMsg> {
    /// Request data.
    pub request: T::Request,
    /// Request attachment.
    pub attachment: Option<Attachment>,
    /// Sender for response.
    sender: RequestSender<T>,
}

impl<T: oxidros_core::ServiceMsg> ServiceRequest<T>
where
    T::Response: TypeSupport,
{
    /// Send a response to this request.
    pub fn respond(self, response: T::Response) -> Result<()> {
        self.sender.send(response)
    }
}

/// Sender for service response.
struct RequestSender<T: oxidros_core::ServiceMsg> {
    query: Query,
    client_gid: [u8; GID_SIZE],
    sequence_number: i64,
    _phantom: PhantomData<T>,
}

impl<T: oxidros_core::ServiceMsg> RequestSender<T>
where
    T::Response: TypeSupport,
{
    fn send(self, response: T::Response) -> Result<()> {
        // Serialize response
        let payload = response.to_bytes()?;

        // Create response attachment (echo back client's seq and gid)
        let attachment = Attachment::new(self.sequence_number, self.client_gid);
        let attachment_bytes = attachment.to_bytes();

        // Reply to query
        self.query
            .reply(self.query.key_expr().clone(), payload)
            .attachment(ZBytes::from(attachment_bytes.to_vec()))
            .wait()
            .map_err(|e| Error::Zenoh(e.into()))?;

        Ok(())
    }
}

/// Service server.
///
/// Receives requests and sends responses.
///
/// # Example
///
/// ```ignore
/// let mut server = node.create_server::<std_srvs::srv::Empty>("my_service", None)?;
///
/// loop {
///     let request = server.recv().await?;
///     let response = std_srvs::srv::Empty_Response {};
///     request.respond(response)?;
/// }
/// ```
pub struct Server<T: oxidros_core::ServiceMsg> {
    /// Parent node.
    node: Arc<Node>,
    /// Service name.
    service_name: String,
    /// Fully qualified service name.
    fq_service_name: String,
    /// Server GID.
    gid: [u8; GID_SIZE],
    /// Request receiver channel.
    receiver: flume::Receiver<(Query, Vec<u8>, Option<Vec<u8>>)>,
    /// Liveliness token.
    _liveliness_token: zenoh::liveliness::LivelinessToken,
    /// Zenoh queryable (kept alive).
    _queryable: zenoh::query::Queryable<()>,
    /// Phantom data for service type.
    _phantom: PhantomData<T>,
}

impl<T: oxidros_core::ServiceMsg> Server<T>
where
    T::Request: TypeSupport,
    T::Response: TypeSupport,
{
    /// Create a new service server.
    pub(crate) fn new(node: Arc<Node>, service_name: &str, qos: Profile) -> Result<Self> {
        // Build fully qualified service name
        let fq_service_name = if service_name.starts_with('/') {
            service_name.to_string()
        } else if node.namespace().is_empty() {
            format!("/{}", service_name)
        } else {
            format!("{}/{}", node.namespace(), service_name)
        };

        // Get type info
        let type_name = T::Request::type_name();
        let type_hash = "RIHS01_TODO"; // TODO: Calculate from TypeDescription

        // Build key expression
        let key_expr = topic_keyexpr(
            node.context().domain_id(),
            &fq_service_name,
            type_name,
            type_hash,
        );

        // Create channel for incoming requests
        let (sender, receiver) = flume::bounded(32);

        // Create Zenoh queryable
        let queryable = node
            .context()
            .session()
            .declare_queryable(&key_expr)
            .complete(true) // Service can answer all queries
            .callback(move |query| {
                // Extract payload
                let payload = query
                    .payload()
                    .map(|p| p.to_bytes().to_vec())
                    .unwrap_or_default();

                // Extract attachment
                let attachment = query.attachment().map(|a| a.to_bytes().to_vec());

                // Send to channel
                let _ = sender.try_send((query, payload, attachment));
            })
            .wait()?;

        // Generate server GID
        let gid = generate_gid();
        let entity_id = node.allocate_entity_id();

        // Create liveliness token
        let token_key = liveliness_entity_keyexpr(
            node.context().domain_id(),
            node.context().session_id(),
            node.node_id(),
            entity_id,
            EntityKind::ServiceServer,
            node.enclave(),
            node.namespace(),
            node.name(),
            &fq_service_name,
            type_name,
            type_hash,
            &qos,
        );

        let liveliness_token = node
            .context()
            .session()
            .liveliness()
            .declare_token(&token_key)
            .wait()?;

        Ok(Server {
            node,
            service_name: service_name.to_string(),
            fq_service_name,
            gid,
            receiver,
            _liveliness_token: liveliness_token,
            _queryable: queryable,
            _phantom: PhantomData,
        })
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the fully qualified service name.
    pub fn fq_service_name(&self) -> &str {
        &self.fq_service_name
    }

    /// Get the server GID.
    pub fn gid(&self) -> &[u8; GID_SIZE] {
        &self.gid
    }

    /// Receive a request asynchronously.
    ///
    /// Returns a `ServiceRequest` that can be used to send a response.
    pub async fn recv(&mut self) -> Result<ServiceRequest<T>> {
        let (query, payload, attachment_bytes) = self
            .receiver
            .recv_async()
            .await
            .map_err(|_| Error::ChannelClosed)?;

        // Deserialize request
        let request = T::Request::from_bytes(&payload)?;

        // Parse attachment
        let attachment = attachment_bytes
            .as_ref()
            .and_then(|bytes| Attachment::from_bytes(bytes));

        // Extract client info for response
        let (sequence_number, client_gid) = attachment
            .as_ref()
            .map(|a| (a.sequence_number, a.gid))
            .unwrap_or((0, [0u8; GID_SIZE]));

        let sender = RequestSender {
            query,
            client_gid,
            sequence_number,
            _phantom: PhantomData,
        };

        Ok(ServiceRequest {
            request,
            attachment,
            sender,
        })
    }

    /// Get the parent node.
    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }
}
