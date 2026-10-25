#![no_main]

use libfuzzer_sys::fuzz_target;

// Camera frames reach the signer's QR decoder directly from whatever is held
// in front of the lens. Byte 0/1 choose the frame size (16..=271 pixels a
// side, covering the device's scan crops); the rest are greyscale pixels,
// repeated to fill the frame so short inputs still drive full-frame geometry.
fuzz_target!(|data: &[u8]| {
    let Some((&width, rest)) = data.split_first() else {
        return;
    };
    let Some((&height, pixels)) = rest.split_first() else {
        return;
    };
    if pixels.is_empty() {
        return;
    }
    let width = 16 + usize::from(width);
    let height = 16 + usize::from(height);
    let mut image = rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| {
        pixels[(y * width + x) % pixels.len()]
    });
    for grid in image.detect_grids() {
        let mut decoded = Vec::new();
        let _ = grid.decode_to(&mut decoded);
    }
});
