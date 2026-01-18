use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{rcl, topic::subscriber::RCLSubscription};

/// A message loaned by a subscriber.
pub struct SubscriberLoanedMessage<T> {
    subscription: Arc<RCLSubscription>,
    chunk: *mut T,
}

impl<T> SubscriberLoanedMessage<T> {
    pub(crate) fn new(subscription: Arc<RCLSubscription>, chunk: *mut T) -> Self {
        Self {
            subscription,
            chunk,
        }
    }
}

impl<T> Deref for SubscriberLoanedMessage<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.chunk }
    }
}

impl<T> DerefMut for SubscriberLoanedMessage<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.chunk }
    }
}

impl<T> Drop for SubscriberLoanedMessage<T> {
    fn drop(&mut self) {
        let guard = rcl::MT_UNSAFE_FN.lock();
        let _ = guard.rcl_return_loaned_message_from_subscription(
            self.subscription.subscription.as_ref(),
            self.chunk as *const _ as *mut _,
        );
    }
}
