//! Tracing support and target constants for oxidros.
//!
//! This module provides standardized tracing targets for consistent
//! telemetry across all oxidros crates.
//!
//! # Tracing Targets
//!
//! Use these constants with tracing macros:
//!
//! ```ignore
//! use oxidros_core::targets;
//!
//! tracing::debug!(target: targets::PUBLISHER, topic = %topic, "Message published");
//! tracing::debug!(target: targets::SUBSCRIBER, topic = %topic, "Message received");
//! ```
//!
//! Configure subscribers to filter by target:
//!
//! ```ignore
//! use oxidros_core::targets;
//! use tracing_subscriber::EnvFilter;
//!
//! let filter = EnvFilter::new(format!("{}=debug", targets::PUBLISHER));
//! tracing_subscriber::fmt()
//!     .with_env_filter(filter)
//!     .init();
//! ```
//!
//! # Target Hierarchy
//!
//! - `oxidros` - Root target for all oxidros crates
//! - `oxidros::publisher` - Publisher operations
//! - `oxidros::subscriber` - Subscriber operations  
//! - `oxidros::service` - Service client/server operations
//! - `oxidros::action` - Action client/server operations
//! - `oxidros::selector` - Selector/spin operations
//! - `oxidros::timer` - Timer operations
//! - `oxidros::rcl` - RCL-specific operations
//! - `oxidros::zenoh` - Zenoh-specific operations
//! - `oxidros::zenoh::publisher` - Zenoh publisher operations
//! - `oxidros::zenoh::subscriber` - Zenoh subscriber operations
//! - `oxidros::zenoh::service` - Zenoh service operations

/// Tracing target constants for consistent naming across crates.
pub mod targets {
    /// Root target for general oxidros operations.
    pub const ROOT: &str = "oxidros";

    /// Target for publisher operations.
    pub const PUBLISHER: &str = "oxidros::publisher";

    /// Target for subscriber operations.
    pub const SUBSCRIBER: &str = "oxidros::subscriber";

    /// Target for service client/server operations.
    pub const SERVICE: &str = "oxidros::service";

    /// Target for action client/server operations.
    pub const ACTION: &str = "oxidros::action";

    /// Target for selector/spin loop operations.
    pub const SELECTOR: &str = "oxidros::selector";

    /// Target for timer operations.
    pub const TIMER: &str = "oxidros::timer";

    /// Target for RCL backend operations.
    pub const RCL: &str = "oxidros::rcl";

    /// Target for Zenoh backend operations.
    pub const ZENOH: &str = "oxidros::zenoh";

    /// Target for Zenoh publisher operations.
    pub const ZENOH_PUBLISHER: &str = "oxidros::zenoh::publisher";

    /// Target for Zenoh subscriber operations.
    pub const ZENOH_SUBSCRIBER: &str = "oxidros::zenoh::subscriber";

    /// Target for Zenoh service operations.
    pub const ZENOH_SERVICE: &str = "oxidros::zenoh::service";

    /// Target for node lifecycle operations.
    pub const NODE: &str = "oxidros::node";

    /// Target for context operations.
    pub const CONTEXT: &str = "oxidros::context";

    /// Target for parameter operations.
    pub const PARAMETER: &str = "oxidros::parameter";
}
