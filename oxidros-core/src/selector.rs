//! Selector abstraction for event multiplexing in ROS2.
//!
//! This module provides traits for implementing event-driven architectures
//! where multiple sources (subscriptions, services, timers) can be waited on
//! simultaneously.

use crate::{Message, ServiceMsg, parameter::Parameters};
use std::collections::BTreeSet;

/// Result type for callback functions.
#[derive(Debug, Eq, PartialEq)]
pub enum CallbackResult {
    /// Callback executed successfully, keep it registered.
    Ok,

    /// Remove this callback from the selector.
    Remove,
}

pub type ServerCallback<T> =
    Box<dyn FnMut(Message<<T as ServiceMsg>::Request>) -> <T as ServiceMsg>::Response>;
pub type ParameterCallback = Box<dyn FnMut(&mut Parameters, BTreeSet<String>)>;

pub struct ConditionHandler<T> {
    pub is_once: bool,
    pub event: T,
    pub handler: Option<Box<dyn FnMut() -> CallbackResult>>,
}

pub type ActionHandler = Box<dyn FnMut() -> CallbackResult>;
