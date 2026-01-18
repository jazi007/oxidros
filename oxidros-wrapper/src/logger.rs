//! Implement logger for ros2 logging
//!
use oxidros::{error::Result, logger};
/// Initialize the logger with the node name
pub fn init_logger(name: &str) -> Result<()> {
    logger::init_ros_logging(name);
    Ok(())
}
