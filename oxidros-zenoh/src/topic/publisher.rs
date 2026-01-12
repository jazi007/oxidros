//! Topic publisher.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Publishers](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#publishers)

use crate::{
    attachment::{Attachment, GID_SIZE, generate_gid},
    error::Result,
    keyexpr::{EntityKind, liveliness_entity_keyexpr, topic_keyexpr},
    node::Node,
    qos::QosMapping,
};
use oxidros_core::{TypeSupport, qos::Profile};
use std::{
    borrow::Cow,
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};
use zenoh::{Wait, bytes::ZBytes};
use zenoh_ext::AdvancedPublisherBuilderExt;

/// Topic publisher.
///
/// Publishes messages to a topic using Zenoh.
///
/// # Example
///
/// ```ignore
/// let publisher = node.create_publisher::<std_msgs::msg::String>("chatter", None)?;
///
/// let msg = std_msgs::msg::String { data: "Hello!".into() };
/// publisher.send(&msg)?;
/// ```
pub struct Publisher<T> {
    /// Parent node.
    node: Arc<Node>,
    /// Topic name.
    topic_name: String,
    /// Fully qualified topic name.
    fq_topic_name: String,
    /// Zenoh advanced publisher (supports cache for TRANSIENT_LOCAL durability).
    zenoh_publisher: zenoh_ext::AdvancedPublisher<'static>,
    /// Publisher GID.
    gid: [u8; GID_SIZE],
    /// Sequence number counter.
    sequence_number: AtomicI64,
    /// Liveliness token.
    _liveliness_token: zenoh::liveliness::LivelinessToken,
    /// Phantom data for type.
    _phantom: PhantomData<T>,
}

impl<T: TypeSupport> Publisher<T> {
    /// Create a new publisher.
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

        // Build key expression
        let key_expr_str = topic_keyexpr(
            node.context().domain_id(),
            fq_topic_name,
            type_name,
            &type_hash,
        );

        // Create Zenoh publisher
        let session = node.context().session();

        // Create an owned key expression
        let key_expr = zenoh::key_expr::KeyExpr::try_from(key_expr_str)?;

        // Build AdvancedPublisher with cache config based on durability QoS
        // For TRANSIENT_LOCAL: cache messages for late-joining subscribers
        // For VOLATILE: no cache (max_samples = 0)
        let cache_depth = if QosMapping::is_transient_local(&qos) {
            QosMapping::effective_depth(&qos)
        } else {
            0
        };

        let zenoh_publisher = session
            .declare_publisher(key_expr)
            .congestion_control(QosMapping::congestion_control(&qos))
            .cache(zenoh_ext::CacheConfig::default().max_samples(cache_depth))
            .wait()?;

        // Generate publisher GID
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

        Ok(Publisher {
            node,
            topic_name: topic_name.to_string(),
            fq_topic_name: fq_topic_name.to_string(),
            zenoh_publisher,
            gid,
            sequence_number: AtomicI64::new(0),
            _liveliness_token: liveliness_token,
            _phantom: PhantomData,
        })
    }
}

impl<T: TypeSupport> Publisher<T> {
    /// Get the topic name.
    pub fn topic_name(&self) -> Result<Cow<'_, String>> {
        Ok(Cow::Borrowed(&self.topic_name))
    }

    /// Get the fully qualified topic name.
    pub fn fully_qualified_topic_name(&self) -> &str {
        &self.fq_topic_name
    }

    /// Get the publisher GID.
    pub fn gid(&self) -> &[u8; GID_SIZE] {
        &self.gid
    }

    /// Publish a message.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the Zenoh put fails.
    pub fn send(&self, msg: &T) -> Result<()> {
        // Serialize message to CDR
        let payload = msg.to_bytes()?;

        // Increment sequence number
        let seq = self.sequence_number.fetch_add(1, Ordering::Relaxed);

        // Create attachment
        let attachment = Attachment::new(seq, self.gid);
        let attachment_bytes = attachment.to_bytes();

        // Publish with attachment
        self.zenoh_publisher
            .put(payload)
            .attachment(ZBytes::from(attachment_bytes.to_vec()))
            .wait()?;

        Ok(())
    }

    /// Get the parent node.
    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }
}

// ============================================================================
// RosPublisher trait implementation
// ============================================================================

impl<T: TypeSupport> oxidros_core::api::RosPublisher<T> for Publisher<T> {
    fn topic_name(&self) -> Result<Cow<'_, String>> {
        Publisher::topic_name(self)
    }

    fn publish(&self, msg: &T) -> crate::error::Result<()> {
        self.send(msg)
    }
}
