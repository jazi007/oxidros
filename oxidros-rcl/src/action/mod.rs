//! Actions

use crate::{
    msg::{ActionGoal, ActionMsg, ActionResult},
    rcl::action_msgs__srv__CancelGoal_Request,
};

pub mod client;
pub mod handle;
pub mod server;

pub type SendGoalServiceRequest<T> = <<T as ActionMsg>::Goal as ActionGoal>::Request;
type SendGoalServiceResponse<T> = <<T as ActionMsg>::Goal as ActionGoal>::Response;
type GetResultServiceRequest<T> = <<T as ActionMsg>::Result as ActionResult>::Request;
type GetResultServiceResponse<T> = <<T as ActionMsg>::Result as ActionResult>::Response;
pub type CancelRequest = action_msgs__srv__CancelGoal_Request;

pub use oxidros_core::action::{GoalEvent, GoalStatus};
