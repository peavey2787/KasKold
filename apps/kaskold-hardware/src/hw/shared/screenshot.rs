// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Development-only shadow framebuffer and its UART mirror.
//!
//! The framebuffer is allocated once and accessed only through scoped
//! callbacks. No mutable reference or raw pointer can escape the storage
//! facade, which keeps the display and mirror paths from creating long-lived
//! aliases. `tools/dev/firmware_mirror.rs` reads the mirror's hex rows.

use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

pub(crate) const W: usize = 320;
pub(crate) const H: usize = 240;
const BUF_SIZE: usize = W * H * 2;

static FRAMEBUFFER: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static FRAMEBUFFER_LOCK: AtomicBool = AtomicBool::new(false);

struct FramebufferGuard;

impl FramebufferGuard {
    fn acquire() -> Self {
        while FRAMEBUFFER_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        Self
    }
}

impl Drop for FramebufferGuard {
    fn drop(&mut self) {
        FRAMEBUFFER_LOCK.store(false, Ordering::Release);
    }
}

fn ensure_framebuffer() -> *mut u8 {
    let existing = FRAMEBUFFER.load(Ordering::Acquire);
    if !existing.is_null() {
        return existing;
    }

    let Ok(layout) = alloc::alloc::Layout::from_size_align(BUF_SIZE, 4) else {
        return ptr::null_mut();
    };
    // SAFETY: `layout` has the non-zero size `BUF_SIZE`.
    let candidate = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if candidate.is_null() {
        return ptr::null_mut();
    }

    match FRAMEBUFFER.compare_exchange(ptr::null_mut(), candidate, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => candidate,
        Err(winner) => {
            // SAFETY: `candidate` came from `alloc_zeroed` with this `layout` and lost the race, so nothing else holds it.
            unsafe { alloc::alloc::dealloc(candidate, layout) };
            winner
        }
    }
}

/// Access the shadow framebuffer for the duration of one callback.
pub(crate) fn with_framebuffer_mut<R>(f: impl FnOnce(&mut [u8]) -> R) -> Option<R> {
    let pointer = ensure_framebuffer();
    if pointer.is_null() {
        return None;
    }
    let _guard = FramebufferGuard::acquire();
    // SAFETY: `pointer` is the never-freed `BUF_SIZE` allocation published in FRAMEBUFFER, and the held
    // lock makes this the only view of it until the callback returns.
    let framebuffer = unsafe { core::slice::from_raw_parts_mut(pointer, BUF_SIZE) };
    Some(f(framebuffer))
}

#[cfg(feature = "mirror")]
mod mirror_state {
    use core::sync::atomic::{AtomicBool, AtomicU16, Ordering};

    use super::{FramebufferGuard, BUF_SIZE, FRAMEBUFFER, H, W};

    const ROWS_PER_CHUNK: usize = 4;
    static ROW: AtomicU16 = AtomicU16::new(0);
    static PENDING: AtomicBool = AtomicBool::new(false);

    fn with_framebuffer<R>(f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        let pointer = FRAMEBUFFER.load(Ordering::Acquire);
        if pointer.is_null() {
            return None;
        }
        let _guard = FramebufferGuard::acquire();
        // SAFETY: `pointer` is the never-freed `BUF_SIZE` allocation published in FRAMEBUFFER, and the held
        // lock keeps writers out until the callback returns.
        let framebuffer = unsafe { core::slice::from_raw_parts(pointer, BUF_SIZE) };
        Some(f(framebuffer))
    }

    fn print_rows(framebuffer: &[u8], start_row: usize, end_row: usize) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for row in start_row..end_row {
            let row_start = row * W * 2;
            for chunk in framebuffer[row_start..row_start + W * 2].chunks(64) {
                let mut hex = [0u8; 128];
                for (index, byte) in chunk.iter().copied().enumerate() {
                    hex[index * 2] = HEX[usize::from(byte >> 4)];
                    hex[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
                }
                if let Ok(text) = core::str::from_utf8(&hex[..chunk.len() * 2]) {
                    esp_println::print!("{}", text);
                }
            }
            esp_println::println!();
        }
    }

    pub fn request_frame() {
        PENDING.store(true, Ordering::Release);
        ROW.store(0, Ordering::Release);
    }

    /// Print the next rows of a requested frame; `true` once no frame is in flight.
    pub fn pump_rows() -> bool {
        if FRAMEBUFFER.load(Ordering::Acquire).is_null() {
            return true;
        }

        let mut row = ROW.load(Ordering::Acquire);
        if PENDING.swap(false, Ordering::AcqRel) && row == 0 {
            row = 1;
            ROW.store(row, Ordering::Release);
            esp_println::println!("SCREENSHOT_BEGIN {} {}", W, H);
        }
        if row == 0 {
            return true;
        }

        let start = usize::from(row - 1);
        let end = (start + ROWS_PER_CHUNK).min(H);
        let _ = with_framebuffer(|framebuffer| print_rows(framebuffer, start, end));

        if end >= H {
            esp_println::println!("SCREENSHOT_END");
            ROW.store(0, Ordering::Release);
            true
        } else {
            // `end` is below `H`, so the next row always fits; 0 would only end the frame.
            ROW.store(u16::try_from(end + 1).unwrap_or(0), Ordering::Release);
            false
        }
    }
}

#[cfg(feature = "mirror")]
pub use mirror_state::{pump_rows, request_frame};
