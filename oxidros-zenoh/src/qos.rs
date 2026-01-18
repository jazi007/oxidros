//! QoS mapping from oxidros-core to Zenoh.
//!
//! # Reference
//!
//! This module implements QoS mapping as specified in:
//! <https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#quality-of-service>
//!
//! ## Reliability Mapping
//!
//! | ROS2 QoS | Zenoh Behavior |
//! |----------|----------------|
//! | `Reliable` | Data delivery via reliable transport (tcp/quic) if configured |
//! | `BestEffort` | May use unreliable transport (udp) if configured, otherwise reliable |
//! | `SystemDefault` | Same as `BestEffort` |
//!
//! ## History Mapping
//!
//! | ROS2 QoS | Zenoh Behavior |
//! |----------|----------------|
//! | `KeepLast(n)` | For subscriptions: queue depth = n. For publishers with TRANSIENT_LOCAL: cache size = n |
//! | `KeepAll` | For publishers with Reliable: `CongestionControl::Block` mode |
//! | `SystemDefault` | Same as `KeepLast` |
//!
//! ## Depth Mapping
//!
//! | ROS2 QoS | Zenoh Behavior |
//! |----------|----------------|
//! | `0` | Defaults to `42` (per rmw_zenoh specification) |
//! | `n > 0` | Uses specified depth |
//!
//! ## Durability Mapping
//!
//! | ROS2 QoS | Zenoh Behavior |
//! |----------|----------------|
//! | `Volatile` | Standard Publisher/Subscriber, no caching |
//! | `TransientLocal` | Publisher: `AdvancedPublisher` with cache. Subscriber: `AdvancedSubscriber` with historical query |
//! | `SystemDefault` | Same as `Volatile` |
//!
//! ## Liveliness Mapping
//!
//! | ROS2 QoS | Zenoh Behavior |
//! |----------|----------------|
//! | `Automatic` | Managed by RMW layer (only supported mode) |
//! | `ManualByTopic` | **Not supported** |
//!
//! ## Unsupported QoS Policies
//!
//! The following policies are currently unimplemented in rmw_zenoh:
//! - `Deadline`
//! - `Lifespan`
//!
//! ## QoS Compatibility Note
//!
//! Unlike DDS, Zenoh has no "incompatible" QoS settings. Any publisher can match
//! any subscription as long as their key expressions match. Type safety is enforced
//! through the type name and hash in the key expression.

use oxidros_core::qos::{DurabilityPolicy, HistoryPolicy, Profile, ReliabilityPolicy};
use zenoh::qos::CongestionControl;

/// Default depth when QoS depth is 0 (per rmw_zenoh specification).
pub const DEFAULT_DEPTH: usize = 42;

/// QoS mapping utilities.
pub struct QosMapping;

impl QosMapping {
    /// Get the effective queue/cache depth.
    ///
    /// Returns `DEFAULT_DEPTH` (42) if the profile depth is 0,
    /// otherwise returns the specified depth.
    pub fn effective_depth(profile: &Profile) -> usize {
        let depth = match profile.history {
            HistoryPolicy::KeepAll => usize::MAX,
            _ => profile.depth,
        };
        if depth == 0 {
            DEFAULT_DEPTH
        } else {
            profile.depth
        }
    }

    /// Check if the profile requires transient local durability.
    ///
    /// Returns `true` if durability is `TransientLocal`.
    pub fn is_transient_local(profile: &Profile) -> bool {
        matches!(profile.durability, DurabilityPolicy::TransientLocal)
    }

    /// Check if the profile requires reliable delivery.
    ///
    /// Returns `true` if reliability is `Reliable` or `SystemDefault`.
    pub fn is_reliable(profile: &Profile) -> bool {
        matches!(
            profile.reliability,
            ReliabilityPolicy::Reliable | ReliabilityPolicy::SystemDefault
        )
    }

    /// Get the Zenoh congestion control mode for a publisher.
    ///
    /// Returns `Block` if history is `KeepAll` and reliability is `Reliable`,
    /// otherwise returns `Drop`.
    ///
    /// # Reference
    ///
    /// Per rmw_zenoh design:
    /// > `KeepAll`: For publishers, if the `RELIABILITY` is `RELIABLE`, the
    /// > `CongestionControl::BLOCK` mode is set, meaning the publisher will be
    /// > blocked when network congestion occurs.
    pub fn congestion_control(profile: &Profile) -> CongestionControl {
        if matches!(profile.history, HistoryPolicy::KeepAll) && Self::is_reliable(profile) {
            CongestionControl::Block
        } else {
            CongestionControl::Drop
        }
    }

    /// Validate QoS profile for supported features.
    ///
    /// Logs warnings for unsupported QoS settings.
    pub fn validate(profile: &Profile) {
        use oxidros_core::qos::LivelinessPolicy;

        // Warn about unsupported liveliness
        if matches!(profile.liveliness, LivelinessPolicy::ManualByTopic) {
            tracing::warn!(
                "QoS liveliness ManualByTopic is not supported by rmw_zenoh, using Automatic"
            );
        }

        // Warn about deadline/lifespan (not implemented)
        if !profile.deadline.is_zero() {
            tracing::warn!("QoS deadline is not implemented in rmw_zenoh, ignoring");
        }

        if !profile.lifespan.is_zero() {
            tracing::warn!("QoS lifespan is not implemented in rmw_zenoh, ignoring");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_depth() {
        let mut profile = Profile {
            depth: 0,
            ..Default::default()
        };
        // Default depth of 0 should return DEFAULT_DEPTH
        assert_eq!(QosMapping::effective_depth(&profile), DEFAULT_DEPTH);

        // Non-zero depth should be returned as-is
        profile.depth = 10;
        assert_eq!(QosMapping::effective_depth(&profile), 10);
    }

    #[test]
    fn test_transient_local() {
        let mut profile = Profile {
            durability: DurabilityPolicy::Volatile,
            ..Default::default()
        };
        assert!(!QosMapping::is_transient_local(&profile));
        profile.durability = DurabilityPolicy::TransientLocal;
        assert!(QosMapping::is_transient_local(&profile));
    }

    #[test]
    fn test_congestion_control() {
        let mut profile = Profile::default();

        // Default should be Drop
        assert_eq!(
            QosMapping::congestion_control(&profile),
            CongestionControl::Drop
        );

        // KeepAll + Reliable = Block
        profile.history = HistoryPolicy::KeepAll;
        profile.reliability = ReliabilityPolicy::Reliable;
        assert_eq!(
            QosMapping::congestion_control(&profile),
            CongestionControl::Block
        );

        // KeepAll + BestEffort = Drop
        profile.reliability = ReliabilityPolicy::BestEffort;
        assert_eq!(
            QosMapping::congestion_control(&profile),
            CongestionControl::Drop
        );
    }
}
