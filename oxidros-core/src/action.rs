//! Action-related types and enums.

/// Status of an action goal.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GoalStatus {
    /// Goal status is unknown.
    Unknown = 0,

    /// Goal has been accepted by the action server.
    Accepted = 1,

    /// Goal is currently being executed.
    Executing = 2,

    /// Goal is in the process of being canceled.
    Canceling = 3,

    /// Goal completed successfully.
    Succeeded = 4,

    /// Goal was canceled.
    Canceled = 5,

    /// Goal was aborted by the action server.
    Aborted = 6,
}

impl From<i8> for GoalStatus {
    fn from(s: i8) -> Self {
        match s {
            0 => GoalStatus::Unknown,
            1 => GoalStatus::Accepted,
            2 => GoalStatus::Executing,
            3 => GoalStatus::Canceling,
            4 => GoalStatus::Succeeded,
            5 => GoalStatus::Canceled,
            6 => GoalStatus::Aborted,
            _ => GoalStatus::Unknown,
        }
    }
}

impl From<GoalStatus> for i8 {
    fn from(status: GoalStatus) -> Self {
        status as i8
    }
}

/// Events that can occur during action goal processing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GoalEvent {
    /// Execute the goal.
    Execute = 0,

    /// Cancel the goal.
    CancelGoal = 1,

    /// Goal succeeded.
    Succeed = 2,

    /// Goal was aborted.
    Abort = 3,

    /// Goal was canceled.
    Canceled = 4,

    /// Number of events
    NumEvents = 5,
}

impl From<i8> for GoalEvent {
    fn from(s: i8) -> Self {
        match s {
            0 => GoalEvent::Execute,
            1 => GoalEvent::CancelGoal,
            2 => GoalEvent::Succeed,
            3 => GoalEvent::Abort,
            4 => GoalEvent::Canceled,
            5 => GoalEvent::NumEvents,
            _ => GoalEvent::Execute,
        }
    }
}

impl From<GoalEvent> for i8 {
    fn from(event: GoalEvent) -> Self {
        event as i8
    }
}
