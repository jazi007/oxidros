//! Event selector for Zenoh-based ROS2 operations.
//!
//! Provides a unified way to wait on multiple ROS2 entities (subscribers, servers, timers)
//! and dispatch callbacks when events occur.

use crate::{
    error::Result, parameter::ParameterServer as ZenohParameterServer, service::server::Server,
    topic::subscriber::Subscriber,
};
use oxidros_core::{TypeSupport, message::TakenMsg, parameter::Parameters};
use std::{
    collections::{BTreeSet, HashMap},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// Timer ID counter.
static TIMER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A timer entry.
struct Timer {
    period: Duration,
    next_fire: Instant,
    handler: Box<dyn FnMut()>,
}

/// Event selector for Zenoh operations.
///
/// The Selector allows you to wait on multiple entities and receive callbacks
/// when events occur. This is the primary mechanism for single-threaded
/// event-driven ROS2 applications.
///
/// # Example
///
/// ```ignore
/// let mut selector = Selector::new(ctx);
///
/// // Add a subscriber
/// selector.add_subscriber(subscriber, Box::new(|msg| {
///     println!("Received: {:?}", msg);
/// }));
///
/// // Add a timer
/// selector.add_timer(Duration::from_secs(1), Box::new(|| {
///     println!("Timer fired!");
/// }));
///
/// // Main loop
/// loop {
///     selector.wait()?;
/// }
/// ```
pub struct Selector {
    /// Subscriber handlers that poll and process messages.
    subscriber_handlers: Vec<Box<dyn FnMut() -> bool>>,
    /// Timers with their next fire time.
    timers: HashMap<u64, Timer>,
}

impl Selector {
    /// Create a new selector.
    pub fn new() -> Self {
        Self {
            subscriber_handlers: Vec::new(),
            timers: HashMap::new(),
        }
    }

    /// Add a subscriber with a callback handler.
    ///
    /// The handler will be called whenever a message arrives on the topic.
    pub fn add_subscriber<T: TypeSupport + 'static>(
        &mut self,
        mut subscriber: Subscriber<T>,
        mut handler: Box<dyn FnMut(TakenMsg<T>)>,
    ) -> bool {
        // Create a closure that tries to receive and call the handler
        let poll_fn = Box::new(move || -> bool {
            match subscriber.try_recv() {
                Ok(Some(msg)) => {
                    handler(TakenMsg::Copied(msg.data));
                    true
                }
                _ => false,
            }
        });
        self.subscriber_handlers.push(poll_fn);
        true
    }

    /// Add a timer with the given period.
    ///
    /// Returns a timer ID that can be used to remove the timer.
    pub fn add_timer(&mut self, period: Duration, handler: Box<dyn FnMut()>) -> u64 {
        let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timer = Timer {
            period,
            next_fire: Instant::now() + period,
            handler,
        };
        self.timers.insert(id, timer);
        id
    }

    /// Add a wall timer (alias for add_timer in Zenoh).
    pub fn add_wall_timer(
        &mut self,
        _name: &str,
        period: Duration,
        handler: Box<dyn FnMut()>,
    ) -> u64 {
        self.add_timer(period, handler)
    }

    /// Remove a timer by ID.
    pub fn remove_timer(&mut self, id: u64) {
        self.timers.remove(&id);
    }

    /// Wait for events indefinitely.
    pub fn wait(&mut self) -> Result<()> {
        self.wait_timeout_internal(None)
    }

    /// Wait for events with a timeout.
    ///
    /// Returns `Ok(true)` if events were processed, `Ok(false)` if timeout occurred.
    pub fn wait_timeout(&mut self, timeout: Duration) -> Result<bool> {
        self.wait_timeout_internal(Some(timeout)).map(|_| true)
    }

    fn wait_timeout_internal(&mut self, timeout: Option<Duration>) -> Result<()> {
        let start = Instant::now();
        let deadline = timeout.map(|t| start + t);
        let poll_interval = Duration::from_millis(10);

        loop {
            // Poll all subscribers
            for handler in &mut self.subscriber_handlers {
                handler();
            }

            // Process expired timers
            let now = Instant::now();
            for timer in self.timers.values_mut() {
                if now >= timer.next_fire {
                    (timer.handler)();
                    timer.next_fire = now + timer.period;
                }
            }

            // Check if we've exceeded the timeout
            if let Some(d) = deadline
                && Instant::now() >= d
            {
                break;
            }

            // Calculate sleep time
            let next_timer = self.timers.values().map(|t| t.next_fire).min();
            let sleep_until = match (deadline, next_timer) {
                (Some(d), Some(t)) => Some(d.min(t)),
                (Some(d), None) => Some(d),
                (None, Some(t)) => Some(t),
                (None, None) => None,
            };

            let sleep_time = sleep_until
                .map(|s| s.saturating_duration_since(Instant::now()))
                .map(|d| d.min(poll_interval))
                .unwrap_or(poll_interval);

            if sleep_time > Duration::ZERO {
                std::thread::sleep(sleep_time);
            }

            // For indefinite wait with no timers, break after one iteration
            // to avoid infinite busy-loop when there's nothing to do
            if deadline.is_none() && self.timers.is_empty() && self.subscriber_handlers.is_empty() {
                break;
            }
        }

        Ok(())
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

// Stub types for unsupported features
/// Stub action server (not supported in Zenoh).
pub struct ActionServer<T>(std::marker::PhantomData<T>);
/// Stub action client (not supported in Zenoh).
pub struct ActionClient<T>(std::marker::PhantomData<T>);
/// Stub action goal handle (not supported in Zenoh).
pub struct ActionGoalHandle<T>(std::marker::PhantomData<T>);

impl oxidros_core::api::RosSelector for Selector {
    type Subscriber<T: TypeSupport + 'static> = Subscriber<T>;
    type Server<T: oxidros_core::ServiceMsg + 'static> = Server<T>;
    type ActionServer<T: oxidros_core::ActionMsg> = ActionServer<T>;
    type ActionClient<T: oxidros_core::ActionMsg> = ActionClient<T>;
    type ActionGoalHandle<T: oxidros_core::ActionMsg> = ActionGoalHandle<T>;
    type ParameterServer = ZenohParameterServer;

    fn add_subscriber_handler<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Self::Subscriber<T>,
        handler: Box<dyn FnMut(TakenMsg<T>)>,
    ) -> bool {
        self.add_subscriber(subscriber, handler)
    }

    fn add_server_handler<T: oxidros_core::ServiceMsg + 'static>(
        &mut self,
        _server: Self::Server<T>,
        _handler: Box<dyn FnMut(T::Request) -> T::Response>,
    ) -> bool {
        // Server handling in Zenoh is done via async patterns
        // For now, just return true
        true
    }

    fn add_parameter_server_handler(
        &mut self,
        _param_server: Self::ParameterServer,
        _handler: Box<dyn FnMut(&mut Parameters, BTreeSet<String>)>,
    ) {
        // Parameter server handling is different in Zenoh
    }

    fn add_timer_handler(&mut self, duration: Duration, handler: Box<dyn FnMut()>) -> u64 {
        self.add_timer(duration, handler)
    }

    fn add_wall_timer_handler(
        &mut self,
        name: &str,
        period: Duration,
        handler: Box<dyn FnMut()>,
    ) -> u64 {
        self.add_wall_timer(name, period, handler)
    }

    fn delete_timer(&mut self, id: u64) {
        self.remove_timer(id)
    }

    fn add_action_server_handler<T, GR, A, CR>(
        &mut self,
        _server: Self::ActionServer<T>,
        _goal_handler: GR,
        _accept_handler: A,
        _cancel_handler: CR,
    ) -> oxidros_core::Result<bool>
    where
        T: oxidros_core::ActionMsg + 'static,
        GR: Fn(&<<T as oxidros_core::ActionMsg>::Goal as oxidros_core::ActionGoal>::Request) -> bool
            + 'static,
        A: Fn(Self::ActionGoalHandle<T>) + 'static,
        CR: Fn(&[u8; 16]) -> bool + 'static,
    {
        Err(oxidros_core::Error::NotImplemented {
            feature: "action_server".into(),
            reason: "Zenoh backend does not support actions yet".into(),
        })
    }

    fn add_action_client_handler<T: oxidros_core::ActionMsg + 'static>(
        &mut self,
        _client: Self::ActionClient<T>,
    ) -> oxidros_core::Result<bool> {
        Err(oxidros_core::Error::NotImplemented {
            feature: "action_client".into(),
            reason: "Zenoh backend does not support actions yet".into(),
        })
    }

    fn spin_once(&mut self) -> oxidros_core::Result<()> {
        self.wait()
    }

    fn spin_timeout(&mut self, timeout: Duration) -> oxidros_core::Result<bool> {
        self.wait_timeout(timeout)
    }
}
