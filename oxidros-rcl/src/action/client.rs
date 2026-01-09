//! Action client.

use oxidros_core::selector::CallbackResult;
use oxidros_core::{ActionError, Error, RclError};
use std::future::Future;
use std::pin::Pin;
use std::{ffi::CString, marker::PhantomData, sync::Arc, task::Poll, time::Duration};

use crate::helper::is_unpin;
use crate::{
    RecvResult,
    error::Result,
    get_allocator, is_halt,
    msg::{
        ActionMsg,
        interfaces::action_msgs::{
            msg::GoalStatusArray,
            srv::{CancelGoal_Request, CancelGoal_Response},
        },
    },
    node::Node,
    qos::Profile,
    rcl,
    selector::{
        Selector,
        async_selector::{self, SELECTOR},
    },
    signal_handler::Signaled,
};

use super::{
    GetResultServiceRequest, GetResultServiceResponse, SendGoalServiceRequest,
    SendGoalServiceResponse,
};

pub struct ClientQosOption {
    goal_service: Profile,
    result_service: Profile,
    cancel_service: Profile,
    feedback_topic: Profile,
    status_topic: Profile,
}

impl From<ClientQosOption> for rcl::rcl_action_client_options_t {
    fn from(opts: ClientQosOption) -> Self {
        rcl::rcl_action_client_options_t {
            goal_service_qos: (&opts.goal_service).into(),
            result_service_qos: (&opts.result_service).into(),
            cancel_service_qos: (&opts.cancel_service).into(),
            feedback_topic_qos: (&opts.feedback_topic).into(),
            status_topic_qos: (&opts.status_topic).into(),
            allocator: get_allocator(),
        }
    }
}

pub(crate) struct ClientData {
    pub(crate) client: rcl::rcl_action_client_t,
    pub(crate) node: Arc<Node>,
}

impl Drop for ClientData {
    fn drop(&mut self) {
        let guard = rcl::MT_UNSAFE_FN.lock();
        let _ = guard.rcl_action_client_fini(&mut self.client, unsafe { self.node.as_ptr_mut() });
    }
}

unsafe impl Sync for ClientData {}
unsafe impl Send for ClientData {}

/// An action client.
///
/// Consult `examples/action_client.rs` for a working example.
pub struct Client<T: ActionMsg> {
    data: Arc<ClientData>,
    // TODO: do like server::Client add Dbs
    _phantom: PhantomData<T>,
}

impl<T> Client<T>
where
    T: ActionMsg,
{
    // Create a client.
    pub fn new(node: Arc<Node>, action_name: &str, qos: Option<ClientQosOption>) -> Result<Self> {
        let mut client = rcl::MTSafeFn::rcl_action_get_zero_initialized_client();
        let options = qos
            .map(rcl::rcl_action_client_options_t::from)
            .unwrap_or_else(rcl::MTSafeFn::rcl_action_client_get_default_options);
        let action_name = CString::new(action_name).unwrap_or_default();

        {
            let guard = rcl::MT_UNSAFE_FN.lock();
            guard.rcl_action_client_init(
                &mut client,
                unsafe { node.as_ptr_mut() },
                T::type_support() as *const rcl::rosidl_action_type_support_t,
                action_name.as_ptr(),
                &options,
            )?;
        }

        Ok(Self {
            data: Arc::new(ClientData { client, node }),
            _phantom: Default::default(),
        })
    }

    /// Get the inner data for use with the selector.
    pub(crate) fn inner_data(&self) -> &Arc<ClientData> {
        &self.data
    }

    /// Returns true if the corresponding action server is available.
    pub fn is_server_available(&self) -> Result<bool> {
        let guard = rcl::MT_UNSAFE_FN.lock();
        let mut is_available = false;
        match guard.rcl_action_server_is_available(
            self.data.node.as_ptr(),
            &self.data.client,
            &mut is_available as *mut _,
        ) {
            Ok(()) => Ok(is_available),
            Err(Error::Action(ActionError::Rcl(RclError::NodeInvalid))) => {
                // TODO: soft failure in case of shutdown context
                eprintln!("Invalid node (the shutdown has started?)");
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Send a goal request to the server with given uuid. the uuid can be any 16-bit slice [u8; 16] i.e. does not have to
    /// conform to the UUID v4 standard. Use the returned [`ClientGoalRecv<T>`] to receive the response.
    pub fn send_goal_with_uuid(
        &mut self,
        goal: <T as ActionMsg>::GoalContent,
        uuid: [u8; 16],
    ) -> Result<ClientGoalRecv<'_, T>> {
        let request = <T as ActionMsg>::new_goal_request(goal, uuid);
        self.send_goal_request(&request)
    }

    /// Send a goal request. Use the returned [`ClientGoalRecv<T>`] to receive the response.
    fn send_goal_request(
        &mut self,
        data: &SendGoalServiceRequest<T>,
    ) -> Result<ClientGoalRecv<'_, T>> {
        if crate::is_halt() {
            return Err(Signaled.into());
        }

        let mut seq: i64 = 0;
        rcl::MTSafeFn::rcl_action_send_goal_request(
            &self.data.client,
            data as *const _ as _,
            &mut seq,
        )?;

        Ok(ClientGoalRecv {
            inner: ClientRecv::new(self),
            seq,
        })
    }

    /// Send a result request to the server. Use the returned [`ClientResultRecv<T>`] to receive the response.
    pub fn send_result_request(
        &mut self,
        data: &GetResultServiceRequest<T>,
    ) -> Result<ClientResultRecv<'_, T>> {
        let mut seq: i64 = 0;
        rcl::MTSafeFn::rcl_action_send_result_request(
            &self.data.client,
            data as *const GetResultServiceRequest<T> as _,
            &mut seq,
        )?;

        Ok(ClientResultRecv {
            inner: ClientRecv::new(self),
            seq,
        })
    }

    /// Send a cancel request. Use the returned [`ClientCancelRecv<T>`] to receive the response.
    pub fn send_cancel_request(
        &mut self,
        request: &CancelGoal_Request,
    ) -> Result<ClientCancelRecv<'_, T>> {
        let guard = rcl::MT_UNSAFE_FN.lock();

        let mut seq: i64 = 0;
        guard.rcl_action_send_cancel_request(
            &self.data.client,
            request as *const _ as _,
            &mut seq,
        )?;

        Ok(ClientCancelRecv {
            inner: ClientRecv::new(self),
            seq,
        })
    }

    /// Takes a feedback for the goal. If there is no feedback, it returns [`RecvResult::RetryLater`].
    pub fn try_recv_feedback(&mut self) -> RecvResult<<T as ActionMsg>::Feedback> {
        match rcl_action_take_feedback::<T>(&self.data.client) {
            Ok(feedback) => RecvResult::Ok(feedback),
            Err(Error::Action(ActionError::ClientTakeFailed)) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Wait until the client receives a feedback message or the duration `t` elapses.
    pub fn recv_feedback_timeout(
        &mut self,
        t: Duration,
        selector: &mut Selector,
    ) -> RecvResult<<T as ActionMsg>::Feedback> {
        selector.add_action_client(self.data.clone(), None, None, None, None, None);
        match selector.wait_timeout(t) {
            Ok(true) => self.try_recv_feedback(),
            Ok(false) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Asynchronously receive a feedback message.
    pub async fn recv_feedback(&mut self) -> Result<<T as ActionMsg>::Feedback> {
        AsyncFeedbackReceiver {
            client: self,
            is_waiting: false,
            _phantom: Default::default(),
        }
        .await
    }

    /// Takes a status message for all the ongoing goals.
    pub fn try_recv_status(&mut self) -> RecvResult<GoalStatusArray> {
        match rcl_action_take_status(&self.data.client) {
            Ok(status_array) => RecvResult::Ok(status_array),
            Err(Error::Action(ActionError::ClientTakeFailed)) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Wait until the client receives a status message or the duration `t` elapses.
    pub fn recv_status_timeout(
        &mut self,
        t: Duration,
        selector: &mut Selector,
    ) -> RecvResult<GoalStatusArray> {
        selector.add_action_client(self.data.clone(), None, None, None, None, None);
        match selector.wait_timeout(t) {
            Ok(true) => self.try_recv_status(),
            Ok(false) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Asynchronously receive a status message.
    pub async fn recv_status(&mut self) -> Result<GoalStatusArray> {
        AsyncStatusReceiver {
            client: self,
            is_waiting: false,
            _phantom: Default::default(),
        }
        .await
    }
}

pub(crate) struct ClientRecv<'a, T: ActionMsg> {
    client: &'a mut Client<T>,
    _phantom: PhantomData<T>,
}

impl<'a, T: ActionMsg> ClientRecv<'a, T> {
    fn new(client: &'a mut Client<T>) -> Self {
        Self {
            client,
            _phantom: Default::default(),
        }
    }
}

/// A receiver for the response of the goal request, usually returned by [`Client::send_goal_with_uuid`].
///
/// Use one of [`ClientGoalRecv::try_recv`], [`ClientGoalRecv::recv_timeout`], or [`ClientGoalRecv::recv`] to receive the response. The action client [`Client<T>`] is returned together with the response so that another request can be made.
pub struct ClientGoalRecv<'a, T: ActionMsg> {
    pub(crate) inner: ClientRecv<'a, T>,
    seq: i64,
}

impl<'a, T: ActionMsg> ClientGoalRecv<'a, T> {
    /// Returns a response if available. If there is no response, it returns [`RecvResult::RetryLater`].
    pub fn try_recv(&self) -> RecvResult<(SendGoalServiceResponse<T>, rcl::rmw_request_id_t)> {
        match rcl_action_take_goal_response::<T>(&self.inner.client.data.client) {
            Ok((response, header)) => {
                if header.sequence_number == self.seq {
                    RecvResult::Ok((response, header))
                } else {
                    RecvResult::RetryLater
                }
            }
            Err(Error::Action(ActionError::ClientTakeFailed)) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Wait until the client receives a response or the duration `t` elapses.
    pub fn recv_timeout(
        &self,
        t: Duration,
        selector: &mut Selector,
    ) -> RecvResult<(SendGoalServiceResponse<T>, rcl::rmw_request_id_t)> {
        selector.add_action_client(self.inner.client.data.clone(), None, None, None, None, None);

        match selector.wait_timeout(t) {
            Ok(true) => self.try_recv(),
            Ok(false) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Asynchronously receive the response.
    pub async fn recv(self) -> Result<(SendGoalServiceResponse<T>, rcl::rmw_request_id_t)> {
        AsyncGoalReceiver {
            client: self,
            is_waiting: false,
        }
        .await
    }
}

pub struct AsyncGoalReceiver<'a, T: ActionMsg> {
    client: ClientGoalRecv<'a, T>,
    is_waiting: bool,
}

impl<'a, T: ActionMsg> AsyncGoalReceiver<'a, T> {
    fn project(self: std::pin::Pin<&mut Self>) -> (&ClientGoalRecv<'a, T>, &mut bool) {
        unsafe {
            let this = self.get_unchecked_mut();
            (&this.client, &mut this.is_waiting)
        }
    }
}

impl<'a, T: ActionMsg> Drop for AsyncGoalReceiver<'a, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.client.inner.client.data.node.context,
                async_selector::Command::RemoveActionClient(self.client.inner.client.data.clone()),
            );
        }
    }
}

impl<'a, T: ActionMsg> Future for AsyncGoalReceiver<'a, T> {
    type Output = Result<(SendGoalServiceResponse<T>, rcl::rmw_request_id_t)>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let (client, is_waiting) = self.project();
        *is_waiting = false;
        match client.try_recv() {
            RecvResult::Ok(v) => Poll::Ready(Ok(v)),
            RecvResult::Err(e) => Poll::Ready(Err(e)),
            RecvResult::RetryLater => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();
                match guard.send_command(
                    &client.inner.client.data.node.context,
                    async_selector::Command::ActionClient {
                        data: client.inner.client.data.clone(),
                        feedback: Box::new(|| CallbackResult::Ok),
                        status: Box::new(|| CallbackResult::Ok),
                        goal: Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Remove
                        }),
                        cancel: Box::new(|| CallbackResult::Ok),
                        result: Box::new(|| CallbackResult::Ok),
                    },
                ) {
                    Ok(_) => {
                        *is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}

pub struct ClientCancelRecv<'a, T: ActionMsg> {
    pub(crate) inner: ClientRecv<'a, T>,
    seq: i64,
}

impl<'a, T: ActionMsg> ClientCancelRecv<'a, T> {
    pub fn try_recv(&self) -> RecvResult<(CancelGoal_Response, rcl::rmw_request_id_t)> {
        match rcl_action_take_cancel_response(&self.inner.client.data.client) {
            Ok((response, header)) => {
                if header.sequence_number == self.seq {
                    RecvResult::Ok((response, header))
                } else {
                    RecvResult::RetryLater
                }
            }
            Err(Error::Action(ActionError::ClientTakeFailed)) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Wait until the client receives a response or the duration `t` elapses.
    pub fn recv_timeout(
        &self,
        t: Duration,
        selector: &mut Selector,
    ) -> RecvResult<(CancelGoal_Response, rcl::rmw_request_id_t)> {
        selector.add_action_client(self.inner.client.data.clone(), None, None, None, None, None);
        match selector.wait_timeout(t) {
            Ok(true) => self.try_recv(),
            Ok(false) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    pub async fn recv(self) -> Result<(CancelGoal_Response, rcl::rmw_request_id_t)> {
        AsyncCancelReceiver {
            client: self,
            is_waiting: false,
        }
        .await
    }
}

pub struct AsyncCancelReceiver<'a, T: ActionMsg> {
    client: ClientCancelRecv<'a, T>,
    is_waiting: bool,
}

impl<'a, T: ActionMsg> AsyncCancelReceiver<'a, T> {
    fn project(self: std::pin::Pin<&mut Self>) -> (&ClientCancelRecv<'a, T>, &mut bool) {
        unsafe {
            let this = self.get_unchecked_mut();
            (&this.client, &mut this.is_waiting)
        }
    }
}

impl<'a, T: ActionMsg> Drop for AsyncCancelReceiver<'a, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.client.inner.client.data.node.context,
                async_selector::Command::RemoveActionClient(self.client.inner.client.data.clone()),
            );
        }
    }
}

impl<'a, T: ActionMsg> Future for AsyncCancelReceiver<'a, T> {
    type Output = Result<(CancelGoal_Response, rcl::rmw_request_id_t)>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let (client, is_waiting) = self.project();
        *is_waiting = false;
        match client.try_recv() {
            RecvResult::Ok(v) => Poll::Ready(Ok(v)),
            RecvResult::Err(e) => Poll::Ready(Err(e)),
            RecvResult::RetryLater => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();

                match guard.send_command(
                    &client.inner.client.data.node.context,
                    async_selector::Command::ActionClient {
                        data: client.inner.client.data.clone(),
                        feedback: Box::new(|| CallbackResult::Ok),
                        status: Box::new(|| CallbackResult::Ok),
                        goal: Box::new(|| CallbackResult::Ok),
                        cancel: Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Remove
                        }),
                        result: Box::new(|| CallbackResult::Ok),
                    },
                ) {
                    Ok(_) => {
                        *is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}

pub struct ClientResultRecv<'a, T: ActionMsg> {
    pub(crate) inner: ClientRecv<'a, T>,
    seq: i64,
}

impl<'a, T: ActionMsg> ClientResultRecv<'a, T> {
    pub fn try_recv(&self) -> RecvResult<(GetResultServiceResponse<T>, rcl::rmw_request_id_t)> {
        match rcl_action_take_result_response::<T>(&self.inner.client.data.client) {
            Ok((response, header)) => {
                if header.sequence_number == self.seq {
                    RecvResult::Ok((response, header))
                } else {
                    RecvResult::RetryLater
                }
            }
            Err(Error::Action(ActionError::ClientTakeFailed)) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    /// Wait until the client receives a response or the duration `t` elapses.
    pub fn recv_timeout(
        &self,
        t: Duration,
        selector: &mut Selector,
    ) -> RecvResult<(GetResultServiceResponse<T>, rcl::rmw_request_id_t)> {
        selector.add_action_client(self.inner.client.data.clone(), None, None, None, None, None);

        match selector.wait_timeout(t) {
            Ok(true) => self.try_recv(),
            Ok(false) => RecvResult::RetryLater,
            Err(e) => RecvResult::Err(e),
        }
    }

    pub async fn recv(self) -> Result<(GetResultServiceResponse<T>, rcl::rmw_request_id_t)> {
        AsyncResultReceiver {
            client: self,
            is_waiting: false,
        }
        .await
    }
}

pub struct AsyncResultReceiver<'a, T: ActionMsg> {
    client: ClientResultRecv<'a, T>,
    is_waiting: bool,
}

impl<'a, T: ActionMsg> AsyncResultReceiver<'a, T> {
    fn project(self: std::pin::Pin<&mut Self>) -> (&ClientResultRecv<'a, T>, &mut bool) {
        unsafe {
            let this = self.get_unchecked_mut();
            (&this.client, &mut this.is_waiting)
        }
    }
}

impl<'a, T: ActionMsg> Drop for AsyncResultReceiver<'a, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.client.inner.client.data.node.context,
                async_selector::Command::RemoveActionClient(self.client.inner.client.data.clone()),
            );
        }
    }
}

impl<'a, T: ActionMsg> Future for AsyncResultReceiver<'a, T> {
    type Output = Result<(GetResultServiceResponse<T>, rcl::rmw_request_id_t)>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let (client, is_waiting) = self.project();
        *is_waiting = false;
        match client.try_recv() {
            RecvResult::Ok(v) => Poll::Ready(Ok(v)),
            RecvResult::Err(e) => Poll::Ready(Err(e)),
            RecvResult::RetryLater => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();
                match guard.send_command(
                    &client.inner.client.data.node.context,
                    async_selector::Command::ActionClient {
                        data: client.inner.client.data.clone(),
                        feedback: Box::new(|| CallbackResult::Ok),
                        status: Box::new(|| CallbackResult::Ok),
                        goal: Box::new(|| CallbackResult::Ok),
                        cancel: Box::new(|| CallbackResult::Ok),
                        result: Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Remove
                        }),
                    },
                ) {
                    Ok(_) => {
                        *is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}

pub struct AsyncFeedbackReceiver<'a, T: ActionMsg> {
    client: &'a mut Client<T>,
    is_waiting: bool,
    _phantom: PhantomData<T>,
}

impl<'a, T: ActionMsg> AsyncFeedbackReceiver<'a, T> {
    fn project(self: std::pin::Pin<&mut Self>) -> (&mut Client<T>, &mut bool) {
        is_unpin::<&mut Client<T>>();
        unsafe {
            let this = self.get_unchecked_mut();
            (&mut this.client, &mut this.is_waiting)
        }
    }
}

impl<'a, T: ActionMsg> Drop for AsyncFeedbackReceiver<'a, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.client.data.node.context,
                async_selector::Command::RemoveActionClient(self.client.data.clone()),
            );
        }
    }
}

impl<'a, T: ActionMsg> Future for AsyncFeedbackReceiver<'a, T> {
    type Output = Result<<T as ActionMsg>::Feedback>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let (client, is_waiting) = self.project();
        *is_waiting = false;
        match client.try_recv_feedback() {
            RecvResult::Ok(v) => Poll::Ready(Ok(v)),
            RecvResult::Err(e) => Poll::Ready(Err(e)),
            RecvResult::RetryLater => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();

                match guard.send_command(
                    &client.data.node.context,
                    async_selector::Command::ActionClient {
                        data: client.data.clone(),
                        feedback: Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Remove
                        }),
                        status: Box::new(|| CallbackResult::Ok),
                        goal: Box::new(|| CallbackResult::Ok),
                        cancel: Box::new(|| CallbackResult::Ok),
                        result: Box::new(|| CallbackResult::Ok),
                    },
                ) {
                    Ok(_) => {
                        *is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}

pub struct AsyncStatusReceiver<'a, T: ActionMsg> {
    client: &'a mut Client<T>,
    is_waiting: bool,
    _phantom: PhantomData<T>,
}

impl<'a, T: ActionMsg> AsyncStatusReceiver<'a, T> {
    fn project(self: std::pin::Pin<&mut Self>) -> (&mut Client<T>, &mut bool) {
        is_unpin::<&mut Client<T>>();
        unsafe {
            let this = self.get_unchecked_mut();
            (&mut this.client, &mut this.is_waiting)
        }
    }
}

impl<'a, T: ActionMsg> Drop for AsyncStatusReceiver<'a, T> {
    fn drop(&mut self) {
        if self.is_waiting {
            let mut guard = SELECTOR.lock();
            let _ = guard.send_command(
                &self.client.data.node.context,
                async_selector::Command::RemoveActionClient(self.client.data.clone()),
            );
        }
    }
}

impl<'a, T: ActionMsg> Future for AsyncStatusReceiver<'a, T> {
    type Output = Result<GoalStatusArray>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        let (client, is_waiting) = self.project();
        *is_waiting = false;
        match client.try_recv_status() {
            RecvResult::Ok(v) => Poll::Ready(Ok(v)),
            RecvResult::Err(e) => Poll::Ready(Err(e)),
            RecvResult::RetryLater => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();

                match guard.send_command(
                    &client.data.node.context,
                    async_selector::Command::ActionClient {
                        data: client.data.clone(),
                        feedback: Box::new(|| CallbackResult::Ok),
                        status: Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Remove
                        }),
                        goal: Box::new(|| CallbackResult::Ok),
                        cancel: Box::new(|| CallbackResult::Ok),
                        result: Box::new(|| CallbackResult::Ok),
                    },
                ) {
                    Ok(_) => {
                        *is_waiting = true;
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}

fn rcl_action_take_goal_response<T>(
    client: &rcl::rcl_action_client_t,
) -> Result<(SendGoalServiceResponse<T>, rcl::rmw_request_id_t)>
where
    T: ActionMsg,
{
    let guard = rcl::MT_UNSAFE_FN.lock();

    let mut header: rcl::rmw_request_id_t = unsafe { std::mem::zeroed() };
    let mut response: SendGoalServiceResponse<T> = unsafe { std::mem::zeroed() };
    guard.rcl_action_take_goal_response(
        client,
        &mut header,
        &mut response as *const _ as *mut _,
    )?;

    Ok((response, header))
}

fn rcl_action_take_cancel_response(
    client: &rcl::rcl_action_client_t,
) -> Result<(CancelGoal_Response, rcl::rmw_request_id_t)> {
    let guard = rcl::MT_UNSAFE_FN.lock();

    let mut header: rcl::rmw_request_id_t = unsafe { std::mem::zeroed() };
    let mut response: CancelGoal_Response = unsafe { std::mem::zeroed() };
    guard.rcl_action_take_cancel_response(
        client,
        &mut header,
        &mut response as *const _ as *mut _,
    )?;

    Ok((response, header))
}

fn rcl_action_take_result_response<T>(
    client: &rcl::rcl_action_client_t,
) -> Result<(GetResultServiceResponse<T>, rcl::rmw_request_id_t)>
where
    T: ActionMsg,
{
    let guard = rcl::MT_UNSAFE_FN.lock();

    let mut header: rcl::rmw_request_id_t = unsafe { std::mem::zeroed() };
    let mut response: GetResultServiceResponse<T> = unsafe { std::mem::zeroed() };
    guard.rcl_action_take_result_response(
        client,
        &mut header,
        &mut response as *const _ as *mut _,
    )?;

    Ok((response, header))
}

fn rcl_action_take_feedback<T>(client: &rcl::rcl_action_client_t) -> Result<T::Feedback>
where
    T: ActionMsg,
{
    let guard = rcl::MT_UNSAFE_FN.lock();

    let mut feedback: <T as ActionMsg>::Feedback = unsafe { std::mem::zeroed() };
    guard.rcl_action_take_feedback(client, &mut feedback as *const _ as *mut _)?;

    Ok(feedback)
}

fn rcl_action_take_status(client: &rcl::rcl_action_client_t) -> Result<GoalStatusArray> {
    let guard = rcl::MT_UNSAFE_FN.lock();
    let mut status_array = GoalStatusArray::new().unwrap();
    guard.rcl_action_take_status(client, &mut status_array as *const _ as *mut _)?;
    Ok(status_array)
}
