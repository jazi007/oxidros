//! CDR (Common Data Representation) serialization support
//!
//! This module provides CDR serialization and deserialization for ROS2 messages,
//! following the RTPS specification v2.5 and DDS X-Types 1.3.
//!
//! # CDR Encapsulation Header
//!
//! All CDR-encoded data is prefixed with a 4-byte encapsulation header:
//! - Bytes 0-1: Representation Identifier (encoding format + endianness)
//! - Bytes 2-3: Options (typically reserved, set to 0x0000)
//!
//! # Supported Encodings
//!
//! This implementation uses the `cdr_encoding` crate which only supports **plain CDR v1**:
//! - `CdrLE` (0x0001) - CDR Little Endian ✓
//! - `CdrBE` (0x0000) - CDR Big Endian ✓
//!
//! The following encodings are **NOT supported** and will return an error:
//! - Parameter List CDR (PL_CDR_BE, PL_CDR_LE)
//! - XCDR v2 encodings (XCDR2_BE, XCDR2_LE, D_CDR2_*, PL_XCDR2_*)
//!
//! # References
//!
//! - RTPS v2.5 Section 10.5, Table 10.3 - Representation Identifier values
//! - DDS X-Types 1.3 Section 7.6.2.1.2 - XCDR encoding identifiers

use crate::error::{Error, Result};

/// CDR Representation Identifier (2 bytes)
///
/// Per RTPS v2.5 Section 10.5, Table 10.3 and DDS X-Types 1.3 Section 7.6.2.1.2
///
/// The representation identifier specifies the encoding format and byte order
/// used for the serialized data payload.
///
/// # Supported Encodings
///
/// Only plain CDR v1 is supported by this implementation:
/// - [`CdrLE`](Self::CdrLE) - CDR Little Endian (recommended, default)
/// - [`CdrBE`](Self::CdrBE) - CDR Big Endian
///
/// All other encodings (Parameter List, XCDR v2) will fail during serialization/deserialization.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationIdentifier {
    /// CDR Big Endian (v1) - `[0x00, 0x00]` ✓ Supported
    CdrBE = 0x0000,
    /// CDR Little Endian (v1) - `[0x00, 0x01]` ✓ Supported
    CdrLE = 0x0001,
    /// Parameter List CDR Big Endian - `[0x00, 0x02]` ✗ Not supported
    PlCdrBE = 0x0002,
    /// Parameter List CDR Little Endian - `[0x00, 0x03]` ✗ Not supported
    PlCdrLE = 0x0003,
    /// XCDR v2 Big Endian (X-Types 1.3) - `[0x00, 0x06]` ✗ Not supported
    Xcdr2BE = 0x0006,
    /// XCDR v2 Little Endian (X-Types 1.3) - `[0x00, 0x07]` ✗ Not supported
    Xcdr2LE = 0x0007,
    /// Delimited CDR v2 Big Endian - `[0x00, 0x08]` ✗ Not supported
    DCdr2BE = 0x0008,
    /// Delimited CDR v2 Little Endian - `[0x00, 0x09]` ✗ Not supported
    DCdr2LE = 0x0009,
    /// Parameter List XCDR v2 Big Endian - `[0x00, 0x0a]` ✗ Not supported
    PlXcdr2BE = 0x000a,
    /// Parameter List XCDR v2 Little Endian - `[0x00, 0x0b]` ✗ Not supported
    PlXcdr2LE = 0x000b,
}

impl RepresentationIdentifier {
    /// Returns true if this representation uses little-endian byte order
    pub const fn is_little_endian(&self) -> bool {
        matches!(
            self,
            Self::CdrLE | Self::PlCdrLE | Self::Xcdr2LE | Self::DCdr2LE | Self::PlXcdr2LE
        )
    }

    /// Returns true if this representation uses big-endian byte order
    pub const fn is_big_endian(&self) -> bool {
        !self.is_little_endian()
    }

    /// Returns true if this is a plain CDR v1 encoding (CdrBE or CdrLE)
    ///
    /// Only plain CDR v1 encodings are supported by this implementation.
    pub const fn is_plain_cdr(&self) -> bool {
        matches!(self, Self::CdrBE | Self::CdrLE)
    }

    /// Returns true if this encoding is supported by the current implementation
    ///
    /// Currently only plain CDR v1 (CdrBE, CdrLE) is supported.
    /// Parameter List and XCDR v2 encodings are not supported.
    pub const fn is_supported(&self) -> bool {
        self.is_plain_cdr()
    }

    /// Returns true if this is a CDR v1 encoding (including Parameter List CDR)
    pub const fn is_cdr_v1(&self) -> bool {
        matches!(
            self,
            Self::CdrBE | Self::CdrLE | Self::PlCdrBE | Self::PlCdrLE
        )
    }

    /// Returns true if this is a CDR v2 / XCDR2 encoding
    pub const fn is_cdr_v2(&self) -> bool {
        matches!(
            self,
            Self::Xcdr2BE
                | Self::Xcdr2LE
                | Self::DCdr2BE
                | Self::DCdr2LE
                | Self::PlXcdr2BE
                | Self::PlXcdr2LE
        )
    }

    /// Returns true if this is a Parameter List encoding
    pub const fn is_parameter_list(&self) -> bool {
        matches!(
            self,
            Self::PlCdrBE | Self::PlCdrLE | Self::PlXcdr2BE | Self::PlXcdr2LE
        )
    }

    /// Convert to raw bytes (big-endian as per spec)
    pub const fn to_bytes(&self) -> [u8; 2] {
        let val = *self as u16;
        [(val >> 8) as u8, val as u8]
    }

    /// Parse from raw bytes
    pub fn from_bytes(bytes: [u8; 2]) -> Result<Self> {
        let val = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
        match val {
            0x0000 => Ok(Self::CdrBE),
            0x0001 => Ok(Self::CdrLE),
            0x0002 => Ok(Self::PlCdrBE),
            0x0003 => Ok(Self::PlCdrLE),
            0x0006 => Ok(Self::Xcdr2BE),
            0x0007 => Ok(Self::Xcdr2LE),
            0x0008 => Ok(Self::DCdr2BE),
            0x0009 => Ok(Self::DCdr2LE),
            0x000a => Ok(Self::PlXcdr2BE),
            0x000b => Ok(Self::PlXcdr2LE),
            _ => Err(Error::CdrError(format!(
                "Unknown representation identifier: 0x{:04x}",
                val
            ))),
        }
    }
}

impl Default for RepresentationIdentifier {
    /// Default to CDR Little Endian (most common for x86/ARM)
    fn default() -> Self {
        Self::CdrLE
    }
}

/// CDR Encapsulation Header (4 bytes)
///
/// Per RTPS v2.5 and DDS X-Types 1.3, all CDR-encoded data is prefixed
/// with a 4-byte encapsulation header:
///
/// ```text
/// +--------+--------+--------+--------+
/// | Rep ID (2 bytes)| Options (2 bytes)|
/// +--------+--------+--------+--------+
/// ```
///
/// # Example
///
/// ```
/// use ros2_types::cdr::{CdrEncapsulationHeader, RepresentationIdentifier};
///
/// // Default header (CDR Little Endian)
/// let header = CdrEncapsulationHeader::default();
/// assert_eq!(header.to_bytes(), [0x00, 0x01, 0x00, 0x00]);
///
/// // Parse header from bytes
/// let header = CdrEncapsulationHeader::from_bytes(&[0x00, 0x01, 0x00, 0x00]).unwrap();
/// assert!(header.representation_id.is_little_endian());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CdrEncapsulationHeader {
    /// Representation identifier (encoding format + endianness)
    pub representation_id: RepresentationIdentifier,
    /// Options field (typically 0x0000, reserved for future use)
    pub options: u16,
}

impl CdrEncapsulationHeader {
    /// Size of the encapsulation header in bytes
    pub const SIZE: usize = 4;

    /// Create a new encapsulation header
    pub const fn new(representation_id: RepresentationIdentifier) -> Self {
        Self {
            representation_id,
            options: 0,
        }
    }

    /// Create a header with custom options
    pub const fn with_options(representation_id: RepresentationIdentifier, options: u16) -> Self {
        Self {
            representation_id,
            options,
        }
    }

    /// Serialize to 4 bytes
    pub const fn to_bytes(&self) -> [u8; 4] {
        let rep_bytes = self.representation_id.to_bytes();
        [
            rep_bytes[0],
            rep_bytes[1],
            (self.options >> 8) as u8,
            self.options as u8,
        ]
    }

    /// Parse from bytes
    ///
    /// # Errors
    ///
    /// Returns `Error::CdrError` if:
    /// - The slice is shorter than 4 bytes
    /// - The representation identifier is unknown
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(Error::CdrError(format!(
                "CDR encapsulation header requires {} bytes, got {}",
                Self::SIZE,
                bytes.len()
            )));
        }

        let representation_id = RepresentationIdentifier::from_bytes([bytes[0], bytes[1]])?;
        let options = ((bytes[2] as u16) << 8) | (bytes[3] as u16);

        Ok(Self {
            representation_id,
            options,
        })
    }
}

impl Default for CdrEncapsulationHeader {
    /// Default to CDR Little Endian with no options
    fn default() -> Self {
        Self::new(RepresentationIdentifier::CdrLE)
    }
}

/// Trait for CDR serialization/deserialization
///
/// This trait provides methods to serialize and deserialize types using
/// CDR (Common Data Representation) encoding with proper encapsulation headers.
///
/// # Implementation
///
/// This trait is automatically implemented for any type that implements
/// `serde::Serialize` and `serde::de::DeserializeOwned`.
///
/// # Wire Format
///
/// Serialized data includes a 4-byte CDR encapsulation header followed
/// by the CDR-encoded payload:
///
/// ```text
/// +------------------+----------------------+
/// | Header (4 bytes) | Payload (N bytes)    |
/// +------------------+----------------------+
/// ```
pub trait CdrSerde: Sized {
    /// Serialize to CDR-encoded bytes with encapsulation header
    ///
    /// The output includes a 4-byte CDR encapsulation header (default: CDR LE)
    /// followed by the serialized payload.
    fn serialize(&self) -> Result<Vec<u8>>;

    /// Serialize with a specific encapsulation header
    fn serialize_with_header(&self, header: CdrEncapsulationHeader) -> Result<Vec<u8>>;

    /// Deserialize from CDR-encoded bytes
    ///
    /// Parses the encapsulation header to determine the encoding format
    /// and deserializes the payload accordingly.
    ///
    /// # Errors
    ///
    /// Returns `Error::CdrError` if:
    /// - The input is shorter than 4 bytes
    /// - The encapsulation header is invalid or unsupported
    /// - Deserialization fails
    fn deserialize(bytes: &[u8]) -> Result<Self>;
}

impl<T: serde::Serialize + serde::de::DeserializeOwned> CdrSerde for T {
    fn serialize(&self) -> Result<Vec<u8>> {
        self.serialize_with_header(CdrEncapsulationHeader::default())
    }

    fn serialize_with_header(&self, header: CdrEncapsulationHeader) -> Result<Vec<u8>> {
        // Only plain CDR v1 is supported
        if !header.representation_id.is_supported() {
            return Err(Error::CdrError(format!(
                "Unsupported CDR encoding for serialization: {:?}. Only CdrLE and CdrBE are supported.",
                header.representation_id
            )));
        }

        let mut result = header.to_bytes().to_vec();

        let buffer = if header.representation_id.is_little_endian() {
            cdr_encoding::to_vec::<T, byteorder::LittleEndian>(self)
                .map_err(|e| Error::CdrError(e.to_string()))?
        } else {
            cdr_encoding::to_vec::<T, byteorder::BigEndian>(self)
                .map_err(|e| Error::CdrError(e.to_string()))?
        };

        result.extend(buffer);
        Ok(result)
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        let header = CdrEncapsulationHeader::from_bytes(bytes)?;

        // Only plain CDR v1 is supported
        if !header.representation_id.is_supported() {
            return Err(Error::CdrError(format!(
                "Unsupported CDR encoding for deserialization: {:?}. Only CdrLE and CdrBE are supported.",
                header.representation_id
            )));
        }

        let payload = &bytes[CdrEncapsulationHeader::SIZE..];

        if header.representation_id.is_little_endian() {
            let (value, _) = cdr_encoding::from_bytes::<T, byteorder::LittleEndian>(payload)
                .map_err(|e| Error::CdrError(e.to_string()))?;
            Ok(value)
        } else {
            let (value, _) = cdr_encoding::from_bytes::<T, byteorder::BigEndian>(payload)
                .map_err(|e| Error::CdrError(e.to_string()))?;
            Ok(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_representation_identifier_bytes() {
        assert_eq!(RepresentationIdentifier::CdrBE.to_bytes(), [0x00, 0x00]);
        assert_eq!(RepresentationIdentifier::CdrLE.to_bytes(), [0x00, 0x01]);
        assert_eq!(RepresentationIdentifier::PlCdrBE.to_bytes(), [0x00, 0x02]);
        assert_eq!(RepresentationIdentifier::PlCdrLE.to_bytes(), [0x00, 0x03]);
        assert_eq!(RepresentationIdentifier::Xcdr2BE.to_bytes(), [0x00, 0x06]);
        assert_eq!(RepresentationIdentifier::Xcdr2LE.to_bytes(), [0x00, 0x07]);
    }

    #[test]
    fn test_representation_identifier_roundtrip() {
        let identifiers = [
            RepresentationIdentifier::CdrBE,
            RepresentationIdentifier::CdrLE,
            RepresentationIdentifier::PlCdrBE,
            RepresentationIdentifier::PlCdrLE,
            RepresentationIdentifier::Xcdr2BE,
            RepresentationIdentifier::Xcdr2LE,
            RepresentationIdentifier::DCdr2BE,
            RepresentationIdentifier::DCdr2LE,
            RepresentationIdentifier::PlXcdr2BE,
            RepresentationIdentifier::PlXcdr2LE,
        ];

        for id in identifiers {
            let bytes = id.to_bytes();
            let parsed = RepresentationIdentifier::from_bytes(bytes).unwrap();
            assert_eq!(id, parsed);
        }
    }

    #[test]
    fn test_header_default() {
        let header = CdrEncapsulationHeader::default();
        assert_eq!(header.representation_id, RepresentationIdentifier::CdrLE);
        assert_eq!(header.options, 0);
        assert_eq!(header.to_bytes(), [0x00, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn test_header_roundtrip() {
        let header = CdrEncapsulationHeader::new(RepresentationIdentifier::Xcdr2LE);
        let bytes = header.to_bytes();
        let parsed = CdrEncapsulationHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, parsed);
    }

    #[test]
    fn test_header_with_options() {
        let header = CdrEncapsulationHeader::with_options(RepresentationIdentifier::CdrLE, 0x1234);
        assert_eq!(header.to_bytes(), [0x00, 0x01, 0x12, 0x34]);
    }

    #[test]
    fn test_endianness_detection() {
        assert!(RepresentationIdentifier::CdrLE.is_little_endian());
        assert!(RepresentationIdentifier::CdrBE.is_big_endian());
        assert!(RepresentationIdentifier::Xcdr2LE.is_little_endian());
        assert!(RepresentationIdentifier::Xcdr2BE.is_big_endian());
    }

    #[test]
    fn test_cdr_version_detection() {
        assert!(RepresentationIdentifier::CdrLE.is_cdr_v1());
        assert!(RepresentationIdentifier::CdrBE.is_cdr_v1());
        assert!(RepresentationIdentifier::Xcdr2LE.is_cdr_v2());
        assert!(RepresentationIdentifier::Xcdr2BE.is_cdr_v2());
    }

    #[test]
    fn test_supported_encodings() {
        // Only plain CDR v1 is supported
        assert!(RepresentationIdentifier::CdrLE.is_supported());
        assert!(RepresentationIdentifier::CdrBE.is_supported());

        // All others are not supported
        assert!(!RepresentationIdentifier::PlCdrBE.is_supported());
        assert!(!RepresentationIdentifier::PlCdrLE.is_supported());
        assert!(!RepresentationIdentifier::Xcdr2BE.is_supported());
        assert!(!RepresentationIdentifier::Xcdr2LE.is_supported());
        assert!(!RepresentationIdentifier::DCdr2BE.is_supported());
        assert!(!RepresentationIdentifier::DCdr2LE.is_supported());
        assert!(!RepresentationIdentifier::PlXcdr2BE.is_supported());
        assert!(!RepresentationIdentifier::PlXcdr2LE.is_supported());
    }

    #[test]
    fn test_unsupported_encoding_error() {
        // Try to deserialize with an unsupported encoding (PL_CDR_LE)
        let bytes = [0x00, 0x03, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04];
        let result = CdrEncapsulationHeader::from_bytes(&bytes);
        assert!(result.is_ok());
        let header = result.unwrap();
        assert_eq!(header.representation_id, RepresentationIdentifier::PlCdrLE);
        assert!(!header.representation_id.is_supported());
    }

    #[test]
    fn test_invalid_representation_identifier() {
        let result = RepresentationIdentifier::from_bytes([0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn test_header_too_short() {
        let result = CdrEncapsulationHeader::from_bytes(&[0x00, 0x01]);
        assert!(result.is_err());
    }
}
