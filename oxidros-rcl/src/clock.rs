use crate::{error::OResult, get_allocator, rcl};

/// A clock. For now only SystemTime/ROSTime is implemented.
#[derive(Debug)]
pub struct Clock {
    pub(crate) clock: *mut rcl::rcl_clock_t,
}

impl Clock {
    /// Create a clock.
    pub fn new() -> OResult<Self> {
        let mut clock = unsafe { std::mem::zeroed() };

        let guard = rcl::MT_UNSAFE_FN.lock();
        guard.rcl_ros_clock_init(&mut clock, &mut get_allocator())?;

        let b = Box::new(clock);
        Ok(Self {
            clock: Box::into_raw(b),
        })
    }

    pub(crate) unsafe fn as_ptr_mut(&self) -> *mut rcl::rcl_clock_t {
        self.clock
    }

    pub fn get_now(&mut self) -> OResult<rcl::rcl_time_point_value_t> {
        let mut now = unsafe { std::mem::zeroed() };
        rcl::MTSafeFn::rcl_clock_get_now(self.clock, &mut now)?;
        Ok(now)
    }
}

impl Drop for Clock {
    fn drop(&mut self) {
        let guard = rcl::MT_UNSAFE_FN.lock();
        let _ = guard.rcl_ros_clock_fini(self.clock);
        let _ = unsafe { Box::from_raw(self.clock) };
    }
}

unsafe impl Send for Clock {}
unsafe impl Sync for Clock {}
