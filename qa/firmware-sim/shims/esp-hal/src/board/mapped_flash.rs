//! Puts readable memory at the fixed addresses the firmware maps flash to, and
//! publishes the image identity the firmware's verifier reads.
use sha2::{Digest, Sha256};

const FIRMWARE_SEGMENTS: [(usize, usize); 2] = [(0x3C00_0000, 0x20_0000), (0x4200_0000, 0x20_0000)];
const CODE_SEGMENT: usize = 0x4206_0020;
/// Both buses read the same flash through one MMU table, so one flash byte has two addresses.
const CODE_SEGMENT_ON_DATA_BUS: usize = 0x3C06_0020;

pub static mut EXPECTED_FIRMWARE_HASH: [u8; 32] = [0; 32];
pub static mut FIRMWARE_SIGNATURE: [u8; 64] = [0; 64];
pub static FIRMWARE_SIGNED: bool = true;
pub static FIRMWARE_SIZE: usize = 426_204;
pub static FIRMWARE_IADDR: u32 = CODE_SEGMENT as u32;

#[cfg(windows)]
mod platform {
    const MEM_COMMIT_RESERVE: u32 = 0x3000;
    const PAGE_READWRITE: u32 = 0x04;

    extern "system" {
        fn VirtualAlloc(address: *mut u8, size: usize, kind: u32, protect: u32) -> *mut u8;
    }

    pub fn map(address: usize, size: usize) -> *mut u8 {
        // SAFETY: a fixed request either returns that free range or null, which the caller checks.
        unsafe { VirtualAlloc(address as *mut u8, size, MEM_COMMIT_RESERVE, PAGE_READWRITE) }
    }
}

#[cfg(unix)]
mod platform {
    const PROT_READ_WRITE: i32 = 0x3;
    const MAP_PRIVATE_ANON_FIXED_NOREPLACE: i32 = 0x02 | 0x20 | 0x10_0000;

    extern "C" {
        fn mmap(
            address: *mut u8,
            size: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut u8;
    }

    pub fn map(address: usize, size: usize) -> *mut u8 {
        // SAFETY: NOREPLACE refuses to overlap an existing mapping; the caller checks the result.
        unsafe {
            mmap(
                address as *mut u8,
                size,
                PROT_READ_WRITE,
                MAP_PRIVATE_ANON_FIXED_NOREPLACE,
                -1,
                0,
            )
        }
    }
}

pub fn install() {
    for (address, size) in FIRMWARE_SEGMENTS {
        let mapped = platform::map(address, size);
        assert_eq!(
            mapped as usize, address,
            "fixed firmware mapping unavailable"
        );
        // SAFETY: the range was just mapped read-write for exactly `size` bytes.
        let bytes = unsafe { core::slice::from_raw_parts_mut(mapped, size) };
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8)
                .wrapping_mul(31)
                .wrapping_add((index >> 8) as u8);
        }
    }
    // SAFETY: the code segment lies inside the mapping above, and no firmware runs yet.
    unsafe {
        let code = core::slice::from_raw_parts(CODE_SEGMENT as *const u8, FIRMWARE_SIZE);
        EXPECTED_FIRMWARE_HASH = Sha256::digest(code).into();
    }
}

/// Changes one executable byte, as an attacker with flash access would.
pub fn tamper() {
    // SAFETY: both views of the code segment were mapped read-write by `install`.
    unsafe {
        *(CODE_SEGMENT as *mut u8) ^= 0x01;
        *(CODE_SEGMENT_ON_DATA_BUS as *mut u8) ^= 0x01;
    }
}

/// The hash of the mapped image, for whoever signs it.
pub fn image_hash() -> [u8; 32] {
    // SAFETY: written once by `install`, before any firmware thread exists.
    unsafe { EXPECTED_FIRMWARE_HASH }
}

/// Records the release signature over the mapped image.
pub fn set_signature(signature: [u8; 64]) {
    // SAFETY: called once before any firmware thread exists.
    unsafe { FIRMWARE_SIGNATURE = signature };
}
