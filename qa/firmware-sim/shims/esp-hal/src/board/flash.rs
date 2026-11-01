use std::sync::Mutex;

const FLASH_BYTES: usize = 16 * 1024 * 1024;
const SECTOR_BYTES: usize = 4096;

static FLASH: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn with_flash<T>(operation: impl FnOnce(&mut Vec<u8>) -> T) -> T {
    let mut flash = FLASH.lock().expect("flash model poisoned");
    if flash.is_empty() {
        flash.resize(FLASH_BYTES, 0xFF);
    }
    operation(&mut flash)
}

pub fn image() -> Vec<u8> {
    with_flash(|flash| flash.clone())
}

pub fn restore(image: &[u8]) {
    with_flash(|flash| flash.copy_from_slice(image));
}

pub fn read(address: u32, out: &mut [u8]) -> i32 {
    with_flash(|flash| {
        let start = address as usize;
        out.copy_from_slice(&flash[start..start + out.len()]);
        0
    })
}

pub fn erase_sector(sector: u32) -> i32 {
    with_flash(|flash| {
        let start = sector as usize * SECTOR_BYTES;
        flash[start..start + SECTOR_BYTES].fill(0xFF);
        0
    })
}

/// NOR semantics: a write can only clear bits, so a missed erase corrupts as on the part.
pub fn write(address: u32, input: &[u8]) -> i32 {
    with_flash(|flash| {
        let start = address as usize;
        for (cell, byte) in flash[start..start + input.len()].iter_mut().zip(input) {
            *cell &= *byte;
        }
        0
    })
}
