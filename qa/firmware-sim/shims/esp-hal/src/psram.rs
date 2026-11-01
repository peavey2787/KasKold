use crate::peripherals::PSRAM;
use std::alloc::{alloc_zeroed, Layout};

const PSRAM_BYTES: usize = 8 * 1024 * 1024;
/// External memory is mapped in whole 64 KiB MMU pages, so the chip's region starts on one.
const MMU_PAGE_BYTES: usize = 64 * 1024;

/// A dedicated region gives the firmware's provenance checks a real address range.
pub fn psram_raw_parts(_psram: &PSRAM<'_>) -> (*mut u8, usize) {
    let layout = Layout::from_size_align(PSRAM_BYTES, MMU_PAGE_BYTES).expect("PSRAM layout");
    // SAFETY: the layout has a non-zero size, and the region is never freed, like the chip's.
    let region = unsafe { alloc_zeroed(layout) };
    assert!(!region.is_null(), "host memory for virtual PSRAM");
    (region, PSRAM_BYTES)
}
