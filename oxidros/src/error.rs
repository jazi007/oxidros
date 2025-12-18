//! Errors returned by ROS2.

use crate::rcl;
pub use oxidros_core::error::*;
pub use oxidros_rcl::RclRetErr;

/// Convert a rcl-style, C-style, return value to a Rust-style value.
/// If `n` indicates successful, this returns Ok(()),
/// otherwise returns Err(_).
pub(crate) fn ret_val_to_err(n: rcl::rcl_ret_t) -> RCLResult<()> {
    if (n as u32) == rcl::RCL_RET_OK {
        Ok(())
    } else {
        Err(RclRetErr(n).into())
    }
}

pub(crate) fn action_ret_val_to_err(n: rcl::rcl_ret_t) -> RCLActionResult<()> {
    match n as u32 {
        rcl::RCL_RET_OK => Ok(()),
        rcl::RCL_RET_ACTION_NAME_INVALID => Err(RCLActionError::NameInvalid),
        rcl::RCL_RET_ACTION_GOAL_ACCEPTED => Err(RCLActionError::GoalAccepted),
        rcl::RCL_RET_ACTION_GOAL_REJECTED => Err(RCLActionError::GoalRejected),
        rcl::RCL_RET_ACTION_CLIENT_INVALID => Err(RCLActionError::ClientInvalid),
        rcl::RCL_RET_ACTION_CLIENT_TAKE_FAILED => Err(RCLActionError::ClientTakeFailed),
        rcl::RCL_RET_ACTION_SERVER_INVALID => Err(RCLActionError::ServerInvalid),
        rcl::RCL_RET_ACTION_SERVER_TAKE_FAILED => Err(RCLActionError::ServerTakeFailed),
        rcl::RCL_RET_ACTION_GOAL_HANDLE_INVALID => Err(RCLActionError::GoalHandleInvalid),
        rcl::RCL_RET_ACTION_GOAL_EVENT_INVALID => Err(RCLActionError::GoalEventInvalid),

        _ => ret_val_to_err(n).map_err(RCLActionError::RCLError),
    }
}
