//! Common types used by all ROS nodes
//!
use futures::ready;
use oxidros::{
    msg::TypeSupport,
    node::Node,
    qos::{
        policy::{DurabilityPolicy, HistoryPolicy, ReliabilityPolicy},
        Profile,
    },
    topic::subscriber::{Subscriber, TakenMsg},
};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio_stream::Stream;
use tokio_util::sync::ReusableBoxFuture;

/// Type aliases for ROS2 `Arc<Node>`
pub type ArcNode = Arc<Node>;

/// Re-export of safe drive DynError
pub use oxidros::error::DynError;
/// Result Type for ROS2
pub type Result<T> = std::result::Result<T, DynError>;
/// Publisher attributes
#[derive(Debug, Default, Clone)]
pub struct Attributes<'a> {
    /// Topic name
    pub name: &'a str,
    /// QoS profile
    pub qos: Option<Profile>,
}

impl<'a> Attributes<'a> {
    /// Create new attributes
    pub const fn new(name: &'a str) -> Self {
        Self { name, qos: None }
    }
    /// set QoS
    pub fn with(mut self, qos: Option<Profile>) -> Self {
        self.qos = qos;
        self
    }
}

/// Create a QoS profile
#[inline(always)]
pub fn create_qos(depth: usize) -> Profile {
    Profile {
        depth,
        history: HistoryPolicy::KeepLast,
        reliability: ReliabilityPolicy::Reliable,
        durability: DurabilityPolicy::Volatile,
        ..Profile::default()
    }
}

/// Implement a wrapper for [`Subscriber`] that implement Stream
pub type RecvResult<T> = Result<TakenMsg<T>>;

/// Subscriber Wrapper to make a Stream
#[derive(Debug)]
pub struct SubscriberStream<T> {
    inner: ReusableBoxFuture<'static, (RecvResult<T>, Subscriber<T>)>,
}

async fn make_future<T: TypeSupport>(mut rx: Subscriber<T>) -> (RecvResult<T>, Subscriber<T>) {
    let result = rx.recv().await;
    (result, rx)
}

impl<T: 'static + Send + TypeSupport> SubscriberStream<T> {
    /// Create a new `BroadcastStream`.
    pub fn new(rx: Subscriber<T>) -> Self {
        Self {
            inner: ReusableBoxFuture::new(make_future(rx)),
        }
    }
}

impl<T: 'static + Send + TypeSupport> Stream for SubscriberStream<T> {
    type Item = RecvResult<T>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (result, rx) = ready!(self.inner.poll(cx));
        self.inner.set(make_future(rx));
        match result {
            Ok(item) => Poll::Ready(Some(Ok(item))),
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}

/// Get ROS Time
pub fn get_time_now() -> Result<Duration> {
    let mut time = oxidros::clock::Clock::new()?;
    Ok(Duration::from_nanos(time.get_now()?.try_into()?))
}
