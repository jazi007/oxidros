//! Errors returned by ROS2.

pub use crate::rcl::RclRetErr;
use crate::rcl::{self, rcutils_error_string_t};
pub use oxidros_core::error::{ActionError, Error, RclError, Result};

/// Convert a rcl-style, C-style, return value to a Rust-style value.
/// If `n` indicates successful, this returns Ok(()),
/// otherwise returns Err(_).
pub(crate) fn ret_val_to_err(n: rcl::rcl_ret_t) -> Result<()> {
    if (n as u32) == rcl::RCL_RET_OK {
        Ok(())
    } else {
        Err(Error::Rcl(RclRetErr(n).into()))
    }
}

pub(crate) fn action_ret_val_to_err(n: rcl::rcl_ret_t) -> Result<()> {
    match n as u32 {
        rcl::RCL_RET_OK => Ok(()),
        rcl::RCL_RET_ACTION_NAME_INVALID => Err(ActionError::NameInvalid.into()),
        rcl::RCL_RET_ACTION_GOAL_ACCEPTED => Err(ActionError::GoalAccepted.into()),
        rcl::RCL_RET_ACTION_GOAL_REJECTED => Err(ActionError::GoalRejected.into()),
        rcl::RCL_RET_ACTION_CLIENT_INVALID => Err(ActionError::ClientInvalid.into()),
        rcl::RCL_RET_ACTION_CLIENT_TAKE_FAILED => Err(ActionError::ClientTakeFailed.into()),
        rcl::RCL_RET_ACTION_SERVER_INVALID => Err(ActionError::ServerInvalid.into()),
        rcl::RCL_RET_ACTION_SERVER_TAKE_FAILED => Err(ActionError::ServerTakeFailed.into()),
        rcl::RCL_RET_ACTION_GOAL_HANDLE_INVALID => Err(ActionError::GoalHandleInvalid.into()),
        rcl::RCL_RET_ACTION_GOAL_EVENT_INVALID => Err(ActionError::GoalEventInvalid.into()),
        _ => ret_val_to_err(n),
    }
}

pub(crate) fn rcutils_error_string_to_err(v: rcutils_error_string_t) -> Error {
    let err = unsafe { std::ffi::CStr::from_ptr(v.str_.as_ptr()) };
    Error::Other(err.to_string_lossy().to_string())
}
