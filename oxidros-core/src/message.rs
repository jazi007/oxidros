use std::ops::{Deref, DerefMut};

/// Metadata about a received message.
#[derive(Debug, Clone, Copy, Default)]
pub struct MessageInfo {
    /// Sequence number of the message.
    pub sequence_number: i64,
    /// Source timestamp in nanoseconds since UNIX epoch.
    pub source_timestamp_ns: i64,
    /// Publisher's global identifier (GID).
    pub publisher_gid: [u8; 16],
}

/// The underlying message data, which can be copied or loaned (zero-copy).
pub enum MessageData<T> {
    /// Message data was copied into owned memory.
    Copied(T),
    /// Message data is loaned from shared memory (zero-copy).
    Loaned(Box<dyn DerefMut<Target = T>>),
}

impl<T> MessageData<T> {
    /// Returns the owned message if it was copied, consuming self.
    /// Returns `None` if the message is loaned (zero-copy).
    pub fn into_owned(self) -> Option<T> {
        match self {
            MessageData::Copied(inner) => Some(inner),
            MessageData::Loaned(_) => None,
        }
    }

    /// Returns `true` if the message data is copied (owned).
    pub fn is_copied(&self) -> bool {
        matches!(self, MessageData::Copied(_))
    }

    /// Returns `true` if the message data is loaned (zero-copy).
    pub fn is_loaned(&self) -> bool {
        matches!(self, MessageData::Loaned(_))
    }
}

impl<T> Deref for MessageData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            MessageData::Copied(copied) => copied,
            MessageData::Loaned(loaned) => loaned.deref(),
        }
    }
}

impl<T> DerefMut for MessageData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MessageData::Copied(copied) => copied,
            MessageData::Loaned(loaned) => loaned.deref_mut(),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for MessageData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageData::Copied(data) => f.debug_tuple("Copied").field(data).finish(),
            MessageData::Loaned(data) => {
                // Deref to get &T which implements Debug
                let inner: &T = data.deref();
                f.debug_tuple("Loaned").field(inner).finish()
            }
        }
    }
}

// SAFETY: MessageData is Send/Sync if T is, loaned data comes from ROS2 middleware
unsafe impl<T> Sync for MessageData<T> {}
unsafe impl<T> Send for MessageData<T> {}

/// A received message with its data and metadata.
pub struct Message<T> {
    /// The message data (copied or loaned).
    pub sample: MessageData<T>,
    /// Metadata about the message (sequence number, timestamp, publisher GID).
    pub info: MessageInfo,
}

impl<T> Message<T> {
    /// Create a new message with copied data and info.
    pub fn new(data: T, info: MessageInfo) -> Self {
        Self {
            sample: MessageData::Copied(data),
            info,
        }
    }

    /// Create a new message with loaned data and info.
    pub fn new_loaned(data: Box<dyn DerefMut<Target = T>>, info: MessageInfo) -> Self {
        Self {
            sample: MessageData::Loaned(data),
            info,
        }
    }

    /// Consume the message and return the owned data if it was copied.
    /// Returns `None` if the data was loaned.
    pub fn into_owned(self) -> Option<T> {
        self.sample.into_owned()
    }

    /// Returns `true` if the message data is copied (owned).
    pub fn is_copied(&self) -> bool {
        self.sample.is_copied()
    }

    /// Returns `true` if the message data is loaned (zero-copy).
    pub fn is_loaned(&self) -> bool {
        self.sample.is_loaned()
    }
}

impl<T> Deref for Message<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.sample
    }
}

impl<T> DerefMut for Message<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sample
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Message<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("sample", &self.sample)
            .field("info", &self.info)
            .finish()
    }
}

// SAFETY: Message is Send/Sync if T is
unsafe impl<T> Sync for Message<T> {}
unsafe impl<T> Send for Message<T> {}
