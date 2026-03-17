//! Topic types: publishers and subscribers.

/// Publisher module.
pub mod publisher {
    #[cfg(feature = "rcl")]
    pub use oxidros_wrapper::Publisher;

    #[cfg(feature = "zenoh")]
    pub use oxidros_zenoh::topic::publisher::Publisher;
}

/// Subscriber module.
pub mod subscriber {
    #[cfg(feature = "rcl")]
    pub use oxidros_wrapper::{Subscriber, SubscriberStream};

    #[cfg(feature = "zenoh")]
    pub use oxidros_zenoh::topic::subscriber::{Subscriber, SubscriberStream};
}
