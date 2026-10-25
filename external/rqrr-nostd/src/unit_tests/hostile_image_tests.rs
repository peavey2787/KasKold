//! Hostile camera frames must be rejected, never abort the decoder.
//!
//! The signer feeds every camera frame to this crate with overflow checks on,
//! so any panic here is a camera-triggered denial of service.

extern crate std;

use alloc::vec::Vec;

use crate::geometry::{clamp_coordinate, MAX_COORDINATE};
use crate::PreparedImage;

/// Frames found by fuzzing. Each made a perspective fitted to stray capstones
/// map grid cells beyond `i32`, which used to trip an `assert!` in
/// `Perspective::map` (via timing-pattern fitness, capstone-ring fitness and
/// grid seeding respectively).
const HOSTILE_FRAMES: [(&str, &[u8], (usize, usize)); 4] = [
    (
        "perspective_overflow",
        include_bytes!("fixtures/perspective_overflow.pgm"),
        (143, 77),
    ),
    (
        "timing_ring_overflow",
        include_bytes!("fixtures/timing_ring_overflow.pgm"),
        (93, 43),
    ),
    (
        "capstone_ring_overflow",
        include_bytes!("fixtures/capstone_ring_overflow.pgm"),
        (137, 67),
    ),
    (
        "grid_seed_overflow",
        include_bytes!("fixtures/grid_seed_overflow.pgm"),
        (142, 119),
    ),
];

fn decode_all(width: usize, height: usize, pixels: &[u8]) -> usize {
    let mut image =
        PreparedImage::prepare_from_greyscale(width, height, |x, y| pixels[y * width + x]);
    image
        .detect_grids()
        .into_iter()
        .filter(|grid| grid.decode_to(Vec::new()).is_ok())
        .count()
}

fn parse_pgm(data: &[u8]) -> (usize, usize, &[u8]) {
    let mut fields = Vec::new();
    let mut position = 0;
    while fields.len() < 4 {
        while data[position].is_ascii_whitespace() {
            position += 1;
        }
        let start = position;
        while !data[position].is_ascii_whitespace() {
            position += 1;
        }
        fields.push(core::str::from_utf8(&data[start..position]).unwrap());
    }
    assert_eq!(fields[0], "P5");
    let width: usize = fields[1].parse().unwrap();
    let height: usize = fields[2].parse().unwrap();
    assert_eq!(fields[3], "255");
    let pixels = &data[position + 1..];
    assert_eq!(pixels.len(), width * height);
    (width, height, pixels)
}

#[test]
fn fuzzed_overflow_frames_are_rejected_without_panicking() {
    for (name, data, dimensions) in HOSTILE_FRAMES {
        let (width, height, pixels) = parse_pgm(data);
        assert_eq!((width, height), dimensions, "{name}");
        assert_eq!(decode_all(width, height, pixels), 0, "{name}");
    }
}

#[test]
fn coordinates_are_rounded_and_clamped_to_the_frame_bound() {
    assert_eq!(clamp_coordinate(2.5), 3);
    assert_eq!(clamp_coordinate(-2.5), -3);
    assert_eq!(clamp_coordinate(1.0e300), MAX_COORDINATE);
    assert_eq!(clamp_coordinate(-1.0e300), -MAX_COORDINATE);
    assert_eq!(clamp_coordinate(f64::NAN), MAX_COORDINATE);
    assert_eq!(clamp_coordinate(f64::INFINITY), MAX_COORDINATE);
}
