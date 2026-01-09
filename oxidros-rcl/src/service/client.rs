//! Client to send a request and receive the reply.
//!
//! The callback execution is not suitable for request and response based communications.
//! So, use async/await to use `Client`.
//!
//! # Example
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context, logger::Logger, msg::common_interfaces::std_srvs, pr_error, pr_info,
//!     pr_warn, service::client::Client,
//! };
//! use std::time::Duration;
//!
//! // Create a context.
//! let ctx = Context::new().unwrap();
//!
//! // Create a server node.
//! let node = ctx
//!     .create_node("service_client_rs", None, Default::default())
//!     .unwrap();
//!
//! // Create a client.
//! let mut client = node
//!     .create_client::<std_srvs::srv::Empty>("service_name1", None)
//!     .unwrap();
//!
//! // Create a logger.
//! let logger = Logger::new("client_rs");
//!
//! async fn run_client(mut client: Client<std_srvs::srv::Empty>, logger: Logger) {
//!     let dur = Duration::from_millis(100);
//!
//!     for _ in 0..5 {
//!         let request = std_srvs::srv::Empty_Request::new().unwrap();
//!         let receiver = client.send(&request).unwrap().recv();
//!         match tokio::time::timeout(dur, receiver).await {
//!             Ok(Ok((response, _header))) => {
//!                 pr_info!(logger, "received: {:?}", response);
//!             }
//!             Ok(Err(e)) => {
//!                 pr_error!(logger, "error: {e}");
//!                 break;
//!             }
//!             Err(_) => {
//!                 pr_warn!(logger, "timeout");
//!             }
//!         }
//!     }
//! }
//!
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(run_client(client, logger)); // Spawn an asynchronous task.
//! ```

use super::Header;
use crate::{
    RecvResult,
    error::{OError, OResult, Result},
    get_allocator, is_halt,
    msg::ServiceMsg,
    node::Node,
    qos::Profile,
    rcl,
    selector::{
        Selector,
        async_selector::{self, SELECTOR},
    },
    signal_handler::Signaled,
};
use oxidros_core::selector::CallbackResult;
use std::{
    ffi::CString, future::Future, marker::PhantomData, os::raw::c_void, sync::Arc, task::Poll,
    time::Duration,
};

pub(crate) struct ClientData {
    pub(crate) client: rcl::rcl_client_t,
    pub(crate) node: Arc<Node>,
}

impl Drop for ClientData {
    fn drop(&mut self) {
        let guard = rcl::MT_UNSAFE_FN.lock();
        let _ = guard.rcl_client_fini(&mut self.client, unsafe { self.node.as_ptr_mut() });
    }
}

unsafe impl Sync for ClientData {}
unsafe impl Send for ClientData {}

/// Client.
pub struct Client<T: ServiceMsg> {
    pub(crate) data: Arc<ClientData>,
    service_name: String,
    _phantom: PhantomData<T>,
}

impl<T: ServiceMsg> Client<T> {
    pub(crate) fn new(node: Arc<Node>, service_name: &str, qos: Option<Profile>) -> OResult<Self> {
        let mut client = rcl::MTSafeFn::rcl_get_zero_initialized_client();
        let service_name_c = CString::new(service_name).unwrap_or_default();
        let profile = qos.unwrap_or_else(Profile::services_default);
        let options = rcl::rcl_client_options_t {
            qos: (&profile).into(),
            allocator: get_allocator(),
        };

        let guard = rcl::MT_UNSAFE_FN.lock();
        guard.rcl_client_init(
            &mut client,
            node.as_ptr(),
            <T as ServiceMsg>::type_support() as *const rcl::rosidl_service_type_support_t,
            service_name_c.as_ptr(),
            &options,
        )?;

        Ok(Client {
            data: Arc::new(ClientData { client, node }),
            service_name: service_name.to_string(),
            _phantom: Default::default(),
        })
    }

    /// Check if service is available
    /// # Errors
    ///
    /// - `RCLError::NodeInvalid`  if the node is invalid, or
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub fn is_service_available(&self) -> OResult<bool> {
        let guard = rcl::MT_UNSAFE_FN.lock();
        guard.rcl_service_server_is_available(self.data.node.as_ptr(), &self.data.client)
    }

    /// Send a request.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_srvs, pr_error, pr_info, pr_warn, service::client::Client,
    /// };
    /// use std::time::Duration;
    ///
    /// async fn run_client(mut client: Client<std_srvs::srv::Empty>, logger: Logger) {
    ///     let dur = Duration::from_millis(100);
    ///
    ///     loop {
    ///         let request = std_srvs::srv::Empty_Request::new().unwrap();
    ///         let receiver = client.send(&request).unwrap().recv();
    ///         match tokio::time::timeout(dur, receiver).await {
    ///             Ok(Ok((response, _header))) => {
    ///                 pr_info!(logger, "received: {:?}", response);
    ///             }
    ///             Ok(Err(e)) => {
    ///                 pr_error!(logger, "error: {e}");
    ///                 break;
    ///             }
    ///             Err(_) => {
    ///                 pr_warn!(logger, "timeout");
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::ClientInvalid` if the client is invalid, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub fn send(&mut self, data: &<T as ServiceMsg>::Request) -> OResult<ClientRecv<'_, T>> {
        let (s, _) = self.send_ret_seq(data)?;
        Ok(s)
    }

    /// `send_ret_seq` is equivalent to `send`, but this returns
    /// the sequence number together.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_srvs, pr_error, pr_info, pr_warn, service::client::Client,
    /// };
    /// use std::time::Duration;
    ///
    /// async fn run_client(mut client: Client<std_srvs::srv::Empty>, logger: Logger) {
    ///     let dur = Duration::from_millis(100);
    ///
    ///     loop {
    ///         let request = std_srvs::srv::Empty_Request::new().unwrap();
    ///         let (receiver, sequence) = client.send_ret_seq(&request).unwrap();
    ///         let receiver = receiver.recv();
    ///         pr_info!(logger, "sent: sequence = {sequence}");
    ///         match tokio::time::timeout(dur, receiver).await {
    ///             Ok(Ok((response, _header))) => {
    ///                 pr_info!(logger, "received: {:?}", response);
    ///             }
    ///             Ok(Err(e)) => {
    ///                 pr_error!(logger, "error: {e}");
    ///                 break;
    ///             }
    ///             Err(_) => {
    ///                 pr_warn!(logger, "timeout");
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::ClientInvalid` if the client is invalid, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub fn send_ret_seq(
        &mut self,
        data: &<T as ServiceMsg>::Request,
    ) -> OResult<(ClientRecv<'_, T>, i64)> {
        let mut seq: i64 = 0;
        rcl::MTSafeFn::rcl_send_request(
            &self.data.client,
            data as *const _ as *const c_void,
            &mut seq,
        )?;
        Ok((ClientRecv { data: self, seq }, seq))
    }
}

/// Receiver to receive a response.
#[must_use]
pub struct ClientRecv<'a, T: ServiceMsg> {
    pub(crate) data: &'a mut Client<T>,
    pub(crate) seq: i64,
}

impl<'a, T: ServiceMsg> ClientRecv<'a, T> {
    /// Receive a message.
    /// `try_recv` is a non-blocking function, and this
    /// returns `RecvResult::RetryLater`.
    /// So, please retry later if this value is returned.
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::ClientInvalid` if the client is invalid, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub fn try_recv(&self) -> RecvResult<(<T as ServiceMsg>::Response, Header)> {
        let (response, header) = match rcl_take_response_with_info::<<T as ServiceMsg>::Response>(
            &self.data.data.client,
            self.seq,
        ) {
            Ok(data) => data,
            Err(OError::ClientTakeFailed) => return RecvResult::RetryLater,
            Err(e) => return RecvResult::Err(e.into()),
        };

        if header.request_id.sequence_number != self.seq {
            return RecvResult::RetryLater;
        }

        RecvResult::Ok((response, Header { header }))
    }

    /// Receive a response asynchronously.
    /// this returns `super::Header` including some information together.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_srvs, pr_error, pr_info, pr_warn, service::client::Client,
    /// };
    /// use std::time::Duration;
    ///
    /// async fn run_client(mut client: Client<std_srvs::srv::Empty>, logger: Logger) {
    ///     let dur = Duration::from_millis(100);
    ///
    ///     loop {
    ///         let request = std_srvs::srv::Empty_Request::new().unwrap();
    ///         let receiver = client.send(&request).unwrap().recv();
    ///         match tokio::time::timeout(dur, receiver).await {
    ///             Ok(Ok((response, header))) => {
    ///                 pr_info!(logger, "received: header = {:?}", header);
    ///             }
    ///             Ok(Err(e)) => {
    ///                 pr_error!(logger, "error: {e}");
    ///                 break;
    ///             }
    ///             Err(_) => {
    ///                 pr_warn!(logger, "timeout");
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::ClientInvalid` if the client is invalid, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub async fn recv(self) -> Result<(<T as ServiceMsg>::Response, Header)> {
        AsyncReceiver {
            client: self,
            is_waiting: false,
        }
        .await
    }

    /// Receive a message.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     error::Result,
    ///     logger::Logger,
    ///     msg::common_interfaces::{std_msgs, std_srvs},
    ///     pr_fatal,
    ///     selector::Selector,
    ///     service::client::Client,
    ///     topic::subscriber::Subscriber,
    ///     RecvResult,
    /// };
    /// use std::time::Duration;
    ///
    /// fn worker(
    ///     mut selector: Selector,
    ///     mut selector_client: Selector,
    ///     subscriber: Subscriber<std_msgs::msg::Empty>,
    ///     mut client: Client<std_srvs::srv::Empty>,
    /// ) -> Result<()> {
    ///     let logger = Logger::new("listen_client");
    ///
    ///     selector.add_subscriber(
    ///         subscriber,
    ///         Box::new(move |_msg| {
    ///             let request = std_srvs::srv::Empty_Request::new().unwrap();
    ///             // Send a request.
    ///             let receiver = client.send(&request).unwrap();
    ///             // Receive a response.
    ///             match receiver.recv_timeout(Duration::from_millis(20), &mut selector_client) {
    ///                 RecvResult::Ok((_response, _header)) => {},
    ///                 RecvResult::RetryLater => {},
    ///                 RecvResult::Err(e) => {
    ///                     pr_fatal!(logger, "{e}");
    ///                     panic!()
    ///                 }
    ///             }
    ///         }),
    ///     );
    ///
    ///     loop {
    ///         selector.wait()?;
    ///     }
    /// }
    /// ```
    pub fn recv_timeout(
        &self,
        t: Duration,
        selector: &mut Selector,
    ) -> RecvResult<(<T as ServiceMsg>::Response, Header)> {
        // Add the receiver.
        selector.add_client_recv(self);
        // Wait a response with timeout.
        match selector.wait_timeout(t) {
            Ok(true) => match self.try_recv() {
                RecvResult::Ok((response, header)) => {
                    // Received a response.
                    RecvResult::Ok((response, header))
                }
                RecvResult::RetryLater => {
                    // No correspondent response.
                    RecvResult::RetryLater
                }
                RecvResult::Err(e) => {
                    // Failed to receive.
                    RecvResult::Err(e)
                }
            },
            Ok(false) => {
                // Timeout.
                RecvResult::RetryLater
            }
            Err(e) => {
                // Failed to wait.
                RecvResult::Err(e)
            }
        }
    }
}

fn rcl_take_response_with_info<T>(
    client: &rcl::rcl_client_t,
    seq: i64,
) -> OResult<(T, rcl::rmw_service_info_t)> {
    let mut header: rcl::rmw_service_info_t = unsafe { std::mem::zeroed() };
    let mut ros_response: T = unsafe { std::mem::zeroed() };

    header.request_id.sequence_number = seq;

    let guard = rcl::MT_UNSAFE_FN.lock();
    guard.rcl_take_response_with_info(
        client,
        &mut header,
        &mut ros_response as *mut _ as *mut c_void,
    )?;

    Ok((ros_response, header))
}

/// Receiver to receive a response asynchronously.
#[must_use]
pub struct AsyncReceiver<'a, T: ServiceMsg> {
    client: ClientRecv<'a, T>,
    is_waiting: bool,
}

impl<'a, T: ServiceMsg> Future for AsyncReceiver<'a, T> {
    type Output = Result<(<T as ServiceMsg>::Response, Header)>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }
        let mut this = self.as_mut();
        this.is_waiting = false;
        match this.client.try_recv() {
            RecvResult::Ok(v) => return Poll::Ready(Ok(v)),
            RecvResult::RetryLater => (),
            RecvResult::Err(e) => return Poll::Ready(Err(e)),
        }
        // wait message arrival
        let mut waker = Some(cx.waker().clone());
        let mut guard = SELECTOR.lock();
        if let Err(e) = guard.send_command(
            &this.client.data.data.node.context,
            async_selector::Command::Client(
                this.client.data.data.clone(),
                Box::new(move || {
                    let w = waker.take().unwrap();
                    w.wake();
                    CallbackResult::Ok
                }),
            ),
        ) {
            return Poll::Ready(Err(e));
        }
        this.is_waiting = true;
        Poll::Pending
    }
}

impl<'a, T: ServiceMsg> Drop for AsyncReceiver<'a, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.client.data.data.node.context,
                async_selector::Command::RemoveClient(self.client.data.data.clone()),
            );
        }
    }
}

// ============================================================================
// RosClient trait implementation
// ============================================================================

impl<T: ServiceMsg> oxidros_core::api::RosClient<T> for Client<T> {
    fn service_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.service_name)
    }

    fn service_available(&self) -> bool {
        self.is_service_available().unwrap_or(false)
    }

    async fn call_service(&mut self, request: &T::Request) -> oxidros_core::Result<T::Response> {
        let (response, _header) = self.send(request)?.recv().await?;
        Ok(response)
    }
}
