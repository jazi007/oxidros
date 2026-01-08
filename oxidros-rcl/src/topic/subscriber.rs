//! Subscriber to receive messages.
//!
//! When creating a subscriber, you can specify a QoS profile.
//!
//! # Single and Multi Threaded Receive
//!
//! `try_recv` is a non-blocking function to receive.
//! Use `try_recv` in a callback function for single threaded receive.
//!
//! `recv` returns `AsyncReceiver`, which is a future object,
//! and you can use `.await` to receive a message.
//! See the example of `recv`.
//!
//! # Examples
//!
//! ## Single Threaded Execution
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context, logger::Logger, msg::common_interfaces::std_msgs, pr_error, pr_info,
//!     RecvResult,
//! };
//!
//! let ctx = Context::new().unwrap();
//! let node = ctx
//!     .create_node("subscriber_rs_try_recv", None, Default::default())
//!     .unwrap();
//!
//! // Create a subscriber.
//! let subscriber = node
//!     .create_subscriber::<std_msgs::msg::UInt32>("subscriber_rs_try_recv_topic", None,
//! ).unwrap();
//!
//! // Create a publisher.
//! let publisher = node
//!     .create_publisher::<std_msgs::msg::UInt32>("subscriber_rs_try_recv_topic", None,
//! ).unwrap();
//!
//! let logger = Logger::new("subscriber_rs");
//!
//! // Send a message.
//! let mut msg = std_msgs::msg::UInt32::new().unwrap();
//! msg.data = 10;
//! publisher.send(&msg).unwrap();
//!
//! // Receive the message.
//! match subscriber.try_recv() {
//!     RecvResult::Ok(msg) => pr_info!(logger, "msg = {}", msg.data),
//!     RecvResult::RetryLater => pr_info!(logger, "retry later"),
//!     RecvResult::Err(e) => pr_error!(logger, "error = {}", e),
//! }
//! ```
//!
//! ## Multi Threaded Execution
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context, logger::Logger, msg::common_interfaces::std_msgs, pr_info, pr_warn,
//!     topic::subscriber::Subscriber,
//! };
//! use std::time::Duration;
//!
//! // Create a context.
//! let ctx = Context::new().unwrap();
//!
//! // Create nodes.
//! let node_sub = ctx
//!     .create_node("subscriber_rs_recv", None, Default::default())
//!     .unwrap();
//!
//! // Create a subscriber.
//! let subscriber = node_sub
//!     .create_subscriber::<std_msgs::msg::String>("subscriber_rs_recv_topic", None,
//!     )
//!     .unwrap();
//!
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! // Create tasks.
//! rt.block_on(async {
//!     let s = tokio::task::spawn(run_subscriber(subscriber));
//!     s.await;
//! });
//!
//! /// The subscriber.
//! async fn run_subscriber(mut s: Subscriber<std_msgs::msg::String>) {
//!     let dur = Duration::from_millis(100);
//!     let logger = Logger::new("subscriber_rs_recv");
//!     for _ in 0..3 {
//!         // receive a message specifying timeout of 100ms
//!         match tokio::time::timeout(dur, s.recv()).await {
//!             Ok(Ok(msg)) => {
//!                 // received a message
//!                 pr_info!(logger, "Received (async): msg = {}", msg.data);
//!             }
//!             Ok(Err(e)) => panic!("{}", e), // fatal error
//!             Err(_) => {
//!                 // timeout
//!                 pr_warn!(logger, "Subscribe (async): timeout");
//!                 break;
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Default QoS Profile
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context,
//!     msg::common_interfaces::std_msgs,
//!     qos::HistoryPolicy,
//! };
//!
//! let ctx = Context::new().unwrap();
//! let node = ctx
//! .create_node("subscriber_rs", None, Default::default())
//! .unwrap();
//!
//! // Use default QoS profile.
//! let subscriber = node
//! .create_subscriber::<std_msgs::msg::Empty>("subscriber_rs_topic", None,
//! )
//! .unwrap();
//! ```
//!
//! ## Specifying QoS Profile
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context,
//!     msg::common_interfaces::std_msgs,
//!     qos::{HistoryPolicy, Profile},
//! };
//!
//! let ctx = Context::new().unwrap();
//! let node = ctx
//!     .create_node("subscriber_rs", None, Default::default())
//!     .unwrap();
//!
//! // Create a QoS profile.
//! let mut profile = Profile::default();
//! profile.history = HistoryPolicy::KeepAll;
//!
//! // Specify the QoS profile.
//! let subscriber = node
//!     .create_subscriber::<std_msgs::msg::Empty>("subscriber_rs_topic", Some(profile),
//! ).unwrap();
//! ```
//!
//! `None` of the 2nd argument of `create_subscriber` is equivalent to `Some(Profile::default())`.

use crate::{
    PhantomUnsync, RecvResult,
    error::{OError, OResult, Result},
    get_allocator,
    helper::is_unpin,
    is_halt,
    msg::TypeSupport,
    node::Node,
    qos, rcl,
    selector::async_selector::{self, SELECTOR},
    signal_handler::Signaled,
    topic::subscriber_loaned_message::SubscriberLoanedMessage,
};
pub use oxidros_core::message::TakenMsg;
use oxidros_core::selector::CallbackResult;
use std::{
    ffi::CString,
    future::Future,
    marker::PhantomData,
    os::raw::c_void,
    pin::Pin,
    ptr::null_mut,
    sync::Arc,
    task::{self, Poll},
};

#[cfg(feature = "rcl_stat")]
use crate::helper::statistics::{SerializableTimeStat, TimeStatistics};

#[cfg(feature = "rcl_stat")]
use parking_lot::Mutex;

pub(crate) struct RCLSubscription {
    pub subscription: Box<rcl::rcl_subscription_t>,
    topic_name: String,
    #[cfg(feature = "rcl_stat")]
    pub latency_take: Mutex<TimeStatistics<4096>>,
    pub node: Arc<Node>,
}

#[cfg(feature = "rcl_stat")]
impl RCLSubscription {
    fn measure_latency(&self, start: std::time::SystemTime) {
        if let Ok(dur) = start.elapsed() {
            let mut guard = self.latency_take.lock();
            guard.add(dur);
        }
    }
}

impl Drop for RCLSubscription {
    fn drop(&mut self) {
        let (node, subscription) = (&mut self.node, &mut self.subscription);
        let guard = rcl::MT_UNSAFE_FN.lock();
        let _ = guard.rcl_subscription_fini(subscription.as_mut(), unsafe { node.as_ptr_mut() });
    }
}

unsafe impl Sync for RCLSubscription {}
unsafe impl Send for RCLSubscription {}

/// Subscriber.
pub struct Subscriber<T> {
    pub(crate) subscription: Arc<RCLSubscription>,
    _phantom: PhantomData<T>,
    _unsync: PhantomUnsync,
}

impl<T: TypeSupport> Subscriber<T> {
    pub(crate) fn new(
        node: Arc<Node>,
        topic_name: &str,
        qos: Option<qos::Profile>,
    ) -> OResult<Self> {
        let mut subscription = Box::new(rcl::MTSafeFn::rcl_get_zero_initialized_subscription());

        let topic_name_c = CString::new(topic_name).unwrap_or_default();

        let options = Options::new(&qos.unwrap_or_default());

        {
            let guard = rcl::MT_UNSAFE_FN.lock();

            guard.rcl_subscription_init(
                subscription.as_mut(),
                node.as_ptr(),
                T::type_support() as *const rcl::rosidl_message_type_support_t,
                topic_name_c.as_ptr(),
                options.as_ptr(),
            )?;
        }

        Ok(Subscriber {
            subscription: Arc::new(RCLSubscription {
                subscription,
                node,
                topic_name: topic_name.to_string(),

                #[cfg(feature = "rcl_stat")]
                latency_take: Mutex::new(TimeStatistics::new()),
            }),
            _phantom: Default::default(),
            _unsync: Default::default(),
        })
    }

    pub(crate) fn new_disable_loaned_message(
        node: Arc<Node>,
        topic_name: &str,
        qos: Option<qos::Profile>,
    ) -> OResult<Self> {
        let mut subscription = Box::new(rcl::MTSafeFn::rcl_get_zero_initialized_subscription());
        let topic_name_c = CString::new(topic_name).unwrap_or_default();
        let mut options = Options::new(&qos.unwrap_or_default());
        options.disable_loaned_message();
        {
            let guard = rcl::MT_UNSAFE_FN.lock();

            guard.rcl_subscription_init(
                subscription.as_mut(),
                node.as_ptr(),
                T::type_support() as *const rcl::rosidl_message_type_support_t,
                topic_name_c.as_ptr(),
                options.as_ptr(),
            )?;
        }
        Ok(Subscriber {
            subscription: Arc::new(RCLSubscription {
                subscription,
                node,
                topic_name: topic_name.to_string(),

                #[cfg(feature = "rcl_stat")]
                latency_take: Mutex::new(TimeStatistics::new()),
            }),
            _phantom: Default::default(),
            _unsync: Default::default(),
        })
    }

    pub fn get_topic_name(&self) -> &str {
        &self.subscription.topic_name
    }

    /// Non-blocking receive.
    ///
    /// Because `rcl::rcl_take` is non-blocking,
    /// `try_recv()` returns `RecvResult::RetryLater` if
    /// data is not available.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_msgs, pr_error, pr_info,
    ///     topic::subscriber::Subscriber, RecvResult,
    /// };
    ///
    /// fn pubsub(subscriber: Subscriber<std_msgs::msg::UInt32>, logger: Logger) {
    ///     // Receive the message.
    ///     match subscriber.try_recv() {
    ///         RecvResult::Ok(msg) => pr_info!(logger, "msg = {}", msg.data),
    ///         RecvResult::RetryLater => pr_info!(logger, "retry later"),
    ///         RecvResult::Err(e) => pr_error!(logger, "error = {}", e),
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::SubscriptionInvalid` if the subscription is invalid, or
    /// - `RCLError::BadAlloc if allocating` memory failed, or
    /// - `RCLError::Error` if an unspecified error occurs.
    #[must_use]
    pub fn try_recv(&self) -> RecvResult<TakenMsg<T>> {
        #[cfg(feature = "rcl_stat")]
        let start = std::time::SystemTime::now();

        let s = self.subscription.clone();
        match take::<T>(&s) {
            Ok(n) => {
                #[cfg(feature = "rcl_stat")]
                self.subscription.measure_latency(start);

                RecvResult::Ok(n)
            }
            Err(OError::SubscriptionTakeFailed) => {
                #[cfg(feature = "rcl_stat")]
                self.subscription.measure_latency(start);

                RecvResult::RetryLater
            }
            Err(e) => RecvResult::Err(e.into()),
        }
    }
    /// Blocking receive.
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::SubscriptionInvalid` if the subscription is invalid, or
    /// - `RCLError::BadAlloc if allocating` memory failed, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub fn recv_blocking(&self) -> Result<TakenMsg<T>> {
        let mut selector = self.subscription.node.context.create_selector()?;
        selector.add_rcl_subscription(self.subscription.clone(), None, false);
        loop {
            match self.try_recv() {
                RecvResult::Ok(msg) => return Ok(msg),
                RecvResult::Err(e) => return Err(e),
                RecvResult::RetryLater => {}
            }
            selector.wait()?;
        }
    }

    /// Receive a message asynchronously.
    ///
    /// This waits and blocks forever until a message arrives.
    /// In order to call `recv()` with timeout,
    /// use mechanisms provided by asynchronous libraries,
    /// such as `tokio::time::timeout`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_msgs, pr_info, pr_warn,
    ///     topic::subscriber::Subscriber,
    /// };
    /// use std::time::Duration;
    ///
    /// async fn run_subscriber(mut s: Subscriber<std_msgs::msg::String>) {
    ///     let dur = Duration::from_millis(100);
    ///     let logger = Logger::new("subscriber_rs_recv");
    ///     for _ in 0..3 {
    ///         // receive a message specifying timeout of 100ms
    ///         match tokio::time::timeout(dur, s.recv()).await {
    ///             Ok(Ok(msg)) => {
    ///                 // received a message
    ///                 pr_info!(logger, "Received (async): msg = {}", msg.data);
    ///             }
    ///             Ok(Err(e)) => panic!("{}", e), // fatal error
    ///             Err(_) => {
    ///                 // timeout
    ///                 pr_warn!(logger, "Subscribe (async): timeout");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::SubscriptionInvalid` if the subscription is invalid, or
    /// - `RCLError::BadAlloc` if allocating memory failed, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub async fn recv(&mut self) -> Result<TakenMsg<T>> {
        AsyncReceiver {
            subscriber: self,
            is_waiting: false,
        }
        .await
    }

    /// Get latency statistics information of `Mutex` and `rcl_take()`.
    /// Because `rcl_take()` is MT-UNSAFE, a latency includes not only `rcl_take` but also `Mutex`.
    #[cfg(feature = "rcl_stat")]
    pub fn statistics(&self) -> SerializableTimeStat {
        let guard = self.subscription.latency_take.lock();
        guard.to_serializable()
    }
}

/// Asynchronous receiver of subscribers.
pub struct AsyncReceiver<'a, T> {
    subscriber: &'a mut Subscriber<T>,
    is_waiting: bool,
}

impl<'a, T> AsyncReceiver<'a, T> {
    fn project(self: std::pin::Pin<&mut Self>) -> (&mut Subscriber<T>, &mut bool) {
        // Safety: Arc<RCLSubscription> is Unpin
        is_unpin::<&mut Subscriber<T>>();
        unsafe {
            let this = self.get_unchecked_mut();
            (this.subscriber, &mut this.is_waiting)
        }
    }
}

impl<'a, T: TypeSupport> Future for AsyncReceiver<'a, T> {
    type Output = Result<TakenMsg<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }
        let (subscriber, is_waiting) = self.project();
        *is_waiting = false;
        match subscriber.try_recv() {
            RecvResult::Ok(v) => Poll::Ready(Ok(v)),
            RecvResult::Err(e) => Poll::Ready(Err(e)),
            RecvResult::RetryLater => {
                let mut guard = SELECTOR.lock();
                let mut waker = Some(cx.waker().clone());
                guard.send_command(
                    &subscriber.subscription.node.context,
                    async_selector::Command::Subscription(
                        subscriber.subscription.clone(),
                        Box::new(move || {
                            let w = waker.take();
                            w.unwrap().wake();
                            CallbackResult::Ok
                        }),
                    ),
                )?;
                *is_waiting = true;
                Poll::Pending
            }
        }
    }
}

impl<T> Drop for AsyncReceiver<'_, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.subscriber.subscription.node.context,
                async_selector::Command::RemoveSubscription(self.subscriber.subscription.clone()),
            );
        }
    }
}

/// Options for subscribers.
struct Options {
    options: rcl::rcl_subscription_options_t,
}

impl Options {
    fn new(qos: &qos::Profile) -> Self {
        let options = rcl::rcl_subscription_options_t {
            qos: qos.into(),
            allocator: get_allocator(),
            rmw_subscription_options: rcl::MTSafeFn::rmw_get_default_subscription_options(),

            #[cfg(any(feature = "jazzy", feature = "kilted"))]
            disable_loaned_message: false,
        };
        Options { options }
    }

    fn disable_loaned_message(&mut self) {
        #[cfg(any(feature = "jazzy", feature = "kilted"))]
        {
            self.options.disable_loaned_message = true;
        }
    }

    pub(crate) fn as_ptr(&self) -> *const rcl::rcl_subscription_options_t {
        &self.options
    }
}

fn take<T: 'static>(subscription: &Arc<RCLSubscription>) -> OResult<TakenMsg<T>> {
    if rcl::MTSafeFn::rcl_subscription_can_loan_messages(subscription.subscription.as_ref()) {
        take_loaned_message(subscription.clone()).map(move |x| TakenMsg::Loaned(Box::new(x)))
    } else {
        rcl_take(subscription.subscription.as_ref()).map(TakenMsg::Copied)
    }
}

fn take_loaned_message<T>(
    subscription: Arc<RCLSubscription>,
) -> OResult<SubscriberLoanedMessage<T>> {
    let guard = rcl::MT_UNSAFE_FN.lock();
    let message: *mut T = null_mut();
    guard
        .rcl_take_loaned_message(
            subscription.subscription.as_ref(),
            &message as *const _ as *mut _,
            null_mut(),
            null_mut(),
        )
        .map(|_| SubscriberLoanedMessage::new(subscription, message))
}

fn rcl_take<T>(subscription: &rcl::rcl_subscription_t) -> OResult<T> {
    let guard = rcl::MT_UNSAFE_FN.lock();
    let mut ros_message: T = unsafe { std::mem::zeroed() };
    match guard.rcl_take(
        subscription,
        &mut ros_message as *mut _ as *mut c_void,
        null_mut(),
        null_mut(),
    ) {
        Ok(_) => Ok(ros_message),
        Err(e) => Err(e),
    }
}
