//! Selector abstraction for event multiplexing in ROS2.
//!
//! This module provides traits for implementing event-driven architectures
//! where multiple sources (subscriptions, services, timers) can be waited on
//! simultaneously.

use crate::error::RCLResult;
use std::{collections::BTreeSet, time::Duration};

/// Result type for callback functions.
#[derive(Debug, Eq, PartialEq)]
pub enum CallbackResult {
    /// Callback executed successfully, keep it registered.
    Ok,

    /// Remove this callback from the selector.
    Remove,
}

/// Trait for entities that can be added to a selector and waited upon.
///
/// This represents any ROS2 entity that can generate events (subscriptions,
/// services, timers, etc.).
pub trait Waitable: Send {
    /// Returns a unique identifier for this waitable entity.
    fn id(&self) -> usize;

    /// Checks if this entity has data ready.
    fn is_ready(&self) -> bool;
}

/// Trait for subscription-like entities that receive messages.
pub trait SubscriptionLike<T>: Waitable {
    /// Attempts to take a message if one is available.
    fn take(&mut self) -> RCLResult<Option<T>>;
}

/// Trait for service-like entities that receive requests and send responses.
pub trait ServiceLike<Req, Resp>: Waitable {
    /// Attempts to take a request if one is available.
    fn take_request(&mut self) -> RCLResult<Option<(Req, RequestId)>>;

    /// Sends a response to a previous request.
    fn send_response(&mut self, request_id: RequestId, response: Resp) -> RCLResult<()>;
}

/// Opaque identifier for a service request.
///
/// This is used to match responses to their corresponding requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u64);

/// Trait for client-like entities that send requests and receive responses.
pub trait ClientLike<Req, Resp>: Waitable {
    /// Sends a request and returns a request ID.
    fn send_request(&mut self, request: Req) -> RCLResult<RequestId>;

    /// Attempts to take a response if one is available.
    fn take_response(&mut self) -> RCLResult<Option<(RequestId, Resp)>>;
}

/// Trait for timer entities that fire periodically or once.
pub trait TimerLike: Waitable {
    /// Returns the duration until the next timeout.
    fn time_until_trigger(&self) -> Duration;

    /// Resets the timer for the next period.
    fn reset(&mut self);

    /// Checks if this is a one-shot timer.
    fn is_oneshot(&self) -> bool;
}

/// Trait for guard conditions that can be manually triggered.
pub trait GuardConditionLike: Waitable {
    /// Triggers this guard condition.
    fn trigger(&self) -> RCLResult<()>;
}

/// Trait for a selector that multiplexes multiple event sources.
///
/// Implementations can vary in their async model (blocking, future-based, etc.).
pub trait SelectorLike {
    /// Waits for any registered entity to become ready.
    ///
    /// Returns the number of entities that became ready, or an error.
    fn wait(&mut self) -> RCLResult<usize>;

    /// Waits for any registered entity with a timeout.
    fn wait_timeout(&mut self, timeout: Duration) -> RCLResult<usize>;
}

/// Trait for adding subscriptions to a selector.
pub trait AddSubscription<T>: SelectorLike {
    /// The subscription type used by this implementation.
    type Subscription: SubscriptionLike<T>;

    /// Adds a subscription with a callback.
    fn add_subscription(
        &mut self,
        subscription: Self::Subscription,
        callback: Box<dyn FnMut(T) -> CallbackResult>,
    ) -> RCLResult<()>;
}

/// Trait for adding services to a selector.
pub trait AddService<Req, Resp>: SelectorLike {
    /// The service type used by this implementation.
    type Service: ServiceLike<Req, Resp>;

    /// Adds a service with a callback.
    fn add_service(
        &mut self,
        service: Self::Service,
        callback: Box<dyn FnMut(Req, RequestId) -> Resp>,
    ) -> RCLResult<()>;
}

/// Trait for adding timers to a selector.
pub trait AddTimer: SelectorLike {
    /// The timer type used by this implementation.
    type Timer: TimerLike;

    /// Adds a periodic timer with a callback.
    fn add_timer(
        &mut self,
        name: &str,
        period: Duration,
        callback: Box<dyn FnMut() -> CallbackResult>,
    ) -> RCLResult<u64>;

    /// Adds a one-shot timer with a callback.
    fn add_oneshot_timer(
        &mut self,
        duration: Duration,
        callback: Box<dyn FnOnce()>,
    ) -> RCLResult<u64>;
}

pub type ParameterServerCb = Box<dyn FnMut(&mut crate::parameter::Parameter, BTreeSet<String>)>;

/// Trait for adding parameter servers to a selector.
pub trait AddParameterServer: SelectorLike {
    /// The parameter server type used by this implementation.
    type ParameterServer;

    /// Adds a parameter server with a callback for parameter updates.
    fn add_parameter_server(
        &mut self,
        server: Self::ParameterServer,
        callback: ParameterServerCb,
    ) -> RCLResult<()>;
}

/// Statistics about selector performance.
#[cfg(feature = "statistics")]
#[derive(Debug, Clone)]
pub struct Statistics {
    /// Time spent waiting for events.
    pub wait_time: Duration,

    /// Time spent in callbacks.
    pub callback_time: Duration,

    /// Number of events processed.
    pub event_count: usize,
}
