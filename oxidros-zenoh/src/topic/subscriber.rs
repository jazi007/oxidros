//! Topic subscriber.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Subscriptions](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#subscriptions)

use crate::{
    attachment::{Attachment, GID_SIZE, generate_gid},
    error::{Error, Result},
    keyexpr::{EntityKind, liveliness_entity_keyexpr, topic_keyexpr},
    node::Node,
    qos::QosMapping,
};
use oxidros_core::qos::Profile;
use ros2_types::{TypeDescription, TypeSupport};
use std::{marker::PhantomData, sync::Arc};
use zenoh::Wait;

/// Received message with metadata.
#[derive(Debug)]
pub struct ReceivedMessage<T> {
    /// The message data.
    pub data: T,
    /// Message attachment (sequence number, timestamp, GID).
    pub attachment: Option<Attachment>,
}

/// Topic subscriber.
///
/// Receives messages from a topic using Zenoh.
///
/// # Example
///
/// ```ignore
/// let mut subscriber = node.create_subscriber::<std_msgs::msg::String>("chatter", None)?;
///
/// // Async receive
/// let msg = subscriber.recv().await?;
/// println!("Received: {}", msg.data.data);
///
/// // Non-blocking receive
/// if let Some(msg) = subscriber.try_recv()? {
///     println!("Received: {}", msg.data.data);
/// }
/// ```
pub struct Subscriber<T> {
    /// Parent node.
    node: Arc<Node>,
    /// Topic name.
    topic_name: String,
    /// Fully qualified topic name.
    fq_topic_name: String,
    /// Subscriber GID.
    gid: [u8; GID_SIZE],
    /// Message receiver channel.
    receiver: flume::Receiver<(Vec<u8>, Option<Vec<u8>>)>,
    /// Liveliness token.
    _liveliness_token: zenoh::liveliness::LivelinessToken,
    /// Zenoh subscriber (kept alive).
    _zenoh_subscriber: zenoh::pubsub::Subscriber<()>,
    /// Phantom data for type.
    _phantom: PhantomData<T>,
}

impl<T: TypeSupport + TypeDescription> Subscriber<T> {
    /// Create a new subscriber.
    ///
    /// # Arguments
    ///
    /// * `node` - Parent node
    /// * `topic_name` - Original topic name (for display)
    /// * `fq_topic_name` - Fully qualified topic name (already expanded and remapped)
    /// * `qos` - QoS profile
    /// * `entity_kind` - Entity kind for liveliness
    pub(crate) fn new(
        node: Arc<Node>,
        topic_name: &str,
        fq_topic_name: &str,
        qos: Profile,
        entity_kind: EntityKind,
    ) -> Result<Self> {
        // Validate QoS
        QosMapping::validate(&qos);

        // Get type info
        let type_name = T::type_name();
        let type_hash = T::compute_hash()?;

        // Build key expression with wildcard for type hash
        // This allows receiving from publishers with different (compatible) type hashes
        let key_expr = topic_keyexpr(
            node.context().domain_id(),
            fq_topic_name,
            type_name,
            "*", // Wildcard to match any hash
        );

        // Create channel for received messages
        let depth = QosMapping::effective_depth(&qos);
        let (sender, receiver) = flume::bounded(depth);

        // Create Zenoh subscriber
        let session = node.context().session();
        let zenoh_subscriber = session
            .declare_subscriber(&key_expr)
            .callback(move |sample| {
                // Extract payload
                let payload: Vec<u8> = sample.payload().to_bytes().to_vec();

                // Extract attachment if present
                let attachment = sample.attachment().map(|a| a.to_bytes().to_vec());

                // Send to channel (drop if full - KeepLast behavior)
                let _ = sender.try_send((payload, attachment));
            })
            .wait()?;

        // Generate subscriber GID
        let gid = generate_gid();
        let entity_id = node.allocate_entity_id();

        // Create liveliness token
        let token_key = liveliness_entity_keyexpr(
            node.context().domain_id(),
            node.context().session_id(),
            node.node_id(),
            entity_id,
            entity_kind,
            node.enclave(),
            node.namespace(),
            node.name(),
            fq_topic_name,
            type_name,
            &type_hash,
            &qos,
        );

        let liveliness_token = session.liveliness().declare_token(&token_key).wait()?;

        Ok(Subscriber {
            node,
            topic_name: topic_name.to_string(),
            fq_topic_name: fq_topic_name.to_string(),
            gid,
            receiver,
            _liveliness_token: liveliness_token,
            _zenoh_subscriber: zenoh_subscriber,
            _phantom: PhantomData,
        })
    }
}

impl<T: TypeSupport> Subscriber<T> {
    /// Get the topic name.
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Get the fully qualified topic name.
    pub fn fq_topic_name(&self) -> &str {
        &self.fq_topic_name
    }

    /// Get the subscriber GID.
    pub fn gid(&self) -> &[u8; GID_SIZE] {
        &self.gid
    }

    /// Receive a message asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the channel is closed.
    pub async fn recv(&mut self) -> Result<ReceivedMessage<T>> {
        let (payload, attachment_bytes) = self
            .receiver
            .recv_async()
            .await
            .map_err(|_| Error::ChannelClosed)?;

        let data = T::from_bytes(&payload)?;
        let attachment = attachment_bytes.and_then(|bytes| Attachment::from_bytes(&bytes));

        Ok(ReceivedMessage { data, attachment })
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns `None` if no message is available.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn try_recv(&mut self) -> Result<Option<ReceivedMessage<T>>> {
        match self.receiver.try_recv() {
            Ok((payload, attachment_bytes)) => {
                let data = T::from_bytes(&payload)?;
                let attachment = attachment_bytes.and_then(|bytes| Attachment::from_bytes(&bytes));
                Ok(Some(ReceivedMessage { data, attachment }))
            }
            Err(flume::TryRecvError::Empty) => Ok(None),
            Err(flume::TryRecvError::Disconnected) => Err(Error::ChannelClosed),
        }
    }

    /// Get the parent node.
    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }
}
