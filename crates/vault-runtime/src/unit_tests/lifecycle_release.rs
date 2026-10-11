//! Dropping a Vault releases its wallets, and the C ABI destroy call frees
//! every allocation the handle owns.

use core::cell::Cell;
use core::mem::MaybeUninit;
use std::alloc::{GlobalAlloc, Layout, System};

use super::MNEMONIC;
use crate::native_ffi::{kaskold_vault_destroy, kaskold_vault_new, kaskold_vault_restore};
use crate::VaultRuntime;

/// Counts the bytes each test thread holds live, so a test can prove that a
/// sequence of calls returns everything it allocated.
struct CountingAllocator;

std::thread_local! {
    static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
}

fn adjust_live_bytes(delta: isize) {
    // During thread teardown the counter may already be gone; nothing reads
    // it then.
    let _ = LIVE_BYTES.try_with(|live| live.set(live.get() + delta));
}

// SAFETY: every call is forwarded unchanged to the system allocator; the
// wrapper only updates a thread-local counter, which never allocates.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        adjust_live_bytes(layout.size() as isize);
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        adjust_live_bytes(-(layout.size() as isize));
        // SAFETY: `ptr` came from `alloc` above with this `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn restored_handle_lifetime() {
    let handle = kaskold_vault_new();
    assert_eq!(
        kaskold_vault_restore(handle, MNEMONIC.as_ptr(), MNEMONIC.len(), b"".as_ptr(), 0),
        0
    );
    kaskold_vault_destroy(handle);
}

#[test]
fn destroying_a_handle_frees_everything_it_allocated() {
    // The first run may initialise process-wide caches that stay allocated.
    restored_handle_lifetime();
    let before = LIVE_BYTES.with(Cell::get);
    restored_handle_lifetime();
    assert_eq!(LIVE_BYTES.with(Cell::get), before);
}

#[test]
fn dropping_a_runtime_releases_its_wallets() {
    let mut runtime = VaultRuntime::new();
    runtime.restore_wallet(MNEMONIC, "").unwrap();
    let mut slot = MaybeUninit::new(runtime);
    // SAFETY: `slot` holds an initialised runtime that is dropped exactly once
    // here and afterwards only has its vector lengths read.
    let (wallets, names) = unsafe {
        slot.as_mut_ptr().drop_in_place();
        let dropped = slot.as_ptr();
        (
            (*core::ptr::addr_of!((*dropped).wallets)).len(),
            (*core::ptr::addr_of!((*dropped).wallet_names)).len(),
        )
    };
    assert_eq!((wallets, names), (0, 0));
}
