//! FT6336U: the registers the driver reads, fed by what the harness presses.
use std::sync::Mutex;

const DISPLAY_MAX_X: u16 = 319;
const DISPLAY_MAX_Y: u16 = 239;
const EVENT_PRESS_DOWN: u8 = 0b00 << 6;
const EVENT_CONTACT: u8 = 0b10 << 6;

struct Finger {
    x: u16,
    y: u16,
    reported: bool,
}

static FINGER: Mutex<Option<Finger>> = Mutex::new(None);

/// The panel is mounted rotated, so raw coordinates are the mirror of display ones.
pub fn press(x: u16, y: u16) {
    *FINGER.lock().expect("touch model poisoned") =
        Some(Finger { x: DISPLAY_MAX_X - x, y: DISPLAY_MAX_Y - y, reported: false });
}

pub fn release() {
    *FINGER.lock().expect("touch model poisoned") = None;
}

/// TD_STATUS followed by the P1 position registers. The chip flags the first
/// sample of a touch as a press and every later one as contact.
pub fn read_registers(output: &mut [u8]) {
    let mut registers = [0u8; 5];
    if let Some(finger) = FINGER.lock().expect("touch model poisoned").as_mut() {
        let event = if finger.reported { EVENT_CONTACT } else { EVENT_PRESS_DOWN };
        finger.reported = true;
        registers[0] = 1;
        registers[1] = event | (finger.x >> 8) as u8;
        registers[2] = finger.x as u8;
        registers[3] = (finger.y >> 8) as u8;
        registers[4] = finger.y as u8;
    }
    let count = output.len().min(registers.len());
    output[..count].copy_from_slice(&registers[..count]);
}
