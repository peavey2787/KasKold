//! Bench-only probe of the largest PSRAM allocation the heap can serve.

use core::alloc::{GlobalAlloc, Layout};

use esp_alloc::{MemoryCapability, HEAP};

pub(crate) fn probe_largest_allocatable(cap: usize, granularity: usize) -> usize {
    if granularity == 0 {
        return 0;
    }
    let upper = super::psram::free_bytes().min(cap) / granularity;
    let mut low = 0usize;
    let mut high = upper;
    while low < high {
        let mid = (low + high).div_ceil(2);
        if can_allocate(mid * granularity) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low * granularity
}

fn can_allocate(size: usize) -> bool {
    if size == 0 {
        return true;
    }
    let Ok(layout) = Layout::from_size_align(size, 8) else {
        return false;
    };
    // SAFETY: `layout` was checked non-zero-sized above.
    let ptr = unsafe { HEAP.alloc_caps(MemoryCapability::External.into(), layout) };
    if ptr.is_null() {
        return false;
    }
    let provenance_ok = super::psram::region().is_ok_and(|psram| psram.contains(ptr as usize, size));
    // SAFETY: `ptr` was just returned by `HEAP.alloc_caps` with this same `layout` and is not yet shared.
    unsafe { HEAP.dealloc(ptr, layout) };
    provenance_ok
}
