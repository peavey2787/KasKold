//! The GC0308 sensor: 8-bit luma, 320 pixels a line, delivered bottom row first.
//!
//! With nothing in front of it the sensor shows live noise. A test can hold up
//! pictures instead, which then play in a loop, as an animated QR on a phone would.
use std::sync::Mutex;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;
const NOISE_SEED: u32 = 0x2F6E_2B1D;

struct Scene {
    pictures: Vec<Vec<u8>>,
    next: usize,
    noise: u32,
}

static SCENE: Mutex<Scene> = Mutex::new(Scene {
    pictures: Vec::new(),
    next: 0,
    noise: NOISE_SEED,
});

/// Holds top-down `WIDTH * HEIGHT` luma pictures in front of the lens.
pub fn show(pictures: Vec<Vec<u8>>) {
    assert!(
        pictures
            .iter()
            .all(|picture| picture.len() == WIDTH * HEIGHT),
        "picture size"
    );
    let mut scene = SCENE.lock().expect("camera model poisoned");
    scene.pictures = pictures;
    scene.next = 0;
}

pub fn capture(frame: &mut [u8]) {
    let mut scene = SCENE.lock().expect("camera model poisoned");
    if scene.pictures.is_empty() {
        scene.fill_with_noise(frame);
        return;
    }
    let index = scene.next % scene.pictures.len();
    scene.next = scene.next.wrapping_add(1);
    copy_upside_down(&scene.pictures[index], frame);
}

impl Scene {
    fn fill_with_noise(&mut self, frame: &mut [u8]) {
        for pixel in frame {
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            *pixel = self.noise.to_le_bytes()[0];
        }
    }
}

fn copy_upside_down(picture: &[u8], frame: &mut [u8]) {
    for (row, line) in picture.as_chunks::<WIDTH>().0.iter().rev().enumerate() {
        let start = row * WIDTH;
        if let Some(target) = frame.get_mut(start..start + WIDTH) {
            target.copy_from_slice(line);
        }
    }
}
