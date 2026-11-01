//! Lock-step between the firmware thread and the test driving it.
//!
//! The firmware reads the touch panel once per event-loop pass. That read parks
//! it until the test grants more passes, so the test always inspects a stopped
//! device and every run is repeatable.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};

#[derive(Default)]
struct Gate {
    granted_polls: u64,
    completed_polls: u64,
}

static GATE: Mutex<Gate> = Mutex::new(Gate {
    granted_polls: 0,
    completed_polls: 0,
});
static CHANGED: Condvar = Condvar::new();
static FREE_RUNNING: AtomicBool = AtomicBool::new(false);
static STOPPED: AtomicBool = AtomicBool::new(false);

/// Stops parking at touch reads, for firmware images that drive themselves.
pub fn free_run() {
    FREE_RUNNING.store(true, Ordering::Release);
}

/// The firmware reset itself: no more passes are coming, so nothing may wait for them.
pub fn firmware_stopped() {
    STOPPED.store(true, Ordering::Release);
    let _gate = GATE.lock().expect("control gate poisoned");
    CHANGED.notify_all();
}

pub fn is_stopped() -> bool {
    STOPPED.load(Ordering::Acquire)
}

/// Called by the firmware's touch read.
pub fn firmware_poll() {
    if FREE_RUNNING.load(Ordering::Acquire) {
        return;
    }
    let mut gate = GATE.lock().expect("control gate poisoned");
    gate.completed_polls += 1;
    CHANGED.notify_all();
    while gate.completed_polls >= gate.granted_polls {
        gate = CHANGED.wait(gate).expect("control gate poisoned");
    }
}

/// Lets the firmware run `polls` more event-loop passes and waits until it stops again.
pub fn run_polls(polls: u64) {
    let mut gate = GATE.lock().expect("control gate poisoned");
    gate.granted_polls = gate.completed_polls + polls;
    CHANGED.notify_all();
    while gate.completed_polls < gate.granted_polls && !is_stopped() {
        gate = CHANGED.wait(gate).expect("control gate poisoned");
    }
}
