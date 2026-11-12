pub mod spiflash {
    use crate::board::flash;

    /// # Safety
    /// `data` must be valid for `len` bytes.
    pub unsafe fn esp_rom_spiflash_read(address: u32, data: *const u32, len: u32) -> i32 {
        // SAFETY: the caller upholds this function's contract that `data` is valid for `len` bytes.
        let out = unsafe { core::slice::from_raw_parts_mut(data as *mut u8, len as usize) };
        flash::read(address, out)
    }

    /// # Safety
    /// Matches the vendor signature; there are no pointer arguments.
    pub unsafe fn esp_rom_spiflash_unlock() -> i32 {
        0
    }

    /// # Safety
    /// Matches the vendor signature; there are no pointer arguments.
    pub unsafe fn esp_rom_spiflash_erase_sector(sector: u32) -> i32 {
        flash::erase_sector(sector)
    }

    /// # Safety
    /// `data` must be valid for `len` bytes.
    pub unsafe fn esp_rom_spiflash_write(address: u32, data: *const u32, len: u32) -> i32 {
        // SAFETY: the caller upholds this function's contract that `data` is valid for `len` bytes.
        let input = unsafe { core::slice::from_raw_parts(data as *const u8, len as usize) };
        flash::write(address, input)
    }
}
