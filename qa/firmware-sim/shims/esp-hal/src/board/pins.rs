use std::sync::atomic::{AtomicU64, Ordering};

static LEVELS: AtomicU64 = AtomicU64::new(u64::MAX);

pub fn set(pin: u8, high: bool) {
    let mask = 1u64 << pin;
    if high {
        LEVELS.fetch_or(mask, Ordering::Relaxed);
    } else {
        LEVELS.fetch_and(!mask, Ordering::Relaxed);
    }
}

pub fn is_high(pin: u8) -> bool {
    LEVELS.load(Ordering::Relaxed) & (1u64 << pin) != 0
}
