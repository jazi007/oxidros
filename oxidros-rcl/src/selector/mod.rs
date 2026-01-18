//! Selector provides functions like `select` or `epoll`.
//! This is used to single threaded execution.
//! For multi threaded execution, this is used internally.
//!
//! # Example
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context, logger::Logger, msg::common_interfaces::std_msgs, pr_info,
//! };
//! use std::time::Duration;
//!
//! // First of all, you need create a context.
//! let ctx = Context::new().unwrap();
//!
//! // Create a subscribe node.
//! let node_sub = ctx
//!     .create_node_with_opt("selector_rs", None, Default::default())
//!     .unwrap();
//!
//! // Create a subscriber.
//! let subscriber = node_sub
//!     .create_subscriber::<std_msgs::msg::String>("selector_topic", None,
//! ).unwrap();
//!
//! // Create a selector, which is for IO multiplexing.
//! let mut selector = ctx.create_selector().unwrap();
//!
//! // Create a logger.
//! let logger_sub = Logger::new("selector_rs");
//!
//! // Add subscriber to the selector.
//! // The 2nd argument is a callback function.
//! // If data arrive, the callback will be invoked.
//! selector.add_subscriber(
//!     subscriber,
//!     Box::new(move |msg| {
//!         // Print the message
//!         pr_info!(logger_sub, "Received: msg = {}", msg.data); // Print a message.
//!     }),
//! );
//!
//! // Create a wall timer, which invoke the callback periodically.
//! selector.add_wall_timer(
//!     "timer_name", // name of the timer
//!     Duration::from_millis(100),
//!     Box::new(move || ()),
//! );
//!
//! // Spin.
//! for _ in 0..10 {
//!     selector.wait().unwrap();
//! }
//! ```

use self::guard_condition::{GuardCondition, RCLGuardCondition};

use crate::{
    PhantomUnsend, PhantomUnsync,
    action::{self, SendGoalServiceRequest, handle::GoalHandle},
    context::Context,
    error::Result,
    get_allocator,
    logger::{Logger, pr_error_in, pr_fatal_in},
    msg::{ActionMsg, GetUUID, ServiceMsg, TypeSupport, interfaces::action_msgs::msg::GoalInfo},
    parameter::ParameterServer,
    rcl::{self, rcl_action_client_t},
    service::{
        client::{ClientData, ClientRecv},
        server::{Server, ServerData},
    },
    signal_handler::{self, Signaled},
    topic::subscriber::{RCLSubscription, Subscriber},
};
use oxidros_core::{
    Error, RclError,
    delta_list::DeltaList,
    message::Message,
    selector::{
        ActionHandler, CallbackResult, ConditionHandler, ParameterCallback, ServerCallback,
    },
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    ptr::null_mut,
    rc::Rc,
    sync::Arc,
    time::{Duration, SystemTime},
};

use parking_lot::Mutex;

#[cfg(not(feature = "statistics"))]
use crate::rcl::rcl_action_server_t;

#[cfg(feature = "statistics")]
use serde::Serialize;

#[cfg(feature = "statistics")]
use crate::helper::statistics::{SerializableTimeStat, TimeStatistics};

pub(crate) mod async_selector;
pub(crate) mod guard_condition;

#[cfg_attr(feature = "statistics", allow(dead_code))]
struct ActionClientConditionHandler {
    client: *const rcl_action_client_t,
    feedback_handler: Option<ActionHandler>,
    status_handler: Option<ActionHandler>,
    goal_handler: Option<ActionHandler>,
    cancel_goal_handler: Option<ActionHandler>,
    result_handler: Option<ActionHandler>,
}

#[cfg_attr(feature = "statistics", allow(dead_code))]
struct ActionServerConditionHandler {
    goal_handler: Option<ActionHandler>,
    cancel_goal_handler: Option<ActionHandler>,
    result_handler: Option<ActionHandler>,
}

enum TimerType {
    WallTimer(Rc<String>, Duration),
    OneShot,
}

#[cfg(feature = "statistics")]
#[derive(Debug)]
struct TimeStat {
    #[cfg(feature = "rcl_stat")]
    rcl_wait: TimeStatistics<4096>,

    callback: BTreeMap<*const (), (String, TimeStatistics<4096>)>,
    wall_timer: BTreeMap<String, TimeStatistics<4096>>,
}

#[cfg(feature = "statistics")]
#[derive(Serialize, Debug)]
pub struct Statistics {
    #[cfg(feature = "rcl_stat")]
    pub rcl_wait: SerializableTimeStat,

    #[cfg(feature = "rcl_stat")]
    pub rcl_take: Vec<SerializableTimeStat>,

    pub callback: BTreeMap<String, SerializableTimeStat>, // callback functions of subscribers and servers
    pub wall_timer: BTreeMap<String, SerializableTimeStat>, // wall timers
}

#[derive(Debug)]
struct EntitySize {
    subscriptions: usize,
    guard_condititons: usize,
    timers: usize,
    clients: usize,
    services: usize,
    events: usize,
}

/// Selector invokes callback functions associated with subscribers, services, timers, or condition variables.
/// Selector cannot send to another thread and shared by multiple threads.
/// So, use this for single threaded execution.
///
/// # Example
///
/// ```
/// use oxidros_rcl::context::Context;
///
/// let ctx = Context::new().unwrap();
/// let mut selector = ctx.create_selector(); // Create a new selector.
/// ```
pub struct Selector {
    param_server: Option<ParameterServer>,
    timer: DeltaList<(ConditionHandler<TimerType>, u64)>,
    base_time: SystemTime,
    signal_cond: GuardCondition,
    wait_set: rcl::rcl_wait_set_t,
    services: BTreeMap<*const rcl::rcl_service_t, ConditionHandler<Arc<ServerData>>>,
    clients: BTreeMap<*const rcl::rcl_client_t, ConditionHandler<Arc<ClientData>>>,
    subscriptions: BTreeMap<*const rcl::rcl_subscription_t, ConditionHandler<Arc<RCLSubscription>>>,
    action_servers: BTreeMap<*const rcl::rcl_action_server_t, Vec<ActionServerConditionHandler>>,
    action_clients: BTreeMap<*const rcl::rcl_action_client_t, ActionClientConditionHandler>,
    cond: BTreeMap<*const rcl::rcl_guard_condition_t, ConditionHandler<Arc<RCLGuardCondition>>>,
    timer_ids: BTreeSet<u64>,
    timer_id: u64,
    context: Arc<Context>,

    #[cfg(feature = "statistics")]
    time_stat: TimeStat,

    _unused: (PhantomUnsync, PhantomUnsend),
}

impl Selector {
    pub(crate) fn new(context: Arc<Context>) -> Result<Self> {
        let mut wait_set = rcl::MTSafeFn::rcl_get_zero_initialized_wait_set();

        {
            let guard = rcl::MT_UNSAFE_FN.lock();
            guard.rcl_wait_set_init(
                &mut wait_set,
                0,
                0,
                0,
                0,
                0,
                0,
                unsafe { context.as_ptr_mut() },
                get_allocator(),
            )?;
        }

        #[cfg(feature = "statistics")]
        let time_stat = TimeStat {
            #[cfg(feature = "rcl_stat")]
            rcl_wait: TimeStatistics::new(),

            callback: BTreeMap::new(),
            wall_timer: BTreeMap::new(),
        };

        let signal_cond = GuardCondition::new(context.clone())?;
        let mut selector = Selector {
            param_server: None,
            timer: DeltaList::Nil,
            base_time: SystemTime::now(),
            signal_cond: signal_cond.clone(),
            wait_set,
            subscriptions: Default::default(),
            services: Default::default(),
            clients: Default::default(),
            action_servers: Default::default(),
            action_clients: Default::default(),
            cond: Default::default(),
            timer_ids: Default::default(),
            timer_id: 0,

            context,

            #[cfg(feature = "statistics")]
            time_stat,

            _unused: (Default::default(), Default::default()),
        };

        selector.add_guard_condition(&signal_cond, None, false);
        signal_handler::register_guard_condition(signal_cond);

        Ok(selector)
    }

    #[cfg(feature = "statistics")]
    pub fn statistics(&self) -> Statistics {
        let callback = self
            .time_stat
            .callback
            .iter()
            .map(|(_, (k, v))| (k.clone(), v.to_serializable()))
            .collect();

        let wall_timer = self
            .time_stat
            .wall_timer
            .iter()
            .map(|(k, v)| (k.clone(), v.to_serializable()))
            .collect();

        #[cfg(feature = "rcl_stat")]
        let mut rcl_take = Vec::new();

        #[cfg(feature = "rcl_stat")]
        for (_, v) in self.subscriptions.iter() {
            let guard = v.event.latency_take.lock();
            let s = guard.to_serializable();
            rcl_take.push(s);
        }

        Statistics {
            #[cfg(feature = "rcl_stat")]
            rcl_wait: self.time_stat.rcl_wait.to_serializable(),

            #[cfg(feature = "rcl_stat")]
            rcl_take,

            callback,
            wall_timer,
        }
    }

    /// Register a subscriber with callback function.
    /// The callback function will be invoked when arriving data.
    ///
    /// # Error
    ///
    /// If a selector takes a subscriber created by a different context,
    /// `add_subscriber()` must fail.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{msg::common_interfaces::std_msgs, node::Node, selector::Selector, topic::subscriber::Message};
    /// use std::sync::Arc;
    ///
    /// fn add_new_subscriber(selector: &mut Selector, node: Arc<Node>) {
    ///     // Create a subscriber.
    ///     let subscriber = node.create_subscriber("node_name", None,
    ///     ).unwrap();
    ///
    ///     // Add the subscriber with a callback function.
    ///     selector.add_subscriber(
    ///         subscriber,
    ///         Box::new(|msg: Message<std_msgs::msg::Bool>| /* some tasks */ ()), // Callback function.
    ///     );
    /// }
    /// ```
    pub fn add_subscriber<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Subscriber<T>,
        mut handler: Box<dyn FnMut(Message<T>)>,
    ) -> bool {
        let sub = subscriber.subscription.clone();
        let context_ptr = subscriber.subscription.node.context.as_ptr();

        #[cfg(feature = "statistics")]
        let symbol = {
            let node_name = subscriber.subscription.node.get_name().unwrap_or_default();
            let node_namespace = subscriber
                .subscription
                .node
                .get_namespace()
                .unwrap_or_default();
            let topic_name = subscriber.get_topic_name();
            format!("{node_namespace}:{node_name}:subscriber:{topic_name}")
        };

        let f = move || {
            let start = SystemTime::now();
            let dur = Duration::from_millis(1);

            loop {
                match subscriber.try_recv() {
                    Ok(Some(n)) => {
                        handler(n);
                    }
                    Ok(None) => return CallbackResult::Ok,
                    Err(e) => {
                        let logger = Logger::new("oxidros");
                        pr_error_in!(logger, "failed try_recv() of subscriber: {}", e);
                        return CallbackResult::Remove;
                    }
                }

                if let Ok(t) = start.elapsed() {
                    if t > dur {
                        return CallbackResult::Ok;
                    }
                } else {
                    return CallbackResult::Ok;
                }
            }
        };

        if self.context.as_ptr() == context_ptr {
            #[cfg(feature = "statistics")]
            {
                self.time_stat.callback.insert(
                    sub.subscription.as_ref() as *const _ as *const (),
                    (symbol, TimeStatistics::new()),
                );
            }

            self.add_rcl_subscription(sub, Some(Box::new(f)), false);
            true
        } else {
            false
        }
    }

    pub(crate) fn add_rcl_subscription(
        &mut self,
        subscription: Arc<RCLSubscription>,
        handler: Option<Box<dyn FnMut() -> CallbackResult>>,
        is_once: bool,
    ) {
        self.subscriptions.insert(
            subscription.subscription.as_ref(),
            ConditionHandler {
                event: subscription,
                handler,
                is_once,
            },
        );
    }

    pub fn add_parameter_server(
        &mut self,
        param_server: ParameterServer,
        mut handler: ParameterCallback,
    ) {
        let params = param_server.params.clone();

        self.add_guard_condition(
            &param_server.cond_callback,
            Some(Box::new(move || {
                let mut guard = params.write();
                let updated = guard.take_updated();
                handler(&mut guard, updated);
                CallbackResult::Ok
            })),
            false,
        );
        self.param_server = Some(param_server);
    }

    /// Register a subscriber with callback function.
    /// The callback function will be invoked when arriving data.
    ///
    /// # Error
    ///
    /// If a selector takes a server created by a different context,
    /// `add_server()` must fail.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{msg::{common_interfaces::std_srvs, ServiceMsg}, node::Node, selector::Selector};
    /// use std::sync::Arc;
    ///
    /// fn add_new_server(selector: &mut Selector, node: Arc<Node>) {
    ///     // Create a server.
    ///     let server = node
    ///         .create_server::<std_srvs::srv::Empty>("select_rs_service", None)
    ///         .unwrap();
    ///
    ///     // Add the server with a callback function.
    ///     selector.add_server(
    ///         server,
    ///         Box::new(|request| {
    ///             // Return the response.
    ///             let response = std_srvs::srv::Empty_Response::new().unwrap();
    ///             response
    ///         }), // Callback function.
    ///     );
    /// }
    /// ```
    pub fn add_server<T: ServiceMsg + 'static>(
        &mut self,
        mut server: Server<T>,
        mut handler: ServerCallback<T>,
    ) -> bool {
        let context_ptr = server.data.node.context.as_ptr();
        let srv = server.data.clone();

        let f = move || {
            let start = SystemTime::now();
            let dur = Duration::from_millis(1);

            loop {
                match server.try_recv() {
                    Ok(Some(service_req)) => {
                        let result = handler(service_req.request);
                        match service_req.sender.send(&result) {
                            Ok(()) => {}
                            Err(e) => {
                                let logger = Logger::new("oxidros");
                                pr_error_in!(logger, "{e}");
                                return CallbackResult::Ok;
                            }
                        }
                    }
                    Ok(None) => {
                        return CallbackResult::Ok;
                    }
                    Err(e) => {
                        let logger = Logger::new("oxidros");
                        pr_fatal_in!(logger, "failed try_recv() of server: {}", e);
                        return CallbackResult::Remove;
                    }
                }

                if let Ok(t) = start.elapsed() {
                    if t > dur {
                        return CallbackResult::Ok;
                    }
                } else {
                    return CallbackResult::Ok;
                }
            }
        };

        if self.context.as_ptr() == context_ptr {
            self.add_server_data(srv, Some(Box::new(f)), false);
            true
        } else {
            false
        }
    }

    pub(crate) fn add_server_data(
        &mut self,
        server: Arc<ServerData>,
        handler: Option<Box<dyn FnMut() -> CallbackResult>>,
        is_once: bool,
    ) {
        let cloned = server.clone();
        if self.context.as_ptr() == server.node.context.as_ptr() {
            self.services.insert(
                &server.service,
                ConditionHandler {
                    event: cloned,
                    handler,
                    is_once,
                },
            );
        }
    }

    /// Wait a response from a server.
    /// After waking up, the registered client is removed from the selector.
    /// You have to register every time when you wait events.
    pub(crate) fn add_client_recv<T: ServiceMsg>(&mut self, client: &ClientRecv<T>) {
        self.add_client_data(client.data.data.clone(), None);
    }

    pub(crate) fn add_client_data(
        &mut self,
        client: Arc<ClientData>,
        handler: Option<Box<dyn FnMut() -> CallbackResult>>,
    ) {
        self.clients.insert(
            &client.client,
            ConditionHandler {
                event: client,
                handler,
                is_once: true,
            },
        );
    }

    /// Register an action server with a callback. The callback is invoked when
    /// requests from action clients arrive.
    /// - `goal_handler` is invoked when the action server receives a new goal.
    /// - `cancel_goal_handler` is invoked when the action server receives a request to cancel a goal.
    ///
    /// Requests for goal results are automatically handled.
    ///
    /// # Example
    /// ```ignore
    /// # // Ignoring this code block since common module is not available in doc tests.
    /// # use oxidros_rcl::{selector::Selector, action::server::Server, msg::ActionMsg};
    /// # use common::msgs::example_msg::action::*;
    ///
    /// fn add_action_server(selector: &mut Selector, server: Server<MyAction>) {
    ///     selector.add_action_server(server,
    ///         // return true to accept the goal
    ///         |req| {
    ///             // do some validation here...
    ///             true
    ///         }
    ///         // executed if accepted
    ///         |handle| {
    ///             // spawn a worker thread
    ///             std::thread::spawn(move || {
    ///                 // send a feedback
    ///                 let feedback = MyAction_Feedback { c: 4 };
    ///                 handle.feedback(feedback).unwrap();
    ///
    ///                 // send a result when finished
    ///                 handle.finish(MyAction_Result { b: 500 }).unwrap();
    ///             });
    ///
    ///             true // return true to accept the goal
    ///         },
    ///         /// handler for cancel requests
    ///         |req| { true } // return true to cancel the goal
    ///     );
    /// }
    /// ```
    pub fn add_action_server<T: ActionMsg + 'static, GR, A, CR>(
        &mut self,
        server: action::server::Server<T>,
        goal_handler: GR,
        accept_handler: A,
        cancel_goal_handler: CR,
    ) -> bool
    where
        GR: Fn(SendGoalServiceRequest<T>) -> bool + 'static,
        A: Fn(GoalHandle<T>) + 'static,
        CR: Fn(&GoalInfo) -> bool + 'static,
    {
        let server = Arc::new(Mutex::new(server));
        let goal = {
            let server = server.clone();
            move || {
                let start = SystemTime::now();
                let dur = Duration::from_millis(1);
                let mut server = server.lock();

                loop {
                    match server.try_recv_goal_request() {
                        Ok(Some((sender, request))) => {
                            let accepted = goal_handler(request);
                            match if accepted {
                                sender.accept(&accept_handler)
                            } else {
                                sender.reject()
                            } {
                                Ok(_) => return CallbackResult::Ok,
                                Err(e) => {
                                    let logger = Logger::new("oxidros");
                                    pr_error_in!(logger, "Failed to send goal response: {}", e);
                                    return CallbackResult::Remove;
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            let logger = Logger::new("oxidros");
                            pr_error_in!(
                                logger,
                                "failed try_recv_goal_request() of action server: {}",
                                e
                            );
                            return CallbackResult::Remove;
                        }
                    }

                    if let Ok(t) = start.elapsed() {
                        if t > dur {
                            return CallbackResult::Ok;
                        }
                    } else {
                        return CallbackResult::Ok;
                    }
                }
            }
        };

        let cancel = {
            let server = server.clone();
            move || {
                let start = SystemTime::now();
                let dur = Duration::from_millis(1);
                let mut server = server.lock();

                loop {
                    match server.try_recv_cancel_request() {
                        Ok(Some((sender, _req, goals))) => {
                            let accepted_goals: Vec<_> = goals
                                .into_iter()
                                .filter(|goal| cancel_goal_handler(goal))
                                .collect();

                            match sender.send(accepted_goals) {
                                Ok(_) => return CallbackResult::Ok,
                                Err(e) => {
                                    let logger = Logger::new("oxidros");
                                    pr_error_in!(
                                        logger,
                                        "failed to send cancel responses from action server: {e}",
                                    );
                                    return CallbackResult::Remove;
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            let logger = Logger::new("oxidros");
                            pr_error_in!(
                                logger,
                                "failed try_recv_cancel_request() of action server: {}",
                                e
                            );
                            return CallbackResult::Remove;
                        }
                    }

                    if let Ok(t) = start.elapsed() {
                        if t > dur {
                            return CallbackResult::Ok;
                        }
                    } else {
                        return CallbackResult::Ok;
                    }
                }
            }
        };

        let result = {
            let server = server.clone();
            move || {
                let start = SystemTime::now();
                let dur = Duration::from_millis(1);
                let mut server = server.lock();

                loop {
                    match server.try_recv_result_request() {
                        Ok(Some((sender, request))) => match sender.send(request.get_uuid()) {
                            Ok(_) => return CallbackResult::Ok,
                            Err(e) => {
                                let logger = Logger::new("oxidros");
                                pr_error_in!(
                                    logger,
                                    "failed to send cancel responses from action server: {e}",
                                );
                                return CallbackResult::Remove;
                            }
                        },
                        Ok(None) => {}
                        Err(e) => {
                            let logger = Logger::new("oxidros");
                            pr_error_in!(
                                logger,
                                "failed try_recv_result_request() of action server: {}",
                                e
                            );
                            return CallbackResult::Remove;
                        }
                    }
                    if let Ok(t) = start.elapsed() {
                        if t > dur {
                            return CallbackResult::Ok;
                        }
                    } else {
                        return CallbackResult::Ok;
                    }
                }
            }
        };

        let server = server.lock();
        let context_ptr = server.data.node.context.as_ptr();
        if self.context.as_ptr() == context_ptr {
            self.add_action_server_data(
                server.data.clone(),
                Some(Box::new(goal)),
                Some(Box::new(cancel)),
                Some(Box::new(result)),
            );
            true
        } else {
            false
        }
    }

    pub(crate) fn add_action_server_data(
        &mut self,
        server: Arc<action::server::ServerData>,
        goal_handler: Option<ActionHandler>,
        cancel_goal_handler: Option<ActionHandler>,
        result_handler: Option<ActionHandler>,
    ) {
        let s = &server.server as *const _;
        if self.action_servers.contains_key(&s) {
            let handlers = self.action_servers.get_mut(&s).unwrap();
            handlers.push(ActionServerConditionHandler {
                goal_handler,
                cancel_goal_handler,
                result_handler,
            });
        } else {
            self.action_servers.insert(
                &server.server,
                vec![ActionServerConditionHandler {
                    goal_handler,
                    cancel_goal_handler,
                    result_handler,
                }],
            );
        }
    }

    pub(crate) fn add_action_client(
        &mut self,
        client: Arc<action::client::ClientData>,
        feedback_handler: Option<ActionHandler>,
        status_handler: Option<ActionHandler>,
        goal_handler: Option<ActionHandler>,
        cancel_goal_handler: Option<ActionHandler>,
        result_handler: Option<ActionHandler>,
    ) {
        self.add_action_client_data(
            client,
            feedback_handler,
            status_handler,
            goal_handler,
            cancel_goal_handler,
            result_handler,
        )
    }

    pub(crate) fn add_action_client_data(
        &mut self,
        client: Arc<action::client::ClientData>,
        feedback_handler: Option<ActionHandler>,
        status_handler: Option<ActionHandler>,
        goal_handler: Option<ActionHandler>,
        cancel_goal_handler: Option<ActionHandler>,
        result_handler: Option<ActionHandler>,
    ) {
        self.action_clients.insert(
            &client.client as *const _,
            ActionClientConditionHandler {
                client: &client.client as *const _,
                feedback_handler,
                status_handler,
                goal_handler,
                cancel_goal_handler,
                result_handler,
            },
        );
    }

    pub(crate) fn add_guard_condition(
        &mut self,
        cond: &GuardCondition,
        handler: Option<Box<dyn FnMut() -> CallbackResult>>,
        is_once: bool,
    ) {
        self.cond.insert(
            cond.cond.cond.as_ref(),
            ConditionHandler {
                event: cond.cond.clone(),
                handler,
                is_once,
            },
        );
    }

    pub(crate) fn remove_guard_condition(&mut self, cond: &GuardCondition) {
        self.cond.remove(&(cond.cond.cond.as_ref() as *const _));
    }

    pub(crate) fn remove_rcl_subscription(&mut self, subscription: &Arc<RCLSubscription>) {
        self.subscriptions
            .remove(&(subscription.subscription.as_ref() as *const _));
    }

    pub(crate) fn remove_server_data(&mut self, server: &Arc<ServerData>) {
        self.services.remove(&(&server.service as *const _));
    }

    pub(crate) fn remove_client_data(&mut self, client: &Arc<ClientData>) {
        self.clients.remove(&(&client.client as *const _));
    }

    pub(crate) fn remove_action_client_data(&mut self, client: &Arc<action::client::ClientData>) {
        self.action_clients.remove(&(&client.client as *const _));
    }

    pub(crate) fn remove_action_server_data(&mut self, server: &Arc<action::server::ServerData>) {
        self.action_servers.remove(&(&server.server as *const _));
    }

    /// Add a timer.
    /// The `handler` is called after `t` seconds later.
    /// The `handler` is called just once.
    ///
    /// # Return Value
    ///
    /// The identifier of the timer.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::selector::Selector;
    /// use std::time::Duration;
    ///
    /// fn add_new_timer(selector: &mut Selector) {
    ///     // Add a timer.
    ///     selector.add_timer(
    ///         Duration::from_millis(100),
    ///         Box::new(|| /* some tasks */ ()), // Callback function.
    ///     );
    /// }
    /// ```
    pub fn add_timer(&mut self, t: Duration, mut handler: Box<dyn FnMut()>) -> u64 {
        self.add_timer_inner(
            t,
            Box::new(move || {
                handler();
                CallbackResult::Ok
            }),
            TimerType::OneShot,
        )
    }

    /// Add a wall timer.
    /// The `handler` is called after `t` seconds later.
    /// The `handler` will be automatically reloaded after calling it.
    /// It means the `handler` is called periodically.
    ///
    /// # Return Value
    ///
    /// The identifier of the timer.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::selector::Selector;
    /// use std::time::Duration;
    ///
    /// fn add_new_wall_timer(selector: &mut Selector) {
    ///     // Add a timer.
    ///     selector.add_wall_timer(
    ///         "timer_name",
    ///         Duration::from_millis(100),
    ///         Box::new(|| /* some tasks */ ()), // Callback function.
    ///     );
    /// }
    /// ```
    pub fn add_wall_timer(
        &mut self,
        name: &str,
        t: Duration,
        mut handler: Box<dyn FnMut()>,
    ) -> u64 {
        #[cfg(feature = "statistics")]
        self.time_stat
            .wall_timer
            .insert(name.to_string(), TimeStatistics::new());

        self.add_timer_inner(
            t,
            Box::new(move || {
                handler();
                CallbackResult::Ok
            }),
            TimerType::WallTimer(Rc::new(name.to_string()), t),
        )
    }

    fn add_timer_inner(
        &mut self,
        t: Duration,
        handler: Box<dyn FnMut() -> CallbackResult>,
        timer_type: TimerType,
    ) -> u64 {
        let now_time = SystemTime::now();

        if self.timer.is_empty() {
            self.base_time = now_time;
        }

        let delta = if let Ok(d) = now_time.duration_since(self.base_time) {
            // if base_time <= now_time
            // delta = now_time - base_time + t
            d + t
        } else {
            // if now_time < base_time
            // delta = t
            let d = self.base_time.duration_since(now_time).unwrap();

            if let Some(head) = self.timer.front_mut() {
                *head.0 += d; // update delta
            }
            self.base_time = now_time; // set base_time now
            t
        };

        let timer_id = self.new_timer_id();

        self.timer.insert(
            delta,
            (
                ConditionHandler {
                    is_once: true,
                    event: timer_type,
                    handler: Some(handler),
                },
                timer_id,
            ),
        );

        timer_id
    }

    /// Wait events and invoke registered callback functions.
    /// This function returns after `t` duration; timeout.
    ///
    /// # Return Value
    ///
    /// - `Ok(true)`: Some events has fired
    /// - `Ok(false)`: Timeout
    /// - `Err(Error)`: Error
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{error::Result, selector::Selector};
    ///
    /// fn wait_events(selector: &mut Selector) -> Result<()> {
    ///     if selector.wait_timeout(std::time::Duration::from_millis(10))? {
    ///         // Some events has fired.
    ///     } else {
    ///         // Timeout.
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn wait_timeout(&mut self, t: Duration) -> Result<bool> {
        let flag = Rc::new(Cell::new(false));
        let flag_cloned = flag.clone();

        let id = self.add_timer(t, Box::new(move || flag_cloned.set(true)));

        let result = self.wait();

        if flag.get() {
            Ok(false) // timeout
        } else {
            // event fired
            self.remove_timer(id);
            result?;
            Ok(true)
        }
    }

    pub fn remove_timer(&mut self, id: u64) {
        self.timer.filter(|e| e.1 != id);
    }

    fn new_timer_id(&mut self) -> u64 {
        loop {
            if !self.timer_ids.contains(&self.timer_id) {
                self.timer_ids.insert(self.timer_id);
                let id = self.timer_id;
                self.timer_id += 1;
                return id;
            }
        }
    }

    fn set_rcl_wait(&mut self) -> Result<()> {
        let guard = rcl::MT_UNSAFE_FN.lock();
        guard.rcl_wait_set_clear(&mut self.wait_set)?;

        let entities = self.get_num_entities()?;
        guard.rcl_wait_set_resize(
            &mut self.wait_set,
            entities.subscriptions,
            entities.guard_condititons,
            entities.timers,
            entities.clients,
            entities.services,
            entities.events,
        )?;
        // set subscriptions
        for (_, h) in self.subscriptions.iter() {
            guard.rcl_wait_set_add_subscription(
                &mut self.wait_set,
                h.event.subscription.as_ref(),
                null_mut(),
            )?;
        }
        // set guard conditions
        for (_, h) in self.cond.iter() {
            guard.rcl_wait_set_add_guard_condition(
                &mut self.wait_set,
                h.event.cond.as_ref(),
                null_mut(),
            )?;
        }
        // set clients
        for (_, h) in self.clients.iter() {
            guard.rcl_wait_set_add_client(&mut self.wait_set, &h.event.client, null_mut())?;
        }
        // set services
        for (_, h) in self.services.iter() {
            guard.rcl_wait_set_add_service(&mut self.wait_set, &h.event.service, null_mut())?;
        }
        // set action clients
        for (_, h) in self.action_clients.iter() {
            guard.rcl_action_wait_set_add_action_client(
                &mut self.wait_set,
                h.client,
                null_mut(),
                null_mut(),
            )?;
        }
        // set action servers
        for (s, _v) in self.action_servers.iter() {
            guard.rcl_action_wait_set_add_action_server(&mut self.wait_set, *s, null_mut())?;
        }
        Ok(())
    }

    /// Wait events and invoke registered callback functions.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{error::Result, selector::Selector};
    ///
    /// fn wait_events(selector: &mut Selector) -> Result<()> {
    ///     // Add subscribers, servers, etc.
    ///
    ///     // Spin.
    ///     loop {
    ///         selector.wait()?;
    ///     }
    /// }
    /// ```
    pub fn wait(&mut self) -> Result<()> {
        // set rcl wait
        self.set_rcl_wait()?;

        // wait events
        self.wait_timer()?;

        // notify timers
        self.notify_timer();

        #[cfg(feature = "statistics")]
        {
            // notify subscriptions
            let (target, time_stat) = (&mut self.subscriptions, &mut self.time_stat);
            notify(target, self.wait_set.subscriptions, time_stat);

            // notify services
            let (target, time_stat) = (&mut self.services, &mut self.time_stat);
            notify(target, self.wait_set.services, time_stat);

            // notify clients
            let (target, time_stat) = (&mut self.clients, &mut self.time_stat);
            notify(target, self.wait_set.clients, time_stat);

            // notify guard conditions
            let (target, time_stat) = (&mut self.cond, &mut self.time_stat);
            notify(target, self.wait_set.guard_conditions, time_stat);
        }

        #[cfg(not(feature = "statistics"))]
        {
            // notify subscriptions
            notify(&mut self.subscriptions, self.wait_set.subscriptions);

            // notify services
            notify(&mut self.services, self.wait_set.services);

            // notify clients
            notify(&mut self.clients, self.wait_set.clients);

            // notify guard conditions
            notify(&mut self.cond, self.wait_set.guard_conditions);

            notify_action_server(&mut self.action_servers, &self.wait_set)?;
            notify_action_client(&mut self.action_clients, &self.wait_set)?;
        }

        Ok(())
    }

    fn wait_timer(&mut self) -> Result<()> {
        if signal_handler::is_halt() {
            return Err(Signaled.into());
        }

        if self.timer.is_empty() {
            #[cfg(feature = "rcl_stat")]
            let wait_start = SystemTime::now();

            // wait forever until arriving events
            rcl::MTSafeFn::rcl_wait(&mut self.wait_set, -1)?;

            #[cfg(feature = "rcl_stat")]
            {
                if let Ok(wait_time) = wait_start.elapsed() {
                    self.time_stat.rcl_wait.add(wait_time);
                }
            }
        } else {
            // insert timer
            let now_time = SystemTime::now();
            let head_delta = *self.timer.front().unwrap().0;
            let timeout = if self.base_time <= now_time {
                let diff = now_time.duration_since(self.base_time).unwrap();
                if diff < head_delta {
                    head_delta - diff
                } else {
                    Duration::ZERO
                }
            } else {
                head_delta + self.base_time.duration_since(now_time).unwrap()
            };

            let timeout_nanos = timeout.as_nanos();
            let timeout_nanos = if timeout_nanos > i64::MAX as u128 {
                let logger = Logger::new("oxidros");
                pr_error_in!(
                    logger,
                    "timeout value became too big (overflow): timeout = {timeout_nanos}"
                );
                i64::MAX
            } else {
                timeout_nanos as i64
            };

            #[cfg(feature = "rcl_stat")]
            let wait_start = SystemTime::now();

            match rcl::MTSafeFn::rcl_wait(&mut self.wait_set, timeout_nanos) {
                Err(Error::Rcl(RclError::Timeout)) => (),
                Err(e) => return Err(e),
                _ => {
                    #[cfg(feature = "rcl_stat")]
                    {
                        if let Ok(wait_time) = wait_start.elapsed() {
                            self.time_stat.rcl_wait.add(wait_time);
                        }
                    }
                }
            }
        }

        if signal_handler::is_halt() {
            return Err(Signaled.into());
        }

        Ok(())
    }

    fn notify_timer(&mut self) {
        let now_time = SystemTime::now();
        let mut reload = Vec::new(); // wall timer to be reloaded: (name, delay, original_period, handler)

        while let Some(head) = self.timer.front() {
            if let Some(head_time) = self.base_time.checked_add(*head.0) {
                if head_time < now_time {
                    // pop and execute a callback function
                    let mut dlist = self.timer.pop().unwrap();
                    let head = dlist.front_mut().unwrap();
                    self.base_time += *head.0;

                    let handler = head.1.0.handler.take();
                    if let Some(mut handler) = handler {
                        #[cfg(feature = "statistics")]
                        let start = std::time::SystemTime::now();

                        handler(); // invoke the callback function

                        // register the wall timer again.
                        if let TimerType::WallTimer(name, period) = &head.1.0.event {
                            let elapsed = now_time.elapsed().unwrap();

                            // Calculate the delay until next fire, compensating for elapsed time
                            let delay = period.saturating_sub(elapsed);
                            // Store (name, delay, original_period, handler)
                            reload.push((name.clone(), delay, *period, handler));

                            #[cfg(feature = "statistics")]
                            if let Ok(elapsed) = start.elapsed() {
                                if let Some(v) = self.time_stat.wall_timer.get_mut(name.as_ref()) {
                                    v.add(elapsed);
                                }
                            }
                        } else {
                            self.timer_ids.remove(&head.1.1);
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // reload wall timers
        for (name, delay, period, handler) in reload {
            self.add_timer_inner(delay, handler, TimerType::WallTimer(name, period));
        }
    }

    /// Calculates how many entities (e.g. subscriptions, timers) the selector has to wait for.
    fn get_num_entities(&self) -> Result<EntitySize> {
        // Action servers and action clients work on several underlying entities.
        let mut action_server_subscriptions_size = 0;
        let mut action_server_guard_conditions_size = 0;
        let mut action_server_timers_size = 0;
        let mut action_server_clients_size = 0;
        let mut action_server_services_size = 0;

        let mut action_client_subscriptions_size = 0;
        let mut action_client_guard_conditions_size = 0;
        let mut action_client_timers_size = 0;
        let mut action_client_clients_size = 0;
        let mut action_client_services_size = 0;

        // All the action servers (and action clients) have the same number of entities, so we are just checking how many entities the first server has to wait for, instead of calling rcl_action_server_wait_set_get_num_entities multiple times.
        if let Some((server, _)) = self.action_servers.first_key_value() {
            rcl::MTSafeFn::rcl_action_server_wait_set_get_num_entities(
                *server,
                &mut action_server_subscriptions_size,
                &mut action_server_guard_conditions_size,
                &mut action_server_timers_size,
                &mut action_server_clients_size,
                &mut action_server_services_size,
            )?;
        }

        if let Some((client, _)) = self.action_clients.first_key_value() {
            rcl::MTSafeFn::rcl_action_client_wait_set_get_num_entities(
                *client,
                &mut action_client_subscriptions_size,
                &mut action_client_guard_conditions_size,
                &mut action_client_timers_size,
                &mut action_client_clients_size,
                &mut action_client_services_size,
            )?;
        }

        let n_servers = self.action_servers.len();
        let n_clients = self.action_clients.len();

        Ok(EntitySize {
            subscriptions: self.subscriptions.len()
                + action_server_subscriptions_size * n_servers
                + action_client_subscriptions_size * n_clients,
            guard_condititons: self.cond.len()
                + action_server_guard_conditions_size * n_servers
                + action_client_guard_conditions_size * n_clients,
            timers: action_server_timers_size * n_servers + action_client_timers_size * n_clients,
            clients: self.clients.len()
                + action_server_clients_size * n_servers
                + action_client_clients_size * n_clients,
            services: self.services.len()
                + action_server_services_size * n_servers
                + action_client_services_size * n_clients,
            events: 0,
        })
    }
}

impl Drop for Selector {
    fn drop(&mut self) {
        signal_handler::unregister_guard_condition(&self.signal_cond);
        {
            let guard = rcl::MT_UNSAFE_FN.lock();
            guard.rcl_wait_set_fini(&mut self.wait_set).unwrap();
        }
    }
}

#[cfg(feature = "statistics")]
fn notify<K, V>(
    m: &mut BTreeMap<*const K, ConditionHandler<V>>,
    array: *const *const K,
    time_stat: &mut TimeStat,
) {
    for i in 0..m.len() {
        unsafe {
            let p = *array.add(i);
            if !p.is_null() {
                debug_assert!(m.contains_key(&p));
                if let Some(h) = m.get_mut(&p) {
                    let mut is_rm = false;
                    if let Some(hdl) = &mut h.handler {
                        let start = std::time::SystemTime::now();

                        let result = hdl();
                        if let Ok(dur) = start.elapsed() {
                            if let Some((_, t)) = time_stat.callback.get_mut(&(p as *const ())) {
                                t.add(dur);
                            }
                        }

                        if result == CallbackResult::Remove {
                            is_rm = true;
                        }
                    }
                    if h.is_once || is_rm {
                        m.remove(&p);
                        time_stat.callback.remove(&(p as *const ()));
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "statistics"))]
fn notify<K, V>(m: &mut BTreeMap<*const K, ConditionHandler<V>>, array: *const *const K) {
    for i in 0..m.len() {
        unsafe {
            let p = *array.add(i);
            if !p.is_null() {
                debug_assert!(m.contains_key(&p));
                if let Some(h) = m.get_mut(&p) {
                    let mut is_rm = false;
                    if let Some(hdl) = &mut h.handler
                        && hdl() == CallbackResult::Remove
                    {
                        is_rm = true;
                    }
                    if h.is_once || is_rm {
                        m.remove(&p);
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "statistics"))]
/// Scan the waitset to see if there are any updates for action servers.
fn notify_action_server(
    m: &mut BTreeMap<*const rcl_action_server_t, Vec<ActionServerConditionHandler>>,
    wait_set: *const rcl::rcl_wait_set_t,
) -> Result<()> {
    let mut ret = Ok(());

    m.retain(|server, handlers| {
        let mut is_goal_request_ready = false;
        let mut is_cancel_request_ready = false;
        let mut is_result_request_ready = false;
        // TODO: handle expired goals
        let mut is_goal_expired = false;

        {
            let guard = rcl::MT_UNSAFE_FN.lock();
            if let Err(e) = guard.rcl_action_server_wait_set_get_entities_ready(
                wait_set,
                *server,
                &mut is_goal_request_ready,
                &mut is_cancel_request_ready,
                &mut is_result_request_ready,
                &mut is_goal_expired,
            ) {
                ret = Err(e);
                return true;
            }
        }

        handlers.retain_mut(|handler| {
            let goal_remove = is_goal_request_ready
                && (handler
                    .goal_handler
                    .as_mut()
                    .is_some_and(|h| h() == CallbackResult::Remove));
            let cancel_remove = is_cancel_request_ready
                && (handler
                    .cancel_goal_handler
                    .as_mut()
                    .is_some_and(|h| h() == CallbackResult::Remove));
            let result_remove = is_result_request_ready
                && (handler
                    .result_handler
                    .as_mut()
                    .is_some_and(|h| h() == CallbackResult::Remove));

            !(goal_remove || cancel_remove || result_remove)
        });

        !handlers.is_empty()
    });

    ret
}

#[cfg(not(feature = "statistics"))]
/// Scan the waitset to see if there are any updates for action clients.
fn notify_action_client(
    m: &mut BTreeMap<*const rcl_action_client_t, ActionClientConditionHandler>,
    wait_set: *const rcl::rcl_wait_set_t,
) -> Result<()> {
    let mut ret = Ok(());

    m.retain(|client, handler| {
        let mut is_feedback_ready = false;
        let mut is_status_ready = false;
        let mut is_goal_response_ready = false;
        let mut is_cancel_response_ready = false;
        let mut is_result_response_ready = false;

        {
            let guard = rcl::MT_UNSAFE_FN.lock();
            if let Err(e) = guard.rcl_action_client_wait_set_get_entities_ready(
                wait_set,
                *client,
                &mut is_feedback_ready,
                &mut is_status_ready,
                &mut is_goal_response_ready,
                &mut is_cancel_response_ready,
                &mut is_result_response_ready,
            ) {
                ret = Err(e);
                return true;
            }
        }

        let feedback_remove = is_feedback_ready
            && (handler
                .feedback_handler
                .as_mut()
                .is_some_and(|h| h() == CallbackResult::Remove));
        let status_remove = is_status_ready
            && (handler
                .status_handler
                .as_mut()
                .is_some_and(|h| h() == CallbackResult::Remove));
        let goal_response_remove = is_goal_response_ready
            && (handler
                .goal_handler
                .as_mut()
                .is_some_and(|h| h() == CallbackResult::Remove));
        let cancel_response_remove = is_cancel_response_ready
            && (handler
                .cancel_goal_handler
                .as_mut()
                .is_some_and(|h| h() == CallbackResult::Remove));
        let result_response_remove = is_result_response_ready
            && (handler
                .result_handler
                .as_mut()
                .is_some_and(|h| h() == CallbackResult::Remove));

        !(feedback_remove
            || status_remove
            || goal_response_remove
            || cancel_response_remove
            || result_response_remove)
    });

    ret
}

// ============================================================================
// RosSelector trait implementation
// ============================================================================

impl oxidros_core::api::RosSelector for Selector {
    type Subscriber<T: TypeSupport> = Subscriber<T>;
    type Server<T: ServiceMsg> = Server<T>;
    type ActionServer<T: ActionMsg> = action::server::Server<T>;
    type ActionClient<T: ActionMsg> = action::client::Client<T>;
    type ActionGoalHandle<T: ActionMsg> = action::handle::GoalHandle<T>;
    type ParameterServer = ParameterServer;

    fn add_subscriber<T: TypeSupport + 'static>(
        &mut self,
        subscriber: Self::Subscriber<T>,
        handler: Box<dyn FnMut(Message<T>)>,
    ) -> bool {
        Self::add_subscriber(self, subscriber, handler)
    }

    fn add_server<T: ServiceMsg + 'static>(
        &mut self,
        server: Self::Server<T>,
        mut handler: Box<dyn FnMut(Message<T::Request>) -> T::Response>,
    ) -> bool {
        let wrapped = Box::new(move |request: Message<T::Request>| handler(request));
        Self::add_server(self, server, wrapped)
    }

    fn add_parameter_server(
        &mut self,
        param_server: Self::ParameterServer,
        handler: Box<
            dyn FnMut(&mut oxidros_core::parameter::Parameters, std::collections::BTreeSet<String>),
        >,
    ) {
        Self::add_parameter_server(self, param_server, handler)
    }

    fn add_timer(&mut self, duration: std::time::Duration, handler: Box<dyn FnMut()>) -> u64 {
        Self::add_timer(self, duration, handler)
    }

    fn add_wall_timer(
        &mut self,
        name: &str,
        period: std::time::Duration,
        handler: Box<dyn FnMut()>,
    ) -> u64 {
        Self::add_wall_timer(self, name, period, handler)
    }

    fn delete_timer(&mut self, id: u64) {
        Self::remove_timer(self, id)
    }

    fn add_action_server<T, GR, A, CR>(
        &mut self,
        server: Self::ActionServer<T>,
        goal_handler: GR,
        accept_handler: A,
        cancel_handler: CR,
    ) -> oxidros_core::Result<bool>
    where
        T: ActionMsg + 'static,
        GR: Fn(&<<T as ActionMsg>::Goal as oxidros_core::ActionGoal>::Request) -> bool + 'static,
        A: Fn(Self::ActionGoalHandle<T>) + 'static,
        CR: Fn(&[u8; 16]) -> bool + 'static,
    {
        // Wrap goal_handler to work with SendGoalServiceRequest<T>
        let wrapped_goal =
            move |req: action::SendGoalServiceRequest<T>| -> bool { goal_handler(&req) };
        // Wrap cancel_handler to work with GoalInfo
        let wrapped_cancel = move |info: &GoalInfo| -> bool { cancel_handler(&info.goal_id.uuid) };
        Ok(Self::add_action_server(
            self,
            server,
            wrapped_goal,
            accept_handler,
            wrapped_cancel,
        ))
    }

    fn add_action_client<T: ActionMsg + 'static>(
        &mut self,
        client: Self::ActionClient<T>,
    ) -> oxidros_core::Result<bool> {
        // Action clients are added internally with specific handlers
        // For the trait, just register with no handlers (used for async operations)
        Self::add_action_client(
            self,
            client.inner_data().clone(),
            None,
            None,
            None,
            None,
            None,
        );
        Ok(true)
    }

    fn wait(&mut self) -> oxidros_core::Result<()> {
        Self::wait(self)
    }

    fn wait_timeout(&mut self, timeout: std::time::Duration) -> oxidros_core::Result<bool> {
        Self::wait_timeout(self, timeout)
    }
}

#[cfg(test)]
mod test {
    use crate::{context::Context, error::Result, selector::CallbackResult};
    use std::thread;

    #[test]
    fn test_guard_condition() -> Result<()> {
        let ctx = Context::new()?;
        let cond = super::GuardCondition::new(ctx.clone())?;

        let ctx2 = ctx.clone();
        let cond2 = cond.clone();

        let w = thread::spawn(move || {
            let mut selector = super::Selector::new(ctx2).unwrap();
            selector.add_guard_condition(
                &cond2,
                Some(Box::new(|| {
                    println!("triggerd!");
                    CallbackResult::Ok
                })),
                false,
            );
            selector.wait().unwrap();
        });

        cond.trigger()?;
        w.join().unwrap();

        Ok(())
    }
}
