//! Server to receive a request and send the reply.
//!
//! # Examples
//!
//! ## Single Threaded Execution
//!
//! ```
//! use oxidros_rcl::{context::Context, msg::common_interfaces::std_srvs};
//! use std::time::Duration;
//!
//! // Create a context.
//! let ctx = Context::new().unwrap();
//!
//! // Create a server node.
//! let node = ctx
//!     .create_node("service_server_rs", None, Default::default())
//!     .unwrap();
//!
//! // Create a server.
//! let server = node
//!     .create_server::<std_srvs::srv::Empty>("service_name1", None)
//!     .unwrap();
//!
//! // Create a selector.
//! let mut selector = ctx.create_selector().unwrap();
//!
//! // Add a wall timer.
//! selector.add_wall_timer("timer_name", Duration::from_millis(100), Box::new(|| ()));
//!
//! // Add a callback of the server.
//! selector.add_server(
//!     server,
//!     Box::new(|request, header| {
//!         // Create a response.
//!         let response = std_srvs::srv::Empty_Response::new().unwrap();
//!         response
//!     })
//! );
//!
//! for _ in 0..2 {
//!     selector.wait().unwrap();
//! }
//! ```
//!
//! ## Multi Threaded Execution
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context, logger::Logger, msg::common_interfaces::std_srvs, pr_error,
//!     service::server::Server,
//! };
//! use std::time::Duration;
//!
//! // Create a context.
//! let ctx = Context::new().unwrap();
//!
//! // Create a server node.
//! let node = ctx.create_node("service_server_rs", None, Default::default()).unwrap();
//!
//! // Create a server.
//! let server = node
//!     .create_server::<std_srvs::srv::Empty>("service_name1", None)
//!     .unwrap();
//!
//! let logger = Logger::new("service_rs");
//!
//! async fn server_task(mut server: Server<std_srvs::srv::Empty>, logger: Logger) {
//!     loop {
//!         // Receive a request.
//!         let req = server.recv().await;
//!         match req {
//!             Ok((sender, request, _header)) => {
//!                 let response = std_srvs::srv::Empty_Response::new().unwrap();
//!                 match sender.send(&response) {
//!                     Ok(()) => {},                  // Get a new server to handle next request.
//!                     Err(_e) => {}, // Failed to send.
//!                 }
//!             }
//!             Err(e) => {
//!                 pr_error!(logger, "error: {e}");
//!                 return;
//!             }
//!         }
//!     }
//! }
//!
//! // We don't call `server_task` here because testing this code will block forever.
//! // let rt = tokio::runtime::Runtime::new().unwrap(); --- IGNORE ---
//! // rt.block_on(server_task(server, logger)); // Spawn an asynchronous task.
//! ```

use super::Header;
#[cfg(feature = "jazzy")]
use crate::msg::interfaces::rosgraph_msgs::msg::Clock;
use crate::{
    PhantomUnsync, RecvResult,
    error::Result,
    get_allocator,
    helper::is_unpin,
    is_halt,
    msg::ServiceMsg,
    node::Node,
    qos::Profile,
    rcl::{self, rmw_request_id_t},
    selector::async_selector::{self, SELECTOR},
    signal_handler::Signaled,
};
use oxidros_core::{Error, RclError, selector::CallbackResult};
use parking_lot::Mutex;
use std::{
    ffi::CString, future::Future, marker::PhantomData, os::raw::c_void, sync::Arc, task::Poll,
};

pub(crate) struct ServerData {
    pub(crate) service: rcl::rcl_service_t,
    pub(crate) node: Arc<Node>,
}

impl Drop for ServerData {
    fn drop(&mut self) {
        let guard = rcl::MT_UNSAFE_FN.lock();
        let _ = guard.rcl_service_fini(&mut self.service, unsafe { self.node.as_ptr_mut() });
    }
}

#[cfg(any(feature = "jazzy", feature = "kilted"))]
pub enum RCLServiceIntrospection {
    RCLServiceIntrospectionOff,
    RCLServiceIntrospectionMetadata,
    RCLServiceIntrospectionContents,
}

#[cfg(any(feature = "jazzy", feature = "kilted"))]
impl From<rcl::rcl_service_introspection_state_t> for RCLServiceIntrospection {
    fn from(value: rcl::rcl_service_introspection_state_t) -> Self {
        use rcl::rcl_service_introspection_state_t::*;
        match value {
            RCL_SERVICE_INTROSPECTION_OFF => Self::RCLServiceIntrospectionOff,
            RCL_SERVICE_INTROSPECTION_CONTENTS => Self::RCLServiceIntrospectionContents,
            RCL_SERVICE_INTROSPECTION_METADATA => Self::RCLServiceIntrospectionMetadata,
        }
    }
}
#[cfg(any(feature = "jazzy", feature = "kilted"))]
impl From<RCLServiceIntrospection> for rcl::rcl_service_introspection_state_t {
    fn from(value: RCLServiceIntrospection) -> Self {
        use RCLServiceIntrospection::*;
        use rcl::rcl_service_introspection_state_t::*;
        match value {
            RCLServiceIntrospectionOff => RCL_SERVICE_INTROSPECTION_OFF,
            RCLServiceIntrospectionMetadata => RCL_SERVICE_INTROSPECTION_METADATA,
            RCLServiceIntrospectionContents => RCL_SERVICE_INTROSPECTION_CONTENTS,
        }
    }
}

unsafe impl Sync for ServerData {}
unsafe impl Send for ServerData {}

/// Server.
#[must_use]
pub struct Server<T> {
    pub(crate) data: Arc<Mutex<ServerData>>,
    service_name: String,
    _phantom: PhantomData<T>,
    _unsync: PhantomUnsync,
}

impl<T: ServiceMsg> Server<T> {
    pub(crate) fn new(node: Arc<Node>, service_name: &str, qos: Option<Profile>) -> Result<Self> {
        let mut service = rcl::MTSafeFn::rcl_get_zero_initialized_service();
        let service_name_c = CString::new(service_name).unwrap_or_default();
        let profile = qos.unwrap_or_else(Profile::services_default);
        let options = rcl::rcl_service_options_t {
            qos: (&profile).into(),
            allocator: get_allocator(),
        };

        {
            let guard = rcl::MT_UNSAFE_FN.lock();
            guard.rcl_service_init(
                &mut service,
                node.as_ptr(),
                <T as ServiceMsg>::type_support() as *const rcl::rosidl_service_type_support_t,
                service_name_c.as_ptr(),
                &options,
            )?;
        }

        Ok(Server {
            data: Arc::new(Mutex::new(ServerData { service, node })),
            service_name: service_name.to_string(),
            _phantom: Default::default(),
            _unsync: Default::default(),
        })
    }

    #[cfg(feature = "jazzy")]
    pub fn configure_introspection(
        &self,
        clock: &mut Clock,
        qos: Profile,
        introspection_state: RCLServiceIntrospection,
    ) -> Result<()> {
        let mut pub_opts = unsafe { rcl::rcl_publisher_get_default_options() };
        pub_opts.qos = (&qos).into();

        let mut data = self.data.lock();
        let guard = rcl::MT_UNSAFE_FN.lock();

        guard.rcl_service_configure_service_introspection(
            &mut data.service,
            unsafe { data.node.as_ptr_mut() },
            clock as *mut Clock as *mut _,
            <T as ServiceMsg>::type_support() as *const rcl::rosidl_service_type_support_t,
            pub_opts,
            introspection_state.into(),
        )
    }

    /// Receive a request.
    /// `try_recv` is a non-blocking function, and
    /// this returns `RecvResult::RetryLater` if there is no available data.
    /// So, please retry later if this error is returned.
    ///
    /// # Return value
    ///
    /// `RecvResult::Ok((ServerSend<T>, <T as ServiceMsg>::Request, Header))` is returned.
    /// `T` is a type of the request and response.
    /// After receiving a request, `ServerSend<T>` can be used to send a response.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_srvs, pr_error, pr_info, service::server::Server,
    ///     RecvResult,
    /// };
    ///
    /// fn server_fn(mut server: Server<std_srvs::srv::Empty>, logger: Logger) {
    ///     loop {
    ///         match server.try_recv() {
    ///             RecvResult::Ok((sender, request, header)) => {
    ///                 pr_info!(logger, "received: header = {:?}", header);
    ///                 let msg = std_srvs::srv::Empty_Response::new().unwrap();
    ///                 match sender.send(&msg) {
    ///                     Ok(()) => {},                  // Get a new server to handle next request.
    ///                     Err(_e) => {}, // Failed to send.
    ///                 }
    ///             }
    ///             RecvResult::RetryLater => {
    ///                 pr_info!(logger, "retry later");
    ///             }
    ///             RecvResult::Err(e) => {
    ///                 pr_error!(logger, "error: {e}");
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
    /// - `RCLError::ServiceInvalid` if the service is invalid, or
    /// - `RCLError::BadAlloc` if allocating memory failed, or
    /// - `RCLError::Error` if an unspecified error occurs.
    #[must_use]
    pub fn try_recv(&mut self) -> RecvResult<(ServerSend<T>, <T as ServiceMsg>::Request, Header)> {
        let data = self.data.lock();
        let (request, header) =
            match rcl_take_request_with_info::<<T as ServiceMsg>::Request>(&data.service) {
                Ok(data) => data,
                Err(Error::Rcl(RclError::ServiceTakeFailed)) => {
                    drop(data);
                    return RecvResult::RetryLater;
                }
                Err(e) => return RecvResult::Err(e),
            };

        drop(data);
        RecvResult::Ok((
            ServerSend {
                data: self.data.clone(),
                request_id: header.request_id,
                _phantom: Default::default(),
                _unsync: Default::default(),
            },
            request,
            Header { header },
        ))
    }

    /// Receive a request asynchronously.
    ///
    /// # Return value
    ///
    /// `Ok((ServerSend<T>, <T as ServiceMsg>::Request, T1, Header))` is returned.
    /// `T` is a type of the request and response.
    /// After receiving a request, `ServerSend<T>` can be used to send a response.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_srvs, pr_error, pr_info, service::server::Server,
    /// };
    ///
    /// async fn server_task(mut server: Server<std_srvs::srv::Empty>, logger: Logger) {
    ///     loop {
    ///         // Receive a request.
    ///         let req = server.recv().await;
    ///         match req {
    ///             Ok((sender, request, header)) => {
    ///                 pr_info!(logger, "recv: header = {:?}", header);
    ///                 let response = std_srvs::srv::Empty_Response::new().unwrap();
    ///                 match sender.send(&response) {
    ///                     Ok(()) => {},                  // Get a new server to handle next request.
    ///                     Err(_e) => {}, // Failed to send.
    ///                 }
    ///             }
    ///             Err(e) => {
    ///                 pr_error!(logger, "error: {e}");
    ///                 return;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::ServiceInvalid` if the service is invalid, or
    /// - `RCLError::BadAlloc` if allocating memory failed, or
    /// - `RCLError::Error` if an unspecified error occurs.
    pub async fn recv(&mut self) -> Result<(ServerSend<T>, <T as ServiceMsg>::Request, Header)> {
        AsyncReceiver {
            server: self,
            is_waiting: false,
        }
        .await
    }
}

unsafe impl<T> Send for Server<T> {}

/// Sender to send a response.
#[must_use]
pub struct ServerSend<T> {
    data: Arc<Mutex<ServerData>>,
    request_id: rmw_request_id_t,
    _phantom: PhantomData<T>,
    _unsync: PhantomUnsync,
}

impl<T: ServiceMsg> ServerSend<T> {
    /// Send a response to the client.
    ///
    /// # Errors
    ///
    /// - `RCLError::InvalidArgument` if any arguments are invalid, or
    /// - `RCLError::ServiceInvalid` if the service is invalid, or
    /// - `RCLError::Error` if an unspecified error occurs.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidros_rcl::{
    ///     logger::Logger, msg::common_interfaces::std_srvs, pr_error, service::server::Server,
    /// };
    ///
    /// async fn server_task(mut server: Server<std_srvs::srv::Empty>, logger: Logger) {
    ///     loop {
    ///         // Call recv() by using timeout.
    ///         let req = server.recv().await;
    ///         match req {
    ///             Ok((sender, request, _header)) => {
    ///                 let response = std_srvs::srv::Empty_Response::new().unwrap();
    ///                 match sender.send(&response) {
    ///                     Ok(()) => {},                  // Get a new server to handle next request.
    ///                     Err(_e) => {}, // Failed to send.
    ///                 }
    ///             }
    ///             Err(e) => {
    ///                 pr_error!(logger, "error: {e}");
    ///                 return;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// `data` should be immutable, but `rcl_send_response` provided
    /// by ROS2 takes normal pointers instead of `const` pointers.
    /// So, currently, `send` takes `data` as mutable.
    pub fn send(mut self, data: &<T as ServiceMsg>::Response) -> Result<()> {
        let server_data = self.data.lock();
        rcl::MTSafeFn::rcl_send_response(
            &server_data.service,
            &mut self.request_id,
            data as *const _ as *mut c_void,
        )
    }
}

fn rcl_take_request_with_info<T>(
    service: &rcl::rcl_service_t,
) -> Result<(T, rcl::rmw_service_info_t)> {
    let mut header: rcl::rmw_service_info_t = unsafe { std::mem::zeroed() };
    let mut ros_request: T = unsafe { std::mem::zeroed() };

    let guard = rcl::MT_UNSAFE_FN.lock();
    guard.rcl_take_request_with_info(
        service,
        &mut header,
        &mut ros_request as *mut _ as *mut c_void,
    )?;

    Ok((ros_request, header))
}

/// Receiver to receive a request asynchronously.
#[must_use]
pub struct AsyncReceiver<'a, T> {
    server: &'a mut Server<T>,
    is_waiting: bool,
}

impl<'a, T> AsyncReceiver<'a, T> {
    fn project(self: std::pin::Pin<&mut Self>) -> (&mut Server<T>, &mut bool) {
        // Safety: Server is Unpin
        is_unpin::<&mut Server<T>>();
        unsafe {
            let this = self.get_unchecked_mut();
            (this.server, &mut this.is_waiting)
        }
    }
}

impl<'a, T: ServiceMsg> Future for AsyncReceiver<'a, T> {
    type Output = Result<(ServerSend<T>, <T as ServiceMsg>::Request, Header)>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let (server, is_waiting) = self.project();
        *is_waiting = false;
        let data = server.data.clone();
        match server.try_recv() {
            RecvResult::Ok(v) => Poll::Ready(Ok(v)),
            RecvResult::Err(e) => Poll::Ready(Err(e)),
            RecvResult::RetryLater => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();
                if let Err(e) = guard.send_command(
                    &data.lock().node.context,
                    async_selector::Command::Server(
                        data.clone(),
                        Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Ok
                        }),
                    ),
                ) {
                    return Poll::Ready(Err(e));
                }
                *is_waiting = true;
                Poll::Pending
            }
        }
    }
}

impl<'a, T> Drop for AsyncReceiver<'a, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let cloned = self.server.data.clone();
            let data = self.server.data.lock();
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &data.node.context,
                async_selector::Command::RemoveServer(cloned),
            );
        }
    }
}

// ============================================================================
// RosServer trait implementation
// ============================================================================

/// A request wrapper that implements `ServiceRequest` for the API traits.
pub struct RclServiceRequest<T: ServiceMsg> {
    sender: ServerSend<T>,
    request: <T as ServiceMsg>::Request,
}

impl<T: ServiceMsg> oxidros_core::api::ServiceRequest<T> for RclServiceRequest<T> {
    fn request(&self) -> &T::Request {
        &self.request
    }

    fn respond(self, response: T::Response) -> oxidros_core::Result<()> {
        // ServerSend::send returns Result which uses RclError
        self.sender.send(&response)
    }
}

impl<T: ServiceMsg> oxidros_core::api::RosServer<T> for Server<T> {
    type Request = RclServiceRequest<T>;

    fn service_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.service_name)
    }

    async fn recv_request(&mut self) -> oxidros_core::Result<Self::Request> {
        // Server::recv returns Result which already uses oxidros_core::Error
        let (sender, request, _header) = self.recv().await?;
        Ok(RclServiceRequest { sender, request })
    }

    fn try_recv_request(&mut self) -> oxidros_core::Result<Option<Self::Request>> {
        match self.try_recv() {
            RecvResult::Ok((sender, request, _header)) => {
                Ok(Some(RclServiceRequest { sender, request }))
            }
            RecvResult::RetryLater => Ok(None),
            // RecvResult::Err contains oxidros_core::Error already
            RecvResult::Err(e) => Err(e),
        }
    }
}
