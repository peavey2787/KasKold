//! Internal-SRAM ownership for the long-lived secret-bearing application root.

use core::sync::atomic::{AtomicPtr, Ordering};

use super::data::AppData;
use static_cell::StaticCell;

static APP_DATA: StaticCell<AppData> = StaticCell::new();

/// The root, kept for the fault handler to wipe. Taken at most once.
static FAULT_ROOT: AtomicPtr<AppData> = AtomicPtr::new(core::ptr::null_mut());

/// Initialize the application root exactly once in static internal DRAM.
///
/// The root object itself remains in internal DRAM. Bounded bulk transaction
/// storage may be heap-backed in PSRAM, but secret key material and ownership
/// state remain rooted in this static application object.
#[inline(never)]
pub(crate) fn initialize() -> &'static mut AppData {
    match AppData::try_initialize(&APP_DATA) {
        Ok(data) => {
            let data = data.into_mut();
            FAULT_ROOT.store(data, Ordering::Release);
            data
        }
        Err(()) => {
            crate::log!("FATAL: runtime state allocation failed");
            loop {
                core::hint::spin_loop();
            }
        }
    }
}

/// Hand the root to the fault handler, once.
///
/// Only a panic or CPU exception calls this, and neither ever returns to the
/// code that held the root, so the abandoned borrow is never used again. A
/// fault raised while wiping finds the root already taken and just resets.
pub(crate) fn take_for_fault() -> Option<&'static mut AppData> {
    let root = FAULT_ROOT.swap(core::ptr::null_mut(), Ordering::AcqRel);
    // SAFETY: `root` is null or the `'static` root stored by `initialize`. The
    // swap hands it out at most once, and its previous holder never resumes.
    unsafe { root.as_mut() }
}
