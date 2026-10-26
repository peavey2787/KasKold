//! Fail-closed fault handling.
//!
//! A panic or CPU exception can strike while wallet keys, seeds or signing
//! state are live. Halting would leave them in RAM for as long as the device
//! stays powered, so every fault wipes the volatile secret surfaces through
//! the same coordinator duress uses, then resets the chip. esp-hal turns CPU
//! exceptions into panics, so this one handler covers both.

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    match info.location() {
        Some(location) => crate::log!("KASKOLD_PANIC at {}:{}", location.file(), location.line()),
        None => crate::log!("KASKOLD_PANIC"),
    }
    wipe_secret_state();
    esp_hal::system::software_reset()
}

#[cfg(not(feature = "qemu"))]
fn wipe_secret_state() {
    if let Some(ad) = crate::runtime::secret_state::take_for_fault() {
        crate::services::device_wipe::zeroize_volatile(ad);
    }
}

#[cfg(feature = "qemu")]
fn wipe_secret_state() {}
