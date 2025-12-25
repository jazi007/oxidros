//! Action server.

use futures_util::try_join;
use oxidros_core::selector::CallbackResult;
use oxidros_core::{
    DurabilityPolicy, HistoryPolicy, LivelinessPolicy, ReliabilityPolicy, TryClone,
};
use oxidros_msg::interfaces::action_msgs::srv::CancelGoal_Response_Constants::{
    ERROR_GOAL_TERMINATED, ERROR_NONE, ERROR_REJECTED, ERROR_UNKNOWN_GOAL_ID,
};
use oxidros_msg::interfaces::unique_identifier_msgs::msg::UUID;
use parking_lot::Mutex;
use pin_project::{pin_project, pinned_drop};
use std::future::Future;
use std::marker::PhantomData;
use std::{collections::BTreeMap, ffi::CString, pin::Pin, sync::Arc, task::Poll, time::Duration};

use crate::logger::{pr_error_in, Logger};
use crate::msg::GetUUID;
use crate::PhantomUnsync;
use crate::{
    clock::Clock,
    error::{DynError, RCLActionError, RCLActionResult},
    get_allocator, is_halt,
    msg::{
        builtin_interfaces::UnsafeTime, interfaces::action_msgs::msg::GoalInfo, ActionGoal,
        ActionMsg, GoalResponse,
    },
    node::Node,
    qos::Profile,
    rcl::{
        self, action_msgs__msg__GoalInfo, action_msgs__msg__GoalInfo__Sequence,
        rcl_action_cancel_request_t, rcl_action_goal_handle_t, rcl_action_server_t,
        rmw_request_id_t, unique_identifier_msgs__msg__UUID,
    },
    selector::async_selector::{Command, SELECTOR},
    signal_handler::Signaled,
    RecvResult,
};

use super::GoalEvent;
use super::{handle::GoalHandle, GetResultServiceRequest, GoalStatus, SendGoalServiceRequest};

pub struct ServerQosOption {
    pub goal_service: Profile,
    pub result_service: Profile,
    pub cancel_service: Profile,
    pub feedback_topic: Profile,
    pub status_topic: Profile,
    pub result_timeout: Duration,
}

impl Default for ServerQosOption {
    fn default() -> Self {
        let status_topic_profile = Profile {
            history: HistoryPolicy::KeepLast,
            depth: 1,
            reliability: ReliabilityPolicy::Reliable,
            durability: DurabilityPolicy::TransientLocal,
            liveliness: LivelinessPolicy::SystemDefault,
            avoid_ros_namespace_conventions: false,
            ..Default::default()
        };

        Self {
            goal_service: Profile::services_default(),
            result_service: Profile::services_default(),
            cancel_service: Profile::services_default(),
            feedback_topic: Profile::default(),
            status_topic: status_topic_profile,
            result_timeout: Duration::from_secs(15 * 60),
        }
    }
}

impl From<ServerQosOption> for rcl::rcl_action_server_options_t {
    fn from(opts: ServerQosOption) -> Self {
        rcl::rcl_action_server_options_t {
            goal_service_qos: (&opts.goal_service).into(),
            cancel_service_qos: (&opts.cancel_service).into(),
            result_service_qos: (&opts.result_service).into(),
            feedback_topic_qos: (&opts.feedback_topic).into(),
            status_topic_qos: (&opts.status_topic).into(),
            allocator: get_allocator(),
            result_timeout: rcl::rcl_duration_t {
                nanoseconds: opts.result_timeout.as_nanos() as i64,
            },
        }
    }
}

pub(crate) struct ServerData {
    pub(crate) server: rcl::rcl_action_server_t,
    pub node: Arc<Node>,
    pub(crate) clock: Mutex<Clock>,
    pub(crate) pending_result_requests: Mutex<BTreeMap<[u8; 16], Vec<rmw_request_id_t>>>,
}

impl ServerData {
    pub(crate) unsafe fn as_ptr_mut(&self) -> *mut rcl::rcl_action_server_t {
        &self.server as *const _ as *mut _
    }

    pub(crate) fn publish_goal_status(&self) -> RCLActionResult<()> {
        let guard = rcl::MT_UNSAFE_FN.lock();

        let mut statuses = rcl::MTSafeFn::rcl_action_get_zero_initialized_goal_status_array();
        guard
            .rcl_action_get_goal_status_array(&self.server, &mut statuses)
            .unwrap();

        guard
            .rcl_action_publish_status(&self.server, &statuses as *const _ as *const _)
            .unwrap();

        Ok(())
    }
}

unsafe impl Sync for ServerData {}
unsafe impl Send for ServerData {}

impl Drop for ServerData {
    fn drop(&mut self) {
        let guard = rcl::MT_UNSAFE_FN.lock();
        let _ = guard.rcl_action_server_fini(unsafe { self.as_ptr_mut() }, unsafe {
            self.node.as_ptr_mut()
        });
    }
}

/// An action server.
///
/// Pass this `Server<T>` to [`AsyncServer<T>`] to receive requests on async/await context.
pub struct Server<T: ActionMsg> {
    pub(crate) data: Arc<ServerData>,
    /// Once the server has completed the result for a goal, it is kept here and the result requests are responsed with the result value in this map.
    pub(crate) results: Arc<Mutex<BTreeMap<[u8; 16], T::ResultContent>>>,
    pub(crate) handles: Arc<Mutex<BTreeMap<[u8; 16], GoalHandle<T>>>>,
}

unsafe impl<T> Send for Server<T> where T: ActionMsg {}
unsafe impl<T> Sync for Server<T> where T: ActionMsg {}

impl<T> Server<T>
where
    T: ActionMsg,
{
    /// Create a server.
    pub fn new(
        node: Arc<Node>,
        action_name: &str,
        qos: Option<ServerQosOption>,
    ) -> RCLActionResult<Self> {
        let mut server = rcl::MTSafeFn::rcl_action_get_zero_initialized_server();
        let options = qos
            .map(rcl::rcl_action_server_options_t::from)
            .unwrap_or_else(rcl::MTSafeFn::rcl_action_server_get_default_options);
        let clock = Clock::new()?;
        let action_name = CString::new(action_name).unwrap_or_default();

        {
            let guard = rcl::MT_UNSAFE_FN.lock();
            guard.rcl_action_server_init(
                &mut server,
                unsafe { node.as_ptr_mut() },
                unsafe { clock.as_ptr_mut() },
                T::type_support() as *const rcl::rosidl_action_type_support_t,
                action_name.as_ptr(),
                &options,
            )?;
        }

        let server = Self {
            data: Arc::new(ServerData {
                server,
                node,
                clock: Mutex::new(clock),
                pending_result_requests: Mutex::new(BTreeMap::new()),
            }),
            results: Arc::new(Mutex::new(BTreeMap::new())),
            handles: Arc::new(Mutex::new(BTreeMap::new())),
        };

        Ok(server)
    }

    pub fn try_recv_goal_request(
        &mut self,
    ) -> RecvResult<(ServerGoalSend<'_, T>, SendGoalServiceRequest<T>)> {
        match rcl_action_take_goal_request::<T>(&self.data.server) {
            Ok((header, request)) => {
                let sender = ServerGoalSend {
                    server: self.clone(),
                    goal_id: *request.get_uuid(),
                    request_id: header,
                    _phantom: PhantomData,
                    _unsync: Default::default(),
                };
                RecvResult::Ok((sender, request))
            }
            Err(RCLActionError::ServerTakeFailed) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e.into()),
        }
    }

    pub fn try_recv_cancel_request(
        &mut self,
    ) -> RecvResult<(
        ServerCancelSend<'_, T>,
        rcl_action_cancel_request_t,
        Vec<GoalInfo>,
    )> {
        match rcl_recv_cancel_request(&self.data.server) {
            Ok((header, request, goals)) => {
                // return sender
                let sender = ServerCancelSend {
                    server: self.clone(),
                    request_id: header,
                    _phantom: PhantomData,
                    _unsync: Default::default(),
                };
                RecvResult::Ok((sender, request, goals))
            }
            Err(RCLActionError::ServerTakeFailed) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e.into()),
        }
    }

    pub fn try_recv_result_request(
        &mut self,
    ) -> RecvResult<(ServerResultSend<'_, T>, GetResultServiceRequest<T>)> {
        match rcl_action_take_result_request::<T>(&self.data.server) {
            Ok((header, request)) => {
                let sender = ServerResultSend {
                    server: self.clone(),
                    request_id: header,
                    _phantom: PhantomData,
                    _unsync: Default::default(),
                };
                RecvResult::Ok((sender, request))
            }
            Err(RCLActionError::ServerTakeFailed) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e.into()),
        }
    }

    pub fn try_recv_data(&mut self) -> Result<(), DynError> {
        let _ = self.try_recv_result_request();
        Ok(())
    }

    pub async fn recv_goal_request(
        &mut self,
    ) -> Result<(ServerGoalSend<'_, T>, SendGoalServiceRequest<T>), DynError> {
        AsyncGoalReceiver {
            server: self,
            is_waiting: false,
        }
        .await
    }

    pub async fn recv_cancel_request(
        &mut self,
    ) -> Result<(ServerCancelSend<'_, T>, Vec<GoalInfo>), DynError> {
        AsyncCancelReceiver {
            server: self,
            is_waiting: false,
        }
        .await
    }

    pub async fn recv_result_request(
        &mut self,
    ) -> Result<(ServerResultSend<'_, T>, GetResultServiceRequest<T>), DynError> {
        AsyncResultReceiver {
            server: self,
            is_waiting: false,
        }
        .await
    }
}

pub struct ServerGoalSend<'a, T: ActionMsg> {
    server: Server<T>,
    request_id: rmw_request_id_t,
    goal_id: [u8; 16],
    _phantom: PhantomData<&'a T>,
    _unsync: PhantomUnsync,
}

impl<'a, T: ActionMsg> ServerGoalSend<'a, T> {
    /// Accept the goal request.
    pub fn accept<F>(self, handler: F) -> Result<(), DynError>
    where
        F: FnOnce(GoalHandle<T>),
    {
        let timestamp = {
            let mut clock = self.server.data.clock.lock();
            get_timestamp(&mut clock)
        };
        let handle = self.accept_goal(timestamp)?;
        {
            let mut handles = self.server.handles.lock();
            handler(handle.clone());
            handles.insert(self.goal_id, handle);
        }
        self.send(true, timestamp)
    }

    /// Reject the goal request.
    pub fn reject(self) -> Result<(), DynError> {
        let timestamp = {
            let mut clock = self.server.data.clock.lock();
            get_timestamp(&mut clock)
        };
        self.send(false, timestamp)
    }

    /// Send a response for SendGoal service, and accept the goal if `accepted` is true.
    fn send(mut self, accepted: bool, timestamp: UnsafeTime) -> Result<(), DynError> {
        // TODO: Make SendgoalServiceResponse independent of T (edit safe-drive-msg)
        type GoalResponse<T> = <<T as ActionMsg>::Goal as ActionGoal>::Response;
        let mut response = GoalResponse::<T>::new(accepted, timestamp);
        // send response to client
        let guard = rcl::MT_UNSAFE_FN.lock();
        guard.rcl_action_send_goal_response(
            unsafe { self.server.data.as_ptr_mut() },
            &mut self.request_id,
            &mut response as *const _ as *mut _,
        )?;
        Ok(())
    }

    fn accept_goal(&self, timestamp: UnsafeTime) -> Result<GoalHandle<T>, DynError> {
        // see rcl_interfaces/action_msgs/msg/GoalInfo.msg for definition
        let mut goal_info = rcl::MTSafeFn::rcl_action_get_zero_initialized_goal_info();
        goal_info.goal_id = unique_identifier_msgs__msg__UUID { uuid: self.goal_id };
        goal_info.stamp.sec = timestamp.sec;
        goal_info.stamp.nanosec = timestamp.nanosec;
        let server_ptr = unsafe { self.server.data.as_ptr_mut() };
        let handle_t = rcl_action_accept_new_goal(server_ptr, &goal_info)?;
        let handle = GoalHandle::new(
            self.goal_id,
            handle_t,
            self.server.data.clone(),
            self.server.results.clone(),
        );

        handle.update(GoalEvent::Execute)?;
        self.server.data.publish_goal_status()?;

        Ok(handle)
    }
}

#[pin_project(PinnedDrop)]
#[must_use]
pub struct AsyncGoalReceiver<'a, T: ActionMsg> {
    server: &'a mut Server<T>,
    is_waiting: bool,
}

impl<'a, T: ActionMsg> Future for AsyncGoalReceiver<'a, T> {
    type Output = Result<(ServerGoalSend<'a, T>, SendGoalServiceRequest<T>), DynError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let this = self.project();
        *this.is_waiting = false;

        match rcl_action_take_goal_request::<T>(&this.server.data.server) {
            Ok((header, request)) => {
                let sender = ServerGoalSend {
                    server: this.server.clone(),
                    goal_id: *request.get_uuid(),
                    request_id: header,
                    _phantom: PhantomData,
                    _unsync: Default::default(),
                };
                Poll::Ready(Ok((sender, request)))
            }
            Err(RCLActionError::ServerTakeFailed) => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();

                let cmd = Command::ActionServer {
                    data: this.server.data.clone(),
                    goal: Box::new(move || {
                        let w = waker.take().unwrap();
                        w.wake();
                        CallbackResult::Remove
                    }),
                    cancel: Box::new(move || CallbackResult::Ok),
                    result: Box::new(move || CallbackResult::Ok),
                };
                match guard.send_command(&this.server.data.node.context, cmd) {
                    Ok(_) => {
                        *this.is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
            Err(e) => Poll::Ready(Err(e.into())),
        }
    }
}

#[pinned_drop]
impl<'a, T: ActionMsg> PinnedDrop for AsyncGoalReceiver<'a, T> {
    fn drop(self: Pin<&mut Self>) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.server.data.node.context,
                Command::RemoveActionServer(self.server.data.clone()),
            );
        }
    }
}

pub struct ServerCancelSend<'a, T: ActionMsg> {
    server: Server<T>,
    request_id: rmw_request_id_t,
    _phantom: PhantomData<&'a T>,
    _unsync: PhantomUnsync,
}

impl<'a, T: ActionMsg> ServerCancelSend<'a, T> {
    /// Accept the cancel requests for accepted_goals and set them to CANCELING state.
    /// `accepted_goals` can be empty if no goals are to be canceled.
    /// The shutdown operation fo each goal should be performed after calling send(),
    /// and use [`GoalHandle::canceled`] when it is done.
    pub fn send(mut self, mut accepted_goals: Vec<GoalInfo>) -> Result<(), DynError> {
        let mut response = rcl::MTSafeFn::rcl_action_get_zero_initialized_cancel_response();

        let code = self.cancel_goals(&accepted_goals)?;
        response.msg.return_code = code;
        if code == ERROR_NONE {
            response.msg.goals_canceling = action_msgs__msg__GoalInfo__Sequence {
                data: accepted_goals.as_mut_ptr() as *mut _ as *mut action_msgs__msg__GoalInfo,
                size: accepted_goals.len(),
                capacity: accepted_goals.capacity(),
            };
        } else {
            let mut empty = vec![];
            response.msg.goals_canceling = action_msgs__msg__GoalInfo__Sequence {
                data: empty.as_mut_ptr() as *mut _,
                size: 0,
                capacity: 0,
            };
        }

        let server = self.server.data.server;

        let guard = rcl::MT_UNSAFE_FN.lock();
        guard.rcl_action_send_cancel_response(
            &server,
            &mut self.request_id,
            &mut response.msg as *const _ as *mut _,
        )?;
        Ok(())
    }

    /// Cancel the goals. Returns the status code for the CancelGoal response.
    fn cancel_goals(&mut self, goals: &[GoalInfo]) -> Result<i8, DynError> {
        if goals.is_empty() {
            return Ok(ERROR_REJECTED);
        }
        let handles = self.server.handles.lock();
        // Make sure that all the goals are found in the handles beforehand
        for goal in goals {
            if !handles.contains_key(&goal.goal_id.uuid) {
                return Ok(ERROR_UNKNOWN_GOAL_ID);
            }
        }
        // Make sure all the goals are not in terminal state
        for goal in goals {
            let handle = handles.get(&goal.goal_id.uuid).unwrap();
            if handle.is_terminal()? {
                return Ok(ERROR_GOAL_TERMINATED);
            }
        }
        for goal in goals {
            let uuid = goal.goal_id.uuid;
            let handle = handles.get(&uuid).unwrap();
            handle.update(GoalEvent::CancelGoal)?;
        }
        Ok(ERROR_NONE)
    }
}

#[pin_project(PinnedDrop)]
#[must_use]
pub struct AsyncCancelReceiver<'a, T: ActionMsg> {
    server: &'a mut Server<T>,
    is_waiting: bool,
}

impl<'a, T: ActionMsg> Future for AsyncCancelReceiver<'a, T> {
    type Output = Result<(ServerCancelSend<'a, T>, Vec<GoalInfo>), DynError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let this = self.project();
        *this.is_waiting = false;
        match rcl_recv_cancel_request(&this.server.data.server) {
            Ok((header, _req, goals)) => {
                let sender = ServerCancelSend {
                    server: this.server.clone(),
                    request_id: header,
                    _phantom: PhantomData,
                    _unsync: Default::default(),
                };
                Poll::Ready(Ok((sender, goals)))
            }
            Err(RCLActionError::ServerTakeFailed) => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();

                match guard.send_command(
                    &this.server.data.node.context,
                    Command::ActionServer {
                        data: this.server.data.clone(),
                        goal: Box::new(move || CallbackResult::Ok),
                        cancel: Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Remove
                        }),
                        result: Box::new(move || CallbackResult::Ok),
                    },
                ) {
                    Ok(_) => {
                        *this.is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
            Err(e) => Poll::Ready(Err(e.into())),
        }
    }
}

#[pinned_drop]
impl<'a, T: ActionMsg> PinnedDrop for AsyncCancelReceiver<'a, T> {
    fn drop(self: Pin<&mut Self>) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.server.data.node.context,
                Command::RemoveActionServer(self.server.data.clone()),
            );
        }
    }
}

pub struct ServerResultSend<'a, T: ActionMsg> {
    server: Server<T>,
    request_id: rmw_request_id_t,
    _phantom: PhantomData<&'a T>,
    _unsync: PhantomUnsync,
}

impl<'a, T: ActionMsg> ServerResultSend<'a, T> {
    pub fn send(mut self, uuid: &[u8; 16]) -> Result<(), DynError> {
        let res = {
            let results = self.server.results.lock();
            results.get(uuid).and_then(|v| v.try_clone())
        };
        match res {
            Some(result) => {
                let mut response = T::new_result_response(GoalStatus::Succeeded as u8, result);
                let guard = rcl::MT_UNSAFE_FN.lock();
                if let Err(e) = guard.rcl_action_send_result_response(
                    &self.server.data.server,
                    &mut self.request_id,
                    &mut response as *const _ as *mut _,
                ) {
                    let logger = Logger::new("oxidros");
                    pr_error_in!(
                        logger,
                        "failed to send result response from action server: {}",
                        e
                    );
                    return Err(e.into());
                }
            }
            None => {
                let mut pending_requests = self.server.data.pending_result_requests.lock();
                let requests = pending_requests.entry(*uuid).or_default();
                requests.push(self.request_id);
            }
        }
        Ok(())
    }
}

#[pin_project(PinnedDrop)]
#[must_use]
pub struct AsyncResultReceiver<'a, T: ActionMsg> {
    server: &'a mut Server<T>,
    is_waiting: bool,
}

impl<'a, T: ActionMsg> Future for AsyncResultReceiver<'a, T> {
    type Output = Result<(ServerResultSend<'a, T>, GetResultServiceRequest<T>), DynError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let this = self.project();
        *this.is_waiting = false;

        match rcl_action_take_result_request::<T>(&this.server.data.server) {
            Ok((header, request)) => {
                let sender = ServerResultSend {
                    server: this.server.clone(),
                    request_id: header,
                    _phantom: PhantomData,
                    _unsync: Default::default(),
                };
                Poll::Ready(Ok((sender, request)))
            }
            Err(RCLActionError::ServerTakeFailed) => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();

                let cmd = Command::ActionServer {
                    data: this.server.data.clone(),
                    goal: Box::new(move || CallbackResult::Ok),
                    cancel: Box::new(move || CallbackResult::Ok),
                    result: Box::new(move || {
                        let w = waker.take().unwrap();
                        w.wake();
                        CallbackResult::Remove
                    }),
                };
                match guard.send_command(&this.server.data.node.context, cmd) {
                    Ok(_) => {
                        *this.is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
            Err(e) => Poll::Ready(Err(e.into())),
        }
    }
}

#[pinned_drop]
impl<'a, T: ActionMsg> PinnedDrop for AsyncResultReceiver<'a, T> {
    fn drop(self: Pin<&mut Self>) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.server.data.node.context,
                Command::RemoveActionServer(self.server.data.clone()),
            );
        }
    }
}

impl<T: ActionMsg> Clone for Server<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            results: self.results.clone(),
            handles: self.handles.clone(),
        }
    }
}

/// An action server which works on async/await context.
///
/// `AsyncServer<T>` does the same job as [`Server<T>`] but on async/await context.
///
/// Consult `examples/action_server.rs` for example usage.
pub struct AsyncServer<T: ActionMsg> {
    server: Server<T>,
}

impl<T: ActionMsg + 'static> AsyncServer<T> {
    pub fn new(server: Server<T>) -> Self {
        Self { server }
    }

    /// Listen for incoming requests.
    pub async fn listen<G, C>(&mut self, goal_handler: G, cancel_handler: C) -> Result<(), DynError>
    where
        G: Fn(ServerGoalSend<T>, SendGoalServiceRequest<T>),
        C: Fn(ServerCancelSend<T>, Vec<GoalInfo>),
    {
        let server_for_goal = self.server.clone();
        let server_for_cancel = self.server.clone();
        let server_for_result = self.server.clone();

        let goal_future = async move {
            let mut server_ = server_for_goal;
            loop {
                let result = server_.recv_goal_request().await;
                match result {
                    Ok((sender, req)) => {
                        goal_handler(sender, req);
                    }
                    Err(e) => break Err::<(), DynError>(e),
                }
            }
        };

        let cancel_future = async move {
            let mut server_ = server_for_cancel;
            loop {
                let result = server_.recv_cancel_request().await;
                match result {
                    Ok((sender, candidates)) => {
                        cancel_handler(sender, candidates);
                    }
                    Err(e) => break Err::<(), DynError>(e),
                }
            }
        };

        let result_future = async move {
            let mut server_ = server_for_result;
            loop {
                let result = server_.recv_result_request().await;
                match result {
                    Ok((sender, req)) => {
                        if let Err(e) = sender.send(req.get_uuid()) {
                            break Err::<(), DynError>(e);
                        }
                    }
                    Err(e) => break Err::<(), DynError>(e),
                }
            }
        };

        try_join!(goal_future, cancel_future, result_future).map(|_: (_, _, _)| ())
    }
}

// Newtype wrappers to avoid orphan rule violations
pub(crate) struct RclGoalInfo(action_msgs__msg__GoalInfo);
pub(crate) struct RclUUID(unique_identifier_msgs__msg__UUID);
pub(crate) struct RclTime(crate::rcl::builtin_interfaces__msg__Time);

impl From<RclGoalInfo> for GoalInfo {
    fn from(value: RclGoalInfo) -> Self {
        Self {
            goal_id: RclUUID(value.0.goal_id).into(),
            stamp: oxidros_msg::interfaces::builtin_interfaces::msg::Time {
                sec: value.0.stamp.sec,
                nanosec: value.0.stamp.nanosec,
            },
        }
    }
}

impl From<action_msgs__msg__GoalInfo> for RclGoalInfo {
    fn from(value: action_msgs__msg__GoalInfo) -> Self {
        RclGoalInfo(value)
    }
}

impl From<RclUUID> for UUID {
    fn from(value: RclUUID) -> Self {
        Self { uuid: value.0.uuid }
    }
}

impl From<unique_identifier_msgs__msg__UUID> for RclUUID {
    fn from(value: unique_identifier_msgs__msg__UUID) -> Self {
        RclUUID(value)
    }
}

impl From<RclTime> for crate::msg::builtin_interfaces__msg__Time {
    fn from(value: RclTime) -> Self {
        Self {
            sec: value.0.sec,
            nanosec: value.0.nanosec,
        }
    }
}

impl From<crate::rcl::builtin_interfaces__msg__Time> for RclTime {
    fn from(value: crate::rcl::builtin_interfaces__msg__Time) -> Self {
        RclTime(value)
    }
}

#[allow(clippy::result_large_err)]
fn rcl_action_accept_new_goal(
    server: *mut rcl_action_server_t,
    goal_info: &action_msgs__msg__GoalInfo,
) -> Result<*mut rcl_action_goal_handle_t, Box<rcl::rcutils_error_string_t>> {
    let goal_handle = {
        let guard = rcl::MT_UNSAFE_FN.lock();
        guard.rcl_action_accept_new_goal(server, goal_info)
    };
    if goal_handle.is_null() {
        let msg = unsafe { rcl::rcutils_get_error_string() };
        return Err(Box::new(msg));
    }

    Ok(goal_handle)
}

fn rcl_action_take_goal_request<T: ActionMsg>(
    server: &rcl_action_server_t,
) -> RCLActionResult<(rcl::rmw_request_id_t, SendGoalServiceRequest<T>)> {
    let mut header: rcl::rmw_request_id_t = unsafe { std::mem::zeroed() };
    let mut request: SendGoalServiceRequest<T> = unsafe { std::mem::zeroed() };
    let guard = rcl::MT_UNSAFE_FN.lock();
    guard.rcl_action_take_goal_request(server, &mut header, &mut request as *const _ as *mut _)?;
    Ok((header, request))
}

fn rcl_recv_cancel_request(
    server: &rcl_action_server_t,
) -> RCLActionResult<(
    rcl::rmw_request_id_t,
    rcl_action_cancel_request_t,
    Vec<GoalInfo>,
)> {
    let mut header: rcl::rmw_request_id_t = unsafe { std::mem::zeroed() };
    let mut request: rcl_action_cancel_request_t =
        rcl::MTSafeFn::rcl_action_get_zero_initialized_cancel_request();

    let guard = rcl::MT_UNSAFE_FN.lock();

    guard.rcl_action_take_cancel_request(
        server,
        &mut header,
        &mut request as *const _ as *mut _,
    )?;
    // process cancel request in advance
    let mut process_response = rcl::MTSafeFn::rcl_action_get_zero_initialized_cancel_response();

    // compute which exact goals are requested to be cancelled
    if let Err(e) = guard.rcl_action_process_cancel_request(
        server,
        &request,
        &mut process_response as *const _ as *mut _,
    ) {
        let logger = Logger::new("oxidros");
        pr_error_in!(
            logger,
            "failed to send cancel responses from action server: {}",
            e
        );
        return Err(e);
    }
    // Convert RCL goal info sequence to oxidros-msg GoalInfo vector
    let rcl_goals = unsafe {
        std::slice::from_raw_parts(
            process_response.msg.goals_canceling.data,
            process_response.msg.goals_canceling.size,
        )
    };
    let goals = rcl_goals
        .iter()
        .map(|g| RclGoalInfo(*g).into())
        .collect::<Vec<_>>();

    Ok((header, request, goals))
}

fn rcl_action_take_result_request<T: ActionMsg>(
    server: &rcl_action_server_t,
) -> RCLActionResult<(rcl::rmw_request_id_t, GetResultServiceRequest<T>)> {
    let mut header: rcl::rmw_request_id_t = unsafe { std::mem::zeroed() };
    let mut request: GetResultServiceRequest<T> = unsafe { std::mem::zeroed() };
    let guard = rcl::MT_UNSAFE_FN.lock();
    guard.rcl_action_take_result_request(
        server,
        &mut header,
        &mut request as *const _ as *mut _,
    )?;
    Ok((header, request))
}

fn get_timestamp(clock: &mut Clock) -> UnsafeTime {
    let now_nanosec = clock.get_now().unwrap();
    let now_sec = now_nanosec / 10_i64.pow(9);
    UnsafeTime {
        sec: now_sec as i32,
        nanosec: (now_nanosec - now_sec * 10_i64.pow(9)) as u32,
    }
}
