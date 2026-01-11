//! Service client.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Service Clients](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#service-clients)

use crate::{
    attachment::{Attachment, GID_SIZE, generate_gid},
    error::{Error, Result},
    keyexpr::{EntityKind, liveliness_entity_keyexpr, topic_keyexpr},
    node::Node,
};
use oxidros_core::{TypeSupport, qos::Profile};
use std::{
    marker::PhantomData,
    sync::{Arc, atomic::AtomicI64},
    time::Duration,
};
use zenoh::query::QueryTarget;
use zenoh::{Wait, bytes::ZBytes};

/// Service client response with header.
pub struct ClientResponse<T> {
    /// Response data.
    pub response: T,
    /// Response attachment (required by rmw_zenoh protocol).
    pub attachment: Attachment,
}

/// Service client.
///
/// Sends requests to a service server and receives responses.
///
/// # Example
///
/// ```ignore
/// let mut client = node.create_client::<std_srvs::srv::Empty>("my_service", None)?;
///
/// let request = std_srvs::srv::Empty_Request {};
/// let response = client.call(&request).await?;
/// ```
pub struct Client<T: oxidros_core::ServiceMsg> {
    /// Parent node.
    node: Arc<Node>,
    /// Service name.
    service_name: String,
    /// Fully qualified service name.
    fq_service_name: String,
    /// Key expression for queries.
    key_expr: String,
    /// Client GID.
    gid: [u8; GID_SIZE],
    /// Sequence number counter.
    sequence_number: AtomicI64,
    /// Liveliness token.
    _liveliness_token: zenoh::liveliness::LivelinessToken,
    /// Phantom data for service type.
    _phantom: PhantomData<T>,
}

impl<T: oxidros_core::ServiceMsg> Client<T>
where
    T::Request: TypeSupport,
    T::Response: TypeSupport,
{
    /// Create a new service client.
    ///
    /// # Arguments
    ///
    /// * `node` - Parent node
    /// * `service_name` - Original service name (for display)
    /// * `fq_service_name` - Fully qualified service name (already expanded and remapped)
    /// * `qos` - QoS profile
    pub(crate) fn new(
        node: Arc<Node>,
        service_name: &str,
        fq_service_name: &str,
        qos: Profile,
    ) -> Result<Self> {
        // Get type info - use request type for service key
        let type_name = T::type_name();
        let type_hash = T::type_hash()?;

        // Build key expression
        let key_expr = topic_keyexpr(
            node.context().domain_id(),
            fq_service_name,
            type_name,
            &type_hash,
        );

        // Generate client GID
        let gid = generate_gid();
        let entity_id = node.allocate_entity_id();

        // Create liveliness token
        let token_key = liveliness_entity_keyexpr(
            node.context().domain_id(),
            node.context().session_id(),
            node.node_id(),
            entity_id,
            EntityKind::ServiceClient,
            node.enclave(),
            &node.namespace()?,
            &node.name()?,
            fq_service_name,
            type_name,
            &type_hash,
            &qos,
        );

        let liveliness_token = node
            .context()
            .session()
            .liveliness()
            .declare_token(&token_key)
            .wait()?;

        Ok(Client {
            node,
            service_name: service_name.to_string(),
            fq_service_name: fq_service_name.to_string(),
            key_expr,
            gid,
            sequence_number: AtomicI64::new(0),
            _liveliness_token: liveliness_token,
            _phantom: PhantomData,
        })
    }
}

impl<T: oxidros_core::ServiceMsg> Client<T>
where
    T::Request: TypeSupport,
    T::Response: TypeSupport,
{
    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the fully qualified service name.
    pub fn fq_service_name(&self) -> &str {
        &self.fq_service_name
    }

    /// Get the client GID.
    pub fn gid(&self) -> &[u8; GID_SIZE] {
        &self.gid
    }

    /// Check if the service is available.
    pub fn is_service_available(&self) -> bool {
        self.node
            .context()
            .graph_cache()
            .is_service_available(&self.fq_service_name)
    }

    /// Send a request and wait for a response.
    ///
    /// # Arguments
    ///
    /// * `request` - The request message
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails
    /// - The query fails
    /// - No response is received
    /// - Deserialization fails
    pub async fn call(&self, request: &T::Request) -> Result<ClientResponse<T::Response>> {
        // Serialize request
        let payload = request.to_bytes()?;
        // Increment sequence number
        let seq = self
            .sequence_number
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        // Create attachment
        let attachment = Attachment::new(seq, self.gid);
        let attachment_bytes = attachment.to_bytes();
        // Send query
        let replies = self
            .node
            .context()
            .session()
            .get(&self.key_expr)
            .payload(payload)
            .attachment(ZBytes::from(attachment_bytes.to_vec()))
            .target(QueryTarget::All) // ALL_COMPLETE equivalent
            .await?;

        // Wait for reply with matching sequence number
        loop {
            let reply = replies.recv_async().await?;
            let sample = reply.result().map_err(|e| Error::Zenoh(format!("{e}")))?;
            // Parse response attachment (required by protocol)
            let attachment_bytes = sample.attachment().ok_or(Error::MissingAttachment)?;
            let response_attachment = Attachment::from_bytes(&attachment_bytes.to_bytes())?;
            // Verify sequence number matches our request
            // (server echoes back the client's sequence number)
            if response_attachment.sequence_number != seq {
                // Wrong sequence number, skip this reply and wait for another
                continue;
            }
            let response_bytes: Vec<u8> = sample.payload().to_bytes().to_vec();
            let response = T::Response::from_bytes(&response_bytes)?;

            return Ok(ClientResponse {
                response,
                attachment: response_attachment,
            });
        }
    }

    /// Send a request with a custom timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails
    /// - The query fails
    /// - No response is received (timeout)
    /// - Response is missing attachment (protocol violation)
    /// - Response attachment is invalid
    /// - Deserialization fails
    pub async fn call_with_timeout(
        &self,
        request: &T::Request,
        timeout: Duration,
    ) -> Result<ClientResponse<T::Response>> {
        match tokio::time::timeout(timeout, self.call(request)).await {
            Ok(v) => v,
            Err(_) => Err(Error::Timeout),
        }
    }

    /// Get the parent node.
    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }
}

// ============================================================================
// RosClient trait implementation
// ============================================================================

impl<T: oxidros_core::ServiceMsg> oxidros_core::api::RosClient<T> for Client<T>
where
    T::Request: TypeSupport,
    T::Response: TypeSupport,
{
    fn service_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(Client::service_name(self))
    }

    fn service_available(&self) -> bool {
        self.is_service_available()
    }

    async fn call_service(&mut self, request: &T::Request) -> Result<T::Response> {
        let response = self.call(request).await?;
        Ok(response.response)
    }
}
