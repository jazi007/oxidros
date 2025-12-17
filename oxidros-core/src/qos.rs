//! Quality of Service (QoS) policies and profiles for ROS2.

use std::time::Duration;

/// QoS history policy - how samples are stored.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HistoryPolicy {
    /// Implementation default for history policy.
    SystemDefault,

    /// Only store up to a maximum number of samples, dropping oldest once max is exceeded.
    KeepLast,

    /// Store all samples, subject to resource limits.
    KeepAll,

    /// History policy has not yet been set.
    Unknown,
}

/// QoS reliability policy - how messages are delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliabilityPolicy {
    /// Implementation specific default.
    SystemDefault,

    /// Guarantee that samples are delivered, may retry multiple times.
    Reliable,

    /// Attempt to deliver samples, but some may be lost if the network is not robust.
    BestEffort,

    /// Reliability policy has not yet been set.
    Unknown,
}

/// QoS durability policy - how samples persist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityPolicy {
    /// Implementation specific default.
    SystemDefault,

    /// The publisher is responsible for persisting samples for "late-joining" subscribers.
    TransientLocal,

    /// Samples are not persistent.
    Volatile,

    /// Durability policy has not yet been set.
    Unknown,
}

/// QoS liveliness policy - a publisher's reporting policy for its alive status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivelinessPolicy {
    /// Implementation specific default.
    SystemDefault,

    /// The signal that establishes a topic is alive comes from the ROS layer.
    Automatic,

    /// The signal that establishes a topic is alive is at the topic level.
    /// Only publishing a message on the topic or an explicit signal from the
    /// application to assert liveliness on the topic will mark the topic as being alive.
    ManualByTopic,

    /// Liveliness policy has not yet been set.
    Unknown,
}

/// Represents a QoS profile.
///
/// QoS profiles control the behavior of publishers and subscribers,
/// including reliability, durability, and other communication characteristics.
#[derive(Debug, Clone)]
pub struct Profile {
    /// Keep last: only store up to N samples, configurable via the queue depth option.
    /// Keep all: store all samples, subject to the configured resource limits.
    pub history: HistoryPolicy,

    /// Size of the message queue.
    pub depth: usize,

    /// Reliability QoS policy setting.
    pub reliability: ReliabilityPolicy,

    /// Durability QoS policy setting.
    pub durability: DurabilityPolicy,

    /// The period at which messages are expected to be sent/received.
    /// Zero duration means unspecified (use implementation default).
    pub deadline: Duration,

    /// The age at which messages are considered expired and no longer valid.
    /// Zero duration means unspecified (use implementation default).
    pub lifespan: Duration,

    /// Liveliness QoS policy setting.
    pub liveliness: LivelinessPolicy,

    /// The time within which the node or publisher must show that it is alive.
    /// Zero duration means unspecified (use implementation default).
    pub liveliness_lease_duration: Duration,

    /// If true, any ROS specific namespacing conventions will be circumvented.
    pub avoid_ros_namespace_conventions: bool,
}

impl Default for Profile {
    /// Default QoS profile:
    /// - History: Keep last
    /// - Depth: 10
    /// - Reliability: Reliable
    /// - Durability: Volatile
    /// - Deadline: Default (zero)
    /// - Lifespan: Default (zero)
    /// - Liveliness: System default
    /// - Liveliness lease duration: Default (zero)
    /// - Avoid ROS namespace conventions: false
    fn default() -> Self {
        Self {
            history: HistoryPolicy::KeepLast,
            depth: 10,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            deadline: Duration::ZERO,
            lifespan: Duration::ZERO,
            liveliness: LivelinessPolicy::SystemDefault,
            liveliness_lease_duration: Duration::ZERO,
            avoid_ros_namespace_conventions: false,
        }
    }
}

impl Profile {
    /// Services QoS profile:
    /// - History: Keep last
    /// - Depth: 10
    /// - Reliability: Reliable
    /// - Durability: Volatile
    pub fn services_default() -> Self {
        Self {
            history: HistoryPolicy::KeepLast,
            depth: 10,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            deadline: Duration::ZERO,
            lifespan: Duration::ZERO,
            liveliness: LivelinessPolicy::SystemDefault,
            liveliness_lease_duration: Duration::ZERO,
            avoid_ros_namespace_conventions: false,
        }
    }

    /// Sensor Data QoS profile:
    /// - History: Keep last
    /// - Depth: 5
    /// - Reliability: Best effort
    /// - Durability: Volatile
    pub const fn sensor_data() -> Self {
        Self {
            history: HistoryPolicy::KeepLast,
            depth: 5,
            reliability: ReliabilityPolicy::BestEffort,
            durability: DurabilityPolicy::Volatile,
            deadline: Duration::ZERO,
            lifespan: Duration::ZERO,
            liveliness: LivelinessPolicy::SystemDefault,
            liveliness_lease_duration: Duration::ZERO,
            avoid_ros_namespace_conventions: false,
        }
    }

    /// Parameters QoS profile:
    /// - History: Keep last
    /// - Depth: 1000
    /// - Reliability: Reliable
    /// - Durability: Volatile
    pub const fn parameters() -> Self {
        Self {
            history: HistoryPolicy::KeepLast,
            depth: 1000,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::Volatile,
            deadline: Duration::ZERO,
            lifespan: Duration::ZERO,
            liveliness: LivelinessPolicy::SystemDefault,
            liveliness_lease_duration: Duration::ZERO,
            avoid_ros_namespace_conventions: false,
        }
    }
}
