//! Message attachment encoding/decoding.
//!
//! This module handles the encoding and decoding of message metadata
//! attached to Zenoh messages, as specified in rmw_zenoh design.
//!
//! # Reference
//!
//! See [rmw_zenoh design - Publishers](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md#publishers)
//!
//! # Attachment Format (33 bytes)
//!
//! | Offset | Size | Content |
//! |--------|------|---------|
//! | 0 | 8 | Sequence number (i64 LE) |
//! | 8 | 8 | Timestamp in nanoseconds since UNIX epoch (i64 LE) |
//! | 16 | 1 | GID length (always 16) |
//! | 17 | 16 | Publisher/Client GID (16 bytes) |
//!
//! Total: 33 bytes

use crate::error::{Error, Result};
use std::time::{SystemTime, UNIX_EPOCH};

/// Size of the attachment in bytes.
pub const ATTACHMENT_SIZE: usize = 33;

/// Size of the GID (Global Identifier).
pub const GID_SIZE: usize = 16;

/// Attachment data for messages.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Sequence number of the message.
    pub sequence_number: i64,
    /// Timestamp in nanoseconds since UNIX epoch.
    pub timestamp_ns: i64,
    /// Global identifier (publisher or client GID).
    pub gid: [u8; GID_SIZE],
}

impl Default for Attachment {
    fn default() -> Self {
        Self::new(0, [0; GID_SIZE])
    }
}

impl Attachment {
    /// Create a new attachment with current timestamp.
    pub fn new(sequence_number: i64, gid: [u8; GID_SIZE]) -> Self {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);

        Self {
            sequence_number,
            timestamp_ns,
            gid,
        }
    }

    /// Encode the attachment to bytes.
    ///
    /// # Format
    ///
    /// - 8 bytes: sequence number (i64 LE)
    /// - 8 bytes: timestamp (i64 LE)
    /// - 1 byte: GID length (16)
    /// - 16 bytes: GID
    pub fn to_bytes(&self) -> [u8; ATTACHMENT_SIZE] {
        let mut bytes = [0u8; ATTACHMENT_SIZE];

        // Sequence number (i64 LE)
        bytes[0..8].copy_from_slice(&self.sequence_number.to_le_bytes());

        // Timestamp (i64 LE)
        bytes[8..16].copy_from_slice(&self.timestamp_ns.to_le_bytes());

        // GID length (always 16)
        bytes[16] = GID_SIZE as u8;

        // GID
        bytes[17..33].copy_from_slice(&self.gid);

        bytes
    }

    /// Decode an attachment from bytes.
    ///
    /// # Errors
    ///
    /// Returns `InvalidAttachment` if the bytes are too short or malformed.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < ATTACHMENT_SIZE {
            return Err(Error::InvalidAttachment(format!(
                "attachment too short: expected {} bytes, got {}",
                ATTACHMENT_SIZE,
                bytes.len()
            )));
        }

        let sequence_number = i64::from_le_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| Error::InvalidAttachment("invalid sequence number".into()))?,
        );
        let timestamp_ns = i64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| Error::InvalidAttachment("invalid timestamp".into()))?,
        );
        let gid_len = bytes[16] as usize;

        if gid_len != GID_SIZE {
            return Err(Error::InvalidAttachment(format!(
                "invalid GID length: expected {}, got {}",
                GID_SIZE, gid_len
            )));
        }

        let mut gid = [0u8; GID_SIZE];
        gid.copy_from_slice(&bytes[17..33]);

        Ok(Self {
            sequence_number,
            timestamp_ns,
            gid,
        })
    }
}

// ============================================================================
// MessageInfo conversion
// ============================================================================

impl From<Attachment> for oxidros_core::MessageInfo {
    fn from(attachment: Attachment) -> Self {
        Self {
            sequence_number: attachment.sequence_number,
            source_timestamp_ns: attachment.timestamp_ns,
            publisher_gid: attachment.gid,
        }
    }
}

impl From<&Attachment> for oxidros_core::MessageInfo {
    fn from(attachment: &Attachment) -> Self {
        Self {
            sequence_number: attachment.sequence_number,
            source_timestamp_ns: attachment.timestamp_ns,
            publisher_gid: attachment.gid,
        }
    }
}

/// Generate a random GID using UUID v4.
pub fn generate_gid() -> [u8; GID_SIZE] {
    *uuid::Uuid::new_v4().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_roundtrip() {
        let gid = generate_gid();
        let attachment = Attachment::new(42, gid);

        let bytes = attachment.to_bytes();
        assert_eq!(bytes.len(), ATTACHMENT_SIZE);

        let decoded = Attachment::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.sequence_number, 42);
        assert_eq!(decoded.gid, gid);
    }

    #[test]
    fn test_attachment_from_short_bytes() {
        let short_bytes = [0u8; 10];
        assert!(matches!(
            Attachment::from_bytes(&short_bytes),
            Err(Error::InvalidAttachment(_))
        ));
    }

    #[test]
    fn test_attachment_invalid_gid_length() {
        let mut bytes = [0u8; ATTACHMENT_SIZE];
        bytes[16] = 8; // Wrong GID length (should be 16)
        assert!(matches!(
            Attachment::from_bytes(&bytes),
            Err(Error::InvalidAttachment(_))
        ));
    }
}
