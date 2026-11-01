use crate::peripherals::PSRAM;

const PSRAM_BYTES: usize = 8 * 1024 * 1024;

/// A dedicated region gives the firmware's provenance checks a real address range.
pub fn psram_raw_parts(_psram: &PSRAM<'_>) -> (*mut u8, usize) {
    let region = Box::leak(vec![0u8; PSRAM_BYTES].into_boxed_slice());
    (region.as_mut_ptr(), region.len())
}
