use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

static MICROS: AtomicU64 = AtomicU64::new(0);
static PEER_CORE_RUNNING: AtomicBool = AtomicBool::new(false);
const PEER_CORE_SLICE: Duration = Duration::from_micros(100);
const MICROS_PER_MILLI: u64 = 1_000;

/// Each read costs a microsecond, so a busy-wait on time always terminates.
pub fn read_micros() -> u64 {
    MICROS.fetch_add(1, Ordering::Relaxed)
}

pub fn advance_micros(micros: u64) {
    MICROS.fetch_add(micros, Ordering::Relaxed);
}

/// The peer core runs on a host thread in real time, so virtual delays must leave it room.
pub fn peer_core_started() {
    PEER_CORE_RUNNING.store(true, Ordering::Release);
}

/// A blocking wait: time passes, and the peer core gets a real slice to do its work.
pub fn wait_micros(micros: u64) {
    advance_micros(micros);
    if micros >= MICROS_PER_MILLI && PEER_CORE_RUNNING.load(Ordering::Acquire) {
        std::thread::sleep(PEER_CORE_SLICE);
    }
}
