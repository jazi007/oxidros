//! Pub/Sub implementation for oxidros publisher and subscriber
//!
use std::pin::Pin;

use oxidros::{
    msg::TypeSupport,
    oxidros_rcl::RecvResult,
    topic::{
        publisher::Publisher,
        subscriber::{Subscriber, TakenMsg},
    },
};
use tokio_stream::Stream;

use crate::common::Result;
use crate::common::SubscriberStream;

/// Type alias for pinned streams
pub type MessageStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

/// Publish trait
pub trait Publish<T>: Send + Sync {
    /// publish a batch of messages
    fn send_many<'a>(&self, messages: impl IntoIterator<Item = &'a T>) -> Result<()>
    where
        T: 'a;
}

/// Subscribe trait
pub trait Subscribe<T>: Send {
    /// receive messages
    fn recv_many(&self, limit: usize) -> Result<Vec<TakenMsg<T>>>;
    /// stream for receiving messages
    fn into_stream(self) -> MessageStream<Result<TakenMsg<T>>>;
}
impl<T: TypeSupport> Publish<T> for Publisher<T> {
    fn send_many<'a>(&self, messages: impl IntoIterator<Item = &'a T>) -> Result<()>
    where
        T: 'a,
    {
        for msg in messages.into_iter() {
            self.send(msg)?;
        }
        Ok(())
    }
}

impl<T: TypeSupport + Send + 'static> Subscribe<T> for Subscriber<T> {
    fn recv_many(&self, limit: usize) -> Result<Vec<TakenMsg<T>>> {
        let mut results = if limit == usize::MAX {
            Vec::new()
        } else {
            Vec::with_capacity(limit)
        };
        while results.len() < limit {
            match self.try_recv() {
                RecvResult::Ok(msg) => results.push(msg),
                RecvResult::Err(e) => return Err(e),
                RecvResult::RetryLater => break,
            }
        }

        Ok(results)
    }
    fn into_stream(self) -> MessageStream<Result<TakenMsg<T>>> {
        Box::pin(SubscriberStream::new(self))
    }
}
