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
use oxidros_core::qos::Profile;
use parking_lot::Mutex;
use ros2_types::{TypeDescription, TypeSupport};
use std::{marker::PhantomData, sync::Arc};
use zenoh::{Wait, bytes::ZBytes};

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
    /// Zenoh publisher.
    zenoh_publisher: zenoh::pubsub::Publisher<'static>,
    /// Publisher GID.
    gid: [u8; GID_SIZE],
    /// Sequence number counter.
    sequence_number: Mutex<i64>,
    /// Liveliness token.
    _liveliness_token: zenoh::liveliness::LivelinessToken,
    /// Phantom data for type.
    _phantom: PhantomData<T>,
}

impl<T: TypeSupport + TypeDescription> Publisher<T> {
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
        let type_hash = T::compute_hash()?;

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
        let mut builder = session.declare_publisher(key_expr);

        // Apply QoS settings
        builder = builder.congestion_control(QosMapping::congestion_control(&qos));

        let zenoh_publisher = builder.wait()?;

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
            node.namespace(),
            node.name(),
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
            sequence_number: Mutex::new(0),
            _liveliness_token: liveliness_token,
            _phantom: PhantomData,
        })
    }
}

impl<T: TypeSupport> Publisher<T> {
    /// Get the topic name.
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Get the fully qualified topic name.
    pub fn fq_topic_name(&self) -> &str {
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
        let seq = {
            let mut seq = self.sequence_number.lock();
            let current = *seq;
            *seq += 1;
            current
        };

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
