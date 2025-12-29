use std::ops::DerefMut;

/// A smart pointer for the message taken from the topic with `rcl_take` or `rcl_take_loaned_message`.
pub enum TakenMsg<T> {
    Copied(T),
    Loaned(Box<dyn DerefMut<Target = T>>),
}

impl<T> TakenMsg<T> {
    // Returns the owned message without cloning
    // if the subscriber owns the memory region and its data.
    // None is returned when it does not own the memory region (i.e. the message is loaned).
    pub fn get_owned(self) -> Option<T> {
        match self {
            TakenMsg::Copied(inner) => Some(inner),
            TakenMsg::Loaned(_) => None,
        }
    }
}

impl<T> std::ops::Deref for TakenMsg<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            TakenMsg::Copied(copied) => copied,
            TakenMsg::Loaned(loaned) => loaned.deref(),
        }
    }
}

impl<T> std::ops::DerefMut for TakenMsg<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            TakenMsg::Copied(copied) => copied,
            TakenMsg::Loaned(loaned) => loaned.deref_mut(),
        }
    }
}

unsafe impl<T> Sync for TakenMsg<T> {}
unsafe impl<T> Send for TakenMsg<T> {}
