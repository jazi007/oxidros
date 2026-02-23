//! Receive signals on a thread for graceful shutdown.

use crate::{
    logger::{Logger, pr_info_in},
    rcl,
    selector::guard_condition::GuardCondition,
};
use once_cell::sync::Lazy;
#[cfg(target_os = "windows")]
use parking_lot::Condvar;
use parking_lot::{Mutex, RawMutex, lock_api::MutexGuard};

use signal_hook::consts::*;

#[cfg(not(target_os = "windows"))]
use signal_hook::iterator::{Handle, Signals, SignalsInfo};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::Display,
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, JoinHandle},
};

#[derive(Eq, PartialEq, Ord, PartialOrd)]
struct KeyCond(*const rcl::rcl_guard_condition_t);

unsafe impl Sync for KeyCond {}
unsafe impl Send for KeyCond {}

type ConditionSet = BTreeMap<KeyCond, GuardCondition>;

static INITIALIZER: std::sync::OnceLock<()> = std::sync::OnceLock::new();
static GUARD_COND: Lazy<Mutex<ConditionSet>> = Lazy::new(|| Mutex::new(ConditionSet::new()));
#[cfg(not(target_os = "windows"))]
static SIGHDL: Lazy<Mutex<Option<Handle>>> = Lazy::new(|| Mutex::new(None));
static THREAD: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));
static IS_HALT: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static WIN_SIGNAL_NOTIFY: Lazy<(Mutex<bool>, Condvar)> =
    Lazy::new(|| (Mutex::new(false), Condvar::new()));

#[derive(Debug)]
pub struct Signaled;

impl Display for Signaled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signaled")
    }
}

impl Error for Signaled {}

impl From<Signaled> for oxidros_core::Error {
    fn from(value: Signaled) -> Self {
        Self::Other(value.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn init() {
    INITIALIZER.get_or_init(|| {
        let signals = Signals::new([SIGHUP, SIGTERM, SIGQUIT]).unwrap();
        let handle = signals.handle();

        let mut guard = SIGHDL.lock();
        *guard = Some(handle);

        let th = thread::spawn(move || handler(signals));
        *THREAD.lock() = Some(th);
    });
}

#[cfg(target_os = "windows")]
pub(crate) fn init() {
    INITIALIZER.get_or_init(|| {
        // SAFETY: On Windows, signal handlers run in a separate thread created by Windows
        // (Console Control Handler), so using locks/condvar is safe.
        unsafe {
            signal_hook::low_level::register(SIGTERM, || {
                let (lock, cvar) = &*WIN_SIGNAL_NOTIFY;
                let mut signaled = lock.lock();
                *signaled = true;
                cvar.notify_all();
            })
            .unwrap();
        }
        let th = thread::spawn(handler);
        *THREAD.lock() = Some(th);
    });
}

pub(crate) fn register_guard_condition(cond: GuardCondition) {
    let mut guard = get_guard_condition();
    guard.insert(KeyCond(cond.cond.as_ptr()), cond);
}

pub(crate) fn unregister_guard_condition(cond: &GuardCondition) {
    let mut guard = get_guard_condition();
    guard.remove(&KeyCond(cond.cond.as_ptr()));
}

/// After receiving SIGINT, SIGTERM, SIGQUIT, or SIGHUP, this function return `true`.
/// If `is_halt()` is `true`, some functions to receive or wait returns error to halt the process.
pub fn is_halt() -> bool {
    IS_HALT.load(Ordering::Relaxed)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn halt() {
    let mut sig = SIGHDL.lock();
    let sig = if let Some(sig) = sig.take() {
        sig
    } else {
        return;
    };
    sig.close();

    let mut th = THREAD.lock();
    let th = if let Some(th) = th.take() {
        th
    } else {
        return;
    };
    th.join().unwrap();
}
#[cfg(target_os = "windows")]
pub(crate) fn halt() {}

fn get_guard_condition() -> MutexGuard<'static, RawMutex, ConditionSet> {
    init();
    GUARD_COND.lock()
}

#[cfg(not(target_os = "windows"))]
fn handler(mut signals: SignalsInfo) {
    if let Some(signal) = signals.forever().next() {
        match signal {
            SIGTERM | SIGQUIT | SIGHUP => {
                IS_HALT.store(true, Ordering::SeqCst);
                let mut cond = get_guard_condition();
                let cond = std::mem::take(&mut *cond);

                for (_, c) in cond {
                    c.trigger().unwrap();
                }

                let logger = Logger::new("oxidros");
                pr_info_in!(logger, "Received signal: {signal}");
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(target_os = "windows")]
fn handler() {
    let (lock, cvar) = &*WIN_SIGNAL_NOTIFY;
    let mut signaled = lock.lock();
    while !*signaled {
        cvar.wait(&mut signaled);
    }

    IS_HALT.store(true, Ordering::SeqCst);
    let mut cond = get_guard_condition();
    let cond = std::mem::take(&mut *cond);

    for (_, c) in cond {
        c.trigger().unwrap();
    }

    let logger = Logger::new("oxidros");
    pr_info_in!(logger, "Received signal");
}
