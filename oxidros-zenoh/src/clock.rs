//! Fake clock implementation for oxidros-zenoh
//!
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::Result;

/// A clock. For now only SystemTime/ROSTime is implemented.
#[derive(Debug)]
pub struct Clock;

impl Clock {
    /// Create a clock.
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Get duration since epoc
    pub fn get_now(&mut self) -> Result<Duration> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| oxidros_core::Error::Other(format!("{e}")))
    }
}
