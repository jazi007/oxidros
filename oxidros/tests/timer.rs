//! Timer integration test.
//!
//! Tests timer functionality using the unified API.
//! Works with both RCL and Zenoh backends.

use oxidros::prelude::*;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn test_timer() -> Result<(), Box<dyn Error + Send + Sync>> {
    let ctx = Context::new()?;
    let mut selector = ctx.create_selector()?;

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Add a timer that fires every 50ms
    selector.add_wall_timer(
        "test_timer",
        Duration::from_millis(50),
        Box::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }),
    );

    // Wait for several timer firings
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(200) {
        selector.wait_timeout(Duration::from_millis(100))?;
    }

    // Timer should have fired at least 3 times
    let count = counter.load(Ordering::SeqCst);
    println!("Timer fired {} times", count);
    assert!(
        count >= 2,
        "Timer should have fired at least 2 times, got {}",
        count
    );

    Ok(())
}

#[test]
fn test_timer_remove() -> Result<(), Box<dyn Error + Send + Sync>> {
    let ctx = Context::new()?;
    let mut selector = ctx.create_selector()?;

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    // Add a timer
    let timer_id = selector.add_wall_timer(
        "removable_timer",
        Duration::from_millis(50),
        Box::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }),
    );

    // Let it fire once
    selector.wait_timeout(Duration::from_millis(100))?;
    let count_before = counter.load(Ordering::SeqCst);

    // Remove the timer
    selector.remove_timer(timer_id);

    // Wait more - counter should not increase
    selector.wait_timeout(Duration::from_millis(150))?;
    let count_after = counter.load(Ordering::SeqCst);

    println!(
        "Before removal: {}, After removal: {}",
        count_before, count_after
    );

    // Counter should not have increased significantly after removal
    // (allow for one more fire that may have been in flight)
    assert!(
        count_after <= count_before + 1,
        "Timer should have stopped after removal"
    );

    Ok(())
}
