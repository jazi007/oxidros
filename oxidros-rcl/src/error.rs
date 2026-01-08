//! Errors returned by ROS2.

pub use crate::rcl::RclRetErr;
use crate::rcl::{self, rcutils_error_string_t};
pub use oxidros_core::error::{ActionError, Error, RclError, Result};

pub type OResult<T> = std::result::Result<T, RclError>;
pub type OError = RclError;
pub type RCLActionError = ActionError;
pub type RCLActionResult<T> = std::result::Result<T, ActionError>;

/// Convert a rcl-style, C-style, return value to a Rust-style value.
/// If `n` indicates successful, this returns Ok(()),
/// otherwise returns Err(_).
pub(crate) fn ret_val_to_err(n: rcl::rcl_ret_t) -> OResult<()> {
    if (n as u32) == rcl::RCL_RET_OK {
        Ok(())
    } else {
        Err(RclRetErr(n).into())
    }
}

pub(crate) fn action_ret_val_to_err(n: rcl::rcl_ret_t) -> RCLActionResult<()> {
    match n as u32 {
        rcl::RCL_RET_OK => Ok(()),
        rcl::RCL_RET_ACTION_NAME_INVALID => Err(ActionError::NameInvalid),
        rcl::RCL_RET_ACTION_GOAL_ACCEPTED => Err(ActionError::GoalAccepted),
        rcl::RCL_RET_ACTION_GOAL_REJECTED => Err(ActionError::GoalRejected),
        rcl::RCL_RET_ACTION_CLIENT_INVALID => Err(ActionError::ClientInvalid),
        rcl::RCL_RET_ACTION_CLIENT_TAKE_FAILED => Err(ActionError::ClientTakeFailed),
        rcl::RCL_RET_ACTION_SERVER_INVALID => Err(ActionError::ServerInvalid),
        rcl::RCL_RET_ACTION_SERVER_TAKE_FAILED => Err(ActionError::ServerTakeFailed),
        rcl::RCL_RET_ACTION_GOAL_HANDLE_INVALID => Err(ActionError::GoalHandleInvalid),
        rcl::RCL_RET_ACTION_GOAL_EVENT_INVALID => Err(ActionError::GoalEventInvalid),
        _ => ret_val_to_err(n).map_err(ActionError::Rcl),
    }
}

pub(crate) fn rcutils_error_string_to_err(v: rcutils_error_string_t) -> Error {
    let err = unsafe { std::ffi::CStr::from_ptr(v.str_.as_ptr()) };
    Error::Other(err.to_string_lossy().to_string())
}
