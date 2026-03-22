use thiserror::Error;

#[derive(Debug, Error)]
pub enum DynamicError {
    #[error("CDR buffer too short: need {need} bytes at offset {offset}, have {have}")]
    BufferTooShort {
        offset: usize,
        need: usize,
        have: usize,
    },

    #[error("Unsupported CDR representation: 0x{0:04x}")]
    UnsupportedRepresentation(u16),

    #[error("Unknown field type_id: {0}")]
    UnknownFieldType(u8),

    #[error("Referenced type not found: {0}")]
    ReferencedTypeNotFound(String),

    #[error("Encoder type mismatch for '{field}': expected {expected}, got {got}")]
    EncoderTypeMismatch {
        expected: String,
        got: String,
        field: String,
    },

    #[error("Invalid UTF-8 in string field: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, DynamicError>;
