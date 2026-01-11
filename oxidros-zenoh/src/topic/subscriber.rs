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
use oxidros_core::{Message, TypeSupport, qos::Profile};
use std::{borrow::Cow, marker::PhantomData, sync::Arc};
use zenoh::Wait;
use zenoh_ext::AdvancedSubscriberBuilderExt;

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
    receiver: flume::Receiver<zenoh::sample::Sample>,
    /// Liveliness token.
    _liveliness_token: zenoh::liveliness::LivelinessToken,
    /// Zenoh advanced subscriber (supports history query for TRANSIENT_LOCAL durability).
    _zenoh_subscriber: zenoh_ext::AdvancedSubscriber<()>,
    /// Phantom data for type.
    _phantom: PhantomData<T>,
}

impl<T: TypeSupport> Subscriber<T> {
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
        let type_hash = T::type_hash()?;

        // Build key expression with wildcard for type hash
        // This allows receiving from publishers with different (compatible) type hashes
        let key_expr = topic_keyexpr(
            node.context().domain_id(),
            fq_topic_name,
            type_name,
            &type_hash,
        );

        // Create channel for received messages
        let depth = QosMapping::effective_depth(&qos);
        let (sender, receiver) = flume::bounded(depth);

        // Clone receiver for use in callback (to implement KeepLast drop-oldest semantics)
        let drain_receiver = receiver.clone();

        // Create Zenoh subscriber
        let session = node.context().session();

        // Build AdvancedSubscriber with history config based on durability QoS
        // For TRANSIENT_LOCAL: query history from publishers with cache
        // For VOLATILE: no history query (max_samples = 0)
        let history_depth = if QosMapping::is_transient_local(&qos) {
            QosMapping::effective_depth(&qos)
        } else {
            0
        };
        let zenoh_subscriber = session
            .declare_subscriber(&key_expr)
            .callback(move |sample: zenoh::sample::Sample| {
                // KeepLast(n) semantics: if channel is full, drop oldest message first
                if sender.is_full() {
                    // Drain one message to make room (drop oldest)
                    let _ = drain_receiver.try_recv();
                }
                // Now there's room - this should always succeed
                let _ = sender.try_send(sample);
            })
            .history(zenoh_ext::HistoryConfig::default().max_samples(history_depth))
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
            &node.namespace()?,
            &node.name()?,
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
    pub fn topic_name(&self) -> Result<Cow<'_, String>> {
        Ok(Cow::Borrowed(&self.topic_name))
    }

    /// Get the fully qualified topic name.
    pub fn fully_qualified_topic_name(&self) -> &str {
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
    pub async fn recv(&mut self) -> Result<Message<T>> {
        let sample = self
            .receiver
            .recv_async()
            .await
            .map_err(|_| Error::ChannelClosed)?;
        let data = T::from_bytes(&sample.payload().to_bytes())?;
        let info = sample
            .attachment()
            .and_then(|bytes| Attachment::from_bytes(&bytes.to_bytes()))
            .unwrap_or_default()
            .into();
        Ok(Message::new(data, info))
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns `None` if no message is available.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn try_recv(&mut self) -> Result<Option<Message<T>>> {
        match self.receiver.try_recv() {
            Ok(sample) => {
                let data = T::from_bytes(&sample.payload().to_bytes())?;
                let info = sample
                    .attachment()
                    .and_then(|bytes| Attachment::from_bytes(&bytes.to_bytes()))
                    .unwrap_or_default()
                    .into();
                Ok(Some(Message::new(data, info)))
            }
            Err(flume::TryRecvError::Empty) => Ok(None),
            Err(flume::TryRecvError::Disconnected) => Err(Error::ChannelClosed),
        }
    }

    /// Receive a message, blocking until one is available.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails or the channel is closed.
    pub fn recv_blocking(&self) -> Result<Message<T>> {
        let sample = self.receiver.recv().map_err(|_| Error::ChannelClosed)?;
        let data = T::from_bytes(&sample.payload().to_bytes())?;
        let info = sample
            .attachment()
            .and_then(|bytes| Attachment::from_bytes(&bytes.to_bytes()))
            .unwrap_or_default()
            .into();
        Ok(Message::new(data, info))
    }

    /// Get the parent node.
    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }
}

// ============================================================================
// RosSubscriber trait implementation
// ============================================================================

impl<T: TypeSupport> oxidros_core::api::RosSubscriber<T> for Subscriber<T> {
    fn topic_name(&self) -> Result<Cow<'_, String>> {
        Subscriber::topic_name(self)
    }

    async fn recv_msg(&mut self) -> Result<Message<T>> {
        self.recv().await
    }

    fn try_recv_msg(&mut self) -> Result<Option<Message<T>>> {
        self.try_recv()
    }
}
