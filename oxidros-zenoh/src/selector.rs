//! Event selector for Zenoh-based ROS2 operations.
//!
//! Provides a unified way to wait on multiple ROS2 entities (subscribers, servers, timers)
//! and dispatch callbacks when events occur.

use crate::{
    error::Result, parameter::ParameterServer as ZenohParameterServer, service::server::Server,
    topic::subscriber::Subscriber,
};
use oxidros_core::{Message, TypeSupport, parameter::Parameters};
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
    /// If true, the timer fires once and is removed.
    one_shot: bool,
}

/// Callback type for parameter server updates.
type ParameterServerCallback = Box<dyn FnMut(&mut Parameters, BTreeSet<String>)>;

/// Event selector for Zenoh operations.
///
/// The Selector allows you to wait on multiple entities and receive callbacks
/// when events occur. This is the primary mechanism for single-threaded
/// event-driven ROS2 applications.
///
/// # Example
///
/// ```ignore
/// // Create a context.
/// let ctx = Context::new().unwrap();
///
/// let mut selector = ctx.create_selector().unwrap();
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
    /// Server handlers that poll and process requests.
    server_handlers: Vec<Box<dyn FnMut() -> bool>>,
    /// Parameter server handler (only one per Selector).
    parameter_server_handler: Option<Box<dyn FnMut() -> bool>>,
    /// Timers with their next fire time.
    timers: HashMap<u64, Timer>,
}

impl Selector {
    /// Create a new selector.
    pub(crate) fn new() -> Self {
        Self {
            subscriber_handlers: Vec::new(),
            server_handlers: Vec::new(),
            parameter_server_handler: None,
            timers: HashMap::new(),
        }
    }

    /// Add a subscriber with a callback handler.
    ///
    /// The handler will be called whenever a message arrives on the topic.
    pub fn add_subscriber<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Subscriber<T>,
        mut handler: Box<dyn FnMut(Message<T>)>,
    ) -> bool {
        // Create a closure that tries to receive and call the handler
        let poll_fn = Box::new(move || -> bool {
            match subscriber.try_recv() {
                Ok(Some(msg)) => {
                    handler(msg);
                    true
                }
                _ => false,
            }
        });
        self.subscriber_handlers.push(poll_fn);
        true
    }

    /// Add a service server to the selector.
    ///
    /// The handler receives the request message and returns a response.
    /// Incoming requests are polled during `wait()` calls.
    ///
    /// Returns true if the server was added successfully.
    pub fn add_server<T: oxidros_core::ServiceMsg + 'static>(
        &mut self,
        mut server: crate::service::Server<T>,
        mut handler: oxidros_core::selector::ServerCallback<T>,
    ) -> bool
    where
        T::Request: oxidros_core::TypeSupport,
        T::Response: oxidros_core::TypeSupport,
    {
        // Create a closure that tries to receive and call the handler
        let poll_fn = Box::new(move || -> bool {
            match server.try_recv() {
                Ok(Some(service_req)) => {
                    let (sender, request) = service_req.split();
                    let response = handler(request);
                    if let Err(e) = sender.send(&response) {
                        eprintln!("Failed to send service response: {e}");
                    }
                    true
                }
                Ok(None) => false,
                Err(e) => {
                    eprintln!("Failed to receive service request: {e}");
                    false
                }
            }
        });
        self.server_handlers.push(poll_fn);
        true
    }

    /// Add a parameter server with a callback handler.
    ///
    /// The handler will be called whenever parameters are updated, receiving
    /// a mutable reference to the parameters and the set of updated parameter names.
    ///
    /// Only one parameter server can be added per Selector. Calling this again
    /// will replace the previous parameter server.
    pub fn add_parameter_server(
        &mut self,
        mut param_server: ZenohParameterServer,
        mut handler: ParameterServerCallback,
    ) {
        let params = param_server.params.clone();

        // Create a closure that polls the parameter server and calls handler on updates
        let poll_fn = Box::new(move || -> bool {
            let processed = param_server.try_process_once();

            // Check for updated parameters and call handler
            let mut guard = params.write();
            let updated = guard.take_updated();
            if !updated.is_empty() {
                handler(&mut guard, updated);
            }

            processed
        });

        self.parameter_server_handler = Some(poll_fn);
    }

    /// Add a one-shot timer that fires once after the given duration.
    ///
    /// Returns a timer ID that can be used to remove the timer before it fires.
    pub fn add_timer(&mut self, duration: Duration, handler: Box<dyn FnMut()>) -> u64 {
        let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timer = Timer {
            period: duration,
            next_fire: Instant::now() + duration,
            handler,
            one_shot: true,
        };
        self.timers.insert(id, timer);
        id
    }

    /// Add a wall timer that fires periodically.
    ///
    /// The timer will fire repeatedly at the given period until removed.
    ///
    /// Returns a timer ID that can be used to remove the timer.
    pub fn add_wall_timer(
        &mut self,
        _name: &str,
        period: Duration,
        handler: Box<dyn FnMut()>,
    ) -> u64 {
        let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timer = Timer {
            period,
            next_fire: Instant::now() + period,
            handler,
            one_shot: false,
        };
        self.timers.insert(id, timer);
        id
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

            // Poll all service servers
            for handler in &mut self.server_handlers {
                handler();
            }

            // Poll parameter server
            if let Some(ref mut handler) = self.parameter_server_handler {
                handler();
            }

            // Process expired timers
            let now = Instant::now();
            let mut timers_to_remove = Vec::new();
            for (&id, timer) in self.timers.iter_mut() {
                if now >= timer.next_fire {
                    (timer.handler)();
                    if timer.one_shot {
                        // One-shot timer: mark for removal
                        timers_to_remove.push(id);
                    } else {
                        // Periodic timer: reschedule
                        timer.next_fire = now + timer.period;
                    }
                }
            }
            // Remove fired one-shot timers
            for id in timers_to_remove {
                self.timers.remove(&id);
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

    fn add_subscriber<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Self::Subscriber<T>,
        handler: Box<dyn FnMut(Message<T>)>,
    ) -> bool {
        Self::add_subscriber(self, subscriber, handler)
    }

    fn add_server<T: oxidros_core::ServiceMsg + 'static>(
        &mut self,
        server: Self::Server<T>,
        handler: Box<dyn FnMut(Message<T::Request>) -> T::Response>,
    ) -> bool
    where
        T::Request: TypeSupport,
        T::Response: TypeSupport,
    {
        Self::add_server(self, server, handler)
    }

    fn add_parameter_server(
        &mut self,
        param_server: Self::ParameterServer,
        handler: Box<dyn FnMut(&mut Parameters, BTreeSet<String>)>,
    ) {
        Self::add_parameter_server(self, param_server, handler)
    }

    fn add_timer(&mut self, duration: Duration, handler: Box<dyn FnMut()>) -> u64 {
        Self::add_timer(self, duration, handler)
    }

    fn add_wall_timer(&mut self, name: &str, period: Duration, handler: Box<dyn FnMut()>) -> u64 {
        Self::add_wall_timer(self, name, period, handler)
    }

    fn delete_timer(&mut self, id: u64) {
        Self::remove_timer(self, id)
    }

    fn add_action_server<T, GR, A, CR>(
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

    fn add_action_client<T: oxidros_core::ActionMsg + 'static>(
        &mut self,
        _client: Self::ActionClient<T>,
    ) -> oxidros_core::Result<bool> {
        Err(oxidros_core::Error::NotImplemented {
            feature: "action_client".into(),
            reason: "Zenoh backend does not support actions yet".into(),
        })
    }

    fn wait(&mut self) -> oxidros_core::Result<()> {
        Self::wait(self)
    }

    fn wait_timeout(&mut self, timeout: Duration) -> oxidros_core::Result<bool> {
        Self::wait_timeout(self, timeout)
    }
}
