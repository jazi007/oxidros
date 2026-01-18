//! Type description trait

use crate::{Result, calculate_type_hash};

/// Information needed to construct a ROS2 message type name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageTypeName {
    /// The message type category (typically "msg", "srv", or "action")
    pub message_type: String,
    /// The ROS2 package name (namespace) where the type is defined
    pub package: String,
    /// The ROS2 type name (without package prefix)
    pub type_name: String,
}

impl MessageTypeName {
    /// Create a new MessageTypeName
    pub fn new(
        message_type: impl Into<String>,
        package: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self {
            message_type: message_type.into(),
            package: package.into(),
            type_name: type_name.into(),
        }
    }

    /// Get the full type name in the format "package/message_type/TypeName"
    pub fn full_name(&self) -> String {
        format!("{}/{}/{}", self.package, self.message_type, self.type_name)
    }
}

/// Trait for types that can provide a ROS2 type description
///
/// This trait should be implemented by ROS2 message types to provide
/// the information needed to calculate RIHS01 type hashes.
pub trait TypeDescription {
    /// Get the type description for this type
    ///
    /// Returns a complete type description including all referenced types
    fn type_description() -> crate::types::TypeDescriptionMsg;

    /// Get the message type name information
    ///
    /// Returns the prefix, package, and type name needed to construct
    /// the full ROS2 message type name.
    fn message_type_name() -> MessageTypeName;

    /// Compute the RIHS01 type hash for this type
    ///
    /// This has a default implementation that uses `type_description()`
    /// and calculates the SHA256 hash according to RIHS01 specification.
    fn compute_hash() -> Result<String> {
        let description = Self::type_description();
        calculate_type_hash(&description)
    }
}

/// Trait for ROS2 service types that can provide a type description for hash computation
///
/// This trait is separate from `ServiceMsg` (which is for runtime FFI) and only
/// requires `TypeDescription` on Request/Response types.
pub trait ServiceTypeDescription {
    /// Get the type description for this service
    ///
    /// The service type description has fields:
    /// - `request_message`: Reference to the Request message type
    /// - `response_message`: Reference to the Response message type  
    /// - `event_message`: Reference to the Event message type
    fn type_description() -> crate::types::TypeDescriptionMsg;

    /// Get the service type name information
    fn service_type_name() -> MessageTypeName;

    /// Compute the RIHS01 type hash for this service
    fn compute_hash() -> Result<String> {
        let description = Self::type_description();
        calculate_type_hash(&description)
    }
}

/// Trait for ROS2 action types that can provide a type description for hash computation
///
/// This trait is separate from `ActionMsg` (which is for runtime FFI) and only
/// requires `TypeDescription` on Goal/Result/Feedback types.
pub trait ActionTypeDescription {
    /// Get the type description for this action
    ///
    /// The action type description has fields:
    /// - `goal`: Reference to the Goal message type
    /// - `result`: Reference to the Result message type
    /// - `feedback`: Reference to the Feedback message type
    /// - `send_goal_service`: Reference to the SendGoal service
    /// - `get_result_service`: Reference to the GetResult service
    /// - `feedback_message`: Reference to the FeedbackMessage type
    fn type_description() -> crate::types::TypeDescriptionMsg;

    /// Get the action type name information
    fn action_type_name() -> MessageTypeName;

    /// Compute the RIHS01 type hash for this action
    fn compute_hash() -> Result<String> {
        let description = Self::type_description();
        calculate_type_hash(&description)
    }
}
