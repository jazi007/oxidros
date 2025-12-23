//! ROS2 comms to make working with oxidros more easy
//!
//! # Example
//! ```ignore
//! use std::time::Duration;
//!
//! use oxidros_wrapper::prelude::*;
//! use oxidros_wrapper::oxidros::msg::common_interfaces::std_msgs;
//! use tokio::{runtime::Builder, signal::ctrl_c, time::interval};
//!
//! static NAME: &str = "EX1";
//!
//! async fn ros2_main() -> Result<()> {
//!     // Create a context.
//!     let ctx = Context::new()?;
//!     // Create a node.
//!     let node = ctx.create_node(NAME, None, Default::default())?;
//!     let publisher = node.new_publisher::<std_msgs::msg::String>(&Attributes::new("ex1_pub"))?;
//!     let mut subscriber = node
//!         .new_subscriber::<std_msgs::msg::String>(&Attributes::new("ex2_pub"))?
//!         .into_stream();
//!     let mut counter: usize = 0;
//!     let mut interval = interval(Duration::from_millis(100));
//!     loop {
//!         tokio::select! {
//!             msg = subscriber.next() => {
//!                 let Some(Ok(v)) = msg else {
//!                     continue;
//!                 };
//!                 println!("Received message {:?}", v.data.get_string());
//!                 let mut message = std_msgs::msg::String::new().unwrap();
//!                 message.data.assign(&format!("{} -> {}", NAME, counter));
//!                 println!("Sending: {:?}", message.data.get_string());
//!                 publisher.send_many([&message, &message])?;
//!                 counter = counter.wrapping_add(1);
//!             },
//!             elapsed = interval.tick() => {
//!                 println!("elapsed : {elapsed:?}");
//!             },
//!             _ = ctrl_c() => {
//!                 break;
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//!
//! fn main() -> Result<()> {
//!     let rt = Builder::new_multi_thread()
//!         .thread_name(NAME)
//!         .enable_all()
//!         .build()
//!         .unwrap();
//!     rt.block_on(ros2_main())
//! }
//! ```
#![deny(
    unsafe_code,
    missing_docs,
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::expect_used
)]

#[macro_use]
pub mod common;
pub mod clientserver;
pub mod communicate;
pub mod logger;
pub mod params_helpers;
pub mod pubsub;

/// prelude module
pub mod prelude {
    pub use crate::clientserver::{Client, Server};
    pub use crate::common::{create_qos, ArcNode, Attributes, DynError, Result};
    pub use crate::communicate::Communicate;
    pub use crate::pubsub::{Publish, Subscribe};
    pub use oxidros::{
        context::Context,
        node::{Node, NodeOptions},
        topic::{
            publisher::Publisher,
            subscriber::{Subscriber, TakenMsg},
        },
    };
    pub use tokio_stream::{Stream, StreamExt};
}
pub use oxidros;
pub use tokio;
pub use tokio_stream;
