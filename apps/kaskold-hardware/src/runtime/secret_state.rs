//! Internal-SRAM ownership for the long-lived secret-bearing application root.

use core::sync::atomic::{compiler_fence, AtomicPtr, Ordering};

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

/// Wipe the root on a fault, once: first through `zeroize`, which follows the
/// root into the heap buffers it owns, then every byte of the root itself, so
/// a field no zeroizer knows about cannot outlive the fault.
///
/// Only a panic or CPU exception calls this, and neither ever returns to the
/// code that held the root, so the abandoned borrow is never used again. A
/// fault raised while wiping finds the root already taken and just resets.
pub(crate) fn wipe_for_fault(zeroize: impl FnOnce(&mut AppData)) {
    let root = FAULT_ROOT.swap(core::ptr::null_mut(), Ordering::AcqRel);
    // SAFETY: `root` is null or the `'static` root stored by `initialize`. The
    // swap hands it out at most once, and its previous holder never resumes.
    let Some(data) = (unsafe { root.as_mut() }) else {
        return;
    };
    zeroize(data);
    let bytes = root.cast::<u8>();
    for offset in 0..core::mem::size_of::<AppData>() {
        // SAFETY: `root` points at a live, exclusively held `AppData`, so every
        // byte below its size is writable. The reference above is no longer
        // used, and the chip resets before anything reads the root again.
        unsafe { bytes.add(offset).write_volatile(0) };
    }
    compiler_fence(Ordering::SeqCst);
}

/// The virtual-board harness reads the root of a parked firmware thread.
#[cfg(not(target_arch = "xtensa"))]
pub(crate) fn peek_for_harness() -> *const AppData {
    FAULT_ROOT.load(Ordering::Acquire)
}
