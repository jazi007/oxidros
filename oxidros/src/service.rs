//! Service types: clients and servers.

/// Client module.
pub mod client {
    #[cfg(feature = "rcl")]
    pub use oxidros_wrapper::Client;

    #[cfg(feature = "zenoh")]
    pub use oxidros_zenoh::service::client::Client;
}

/// Server module.
pub mod server {
    #[cfg(feature = "rcl")]
    pub use oxidros_wrapper::{Server, ServiceRequest};

    #[cfg(feature = "zenoh")]
    pub use oxidros_zenoh::service::server::{Server, ServiceRequest};
}
