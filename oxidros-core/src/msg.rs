//! Message traits and types for ROS2 messages, services, and actions.

use crate::time::UnsafeTime;
use std::ffi::c_void;

/// Trait for type that can fail cloning
pub trait TryClone: Sized {
    /// Returns Some(Self) if clone susccess else None
    fn try_clone(&self) -> Option<Self>;
}

/// Trait for types that have type support information.
///
/// This allows the runtime to understand the structure of messages
/// for serialization and deserialization.
pub trait TypeSupport {
    /// Returns an opaque pointer to the type support structure.
    ///
    /// The actual type of this pointer depends on the implementation
    /// (e.g., `rosidl_message_type_support_t` in RCL).
    fn type_support() -> *const c_void;
}

/// Trait for ROS2 service message types.
///
/// Services consist of a request and response message pair.
pub trait ServiceMsg {
    /// The request message type.
    type Request: TypeSupport;

    /// The response message type.
    type Response: TypeSupport;

    /// Returns an opaque pointer to the service type support structure.
    fn type_support() -> *const c_void;
}

/// Trait for ROS2 action message types.
///
/// Actions are more complex than services and include goals, results,
/// and feedback messages.
pub trait ActionMsg {
    /// The goal service type.
    type Goal: ActionGoal;

    /// The result service type.
    type Result: ActionResult;

    /// The feedback message type.
    type Feedback: TypeSupport + GetUUID;

    /// Returns an opaque pointer to the action type support structure.
    fn type_support() -> *const c_void;

    /// The goal content type (the actual goal data).
    type GoalContent: TypeSupport;

    /// Create a new goal request with the given goal and UUID.
    fn new_goal_request(
        goal: Self::GoalContent,
        uuid: [u8; 16],
    ) -> <Self::Goal as ActionGoal>::Request;

    /// The result content type (the actual result data).
    type ResultContent: TypeSupport + TryClone;

    /// Create a new result response with the given status and result.
    fn new_result_response(
        status: u8,
        result: Self::ResultContent,
    ) -> <Self::Result as ActionResult>::Response;

    /// The feedback content type (the actual feedback data).
    type FeedbackContent: TypeSupport;

    /// Create a new feedback message with the given feedback and UUID.
    fn new_feedback_message(feedback: Self::FeedbackContent, uuid: [u8; 16]) -> Self::Feedback;
}

/// Trait for action goal types.
pub trait ActionGoal {
    /// The request message type for sending a goal.
    type Request: TypeSupport + GetUUID;

    /// The response message type for goal acceptance/rejection.
    type Response: TypeSupport + GoalResponse;

    /// Returns an opaque pointer to the goal service type support structure.
    fn type_support() -> *const c_void;
}

/// Trait for types that contain a UUID.
///
/// Used for tracking goals and feedback in actions.
pub trait GetUUID {
    /// Returns a reference to the UUID.
    fn get_uuid(&self) -> &[u8; 16];
}

/// Trait for action goal response types.
pub trait GoalResponse {
    /// Returns whether the goal was accepted.
    fn is_accepted(&self) -> bool;

    /// Returns the timestamp of the response.
    fn get_time_stamp(&self) -> UnsafeTime;

    /// Creates a new goal response with the given acceptance status and timestamp.
    fn new(accepted: bool, stamp: UnsafeTime) -> Self;
}

/// Trait for action result types.
pub trait ActionResult {
    /// The request message type for getting a result.
    type Request: TypeSupport + GetUUID;

    /// The response message type containing the result.
    type Response: TypeSupport + ResultResponse;

    /// Returns an opaque pointer to the result service type support structure.
    fn type_support() -> *const c_void;
}

/// Trait for action result response types.
pub trait ResultResponse {
    /// Returns the status code of the result.
    fn get_status(&self) -> u8;
}
