#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(65_536)];
    let key = b"KasKold stego parser fuzz";
    let _ = kaskold_hardware_core::backup::stego_picture::capacity_bits(data, key);
    let mut output = [0u8; 1024];
    let _ = kaskold_hardware_core::backup::stego_picture::extract(data, key, &mut output);
});
