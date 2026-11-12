//! Types on the on-screen keyboard: finds each key where the firmware's hit test says it is.
use kaskold_hardware_core::input::keyboard::{hit_test, KeyAction, KeyboardMode};

use super::Device;
use crate::ui::screens::device::persistence::{pin_pad_action, PinPadAction};

const WIDTH: u16 = 320;
const HEIGHT: u16 = 240;
const FULL_KEYBOARD_PAGES: u8 = 4;

impl Device {
    /// Types lower-case letters on the recovery-word keyboard.
    pub(crate) fn type_letters(&self, text: &str) {
        for character in text.bytes() {
            self.press_key(KeyAction::Char(character), KeyboardMode::Alpha, 0);
        }
    }

    /// Enters a PIN on the keypad and submits it.
    pub(crate) fn enter_pin(&self, pin: &str) {
        for digit in pin.bytes() {
            self.press_pin_pad(PinPadAction::Digit(digit));
        }
        self.press_pin_pad(PinPadAction::Submit);
    }

    /// Enters digits on the numeric keyboard and submits them.
    pub(crate) fn enter_on_number_row(&self, digits: &str) {
        for digit in digits.bytes() {
            self.press_key(KeyAction::Char(digit), KeyboardMode::Numeric, 0);
        }
        self.press_key(KeyAction::Ok, KeyboardMode::Numeric, 0);
    }

    fn press_pin_pad(&self, action: PinPadAction) {
        let (x, y) =
            centre_of(screen_points().filter(|(x, y)| pin_pad_action(*x, *y) == Some(action)));
        self.tap_at(x, y);
    }

    /// Types any text on the full keyboard, turning its pages as a person would.
    pub(crate) fn type_text(&self, text: &str) {
        let mut page = 0;
        for character in text.bytes() {
            if character == b' ' {
                self.press_key(KeyAction::Space, KeyboardMode::Full, page);
                continue;
            }
            let wanted = page_with(character);
            while page != wanted {
                self.press_key(KeyAction::Page, KeyboardMode::Full, page);
                page = (page + 1) % FULL_KEYBOARD_PAGES;
            }
            self.press_key(KeyAction::Char(character), KeyboardMode::Full, page);
        }
    }

    /// Taps the middle of the area that performs `action`.
    pub(crate) fn press_key(&self, action: KeyAction, mode: KeyboardMode, page: u8) {
        let (x, y) = key_centre(action, mode, page);
        self.tap_at(x, y);
    }
}

fn page_with(character: u8) -> u8 {
    (0..FULL_KEYBOARD_PAGES)
        .find(|page| {
            let action = KeyAction::Char(character);
            area(action, KeyboardMode::Full, *page).next().is_some()
        })
        .unwrap_or_else(|| panic!("no keyboard page has {:?}", character as char))
}

fn screen_points() -> impl Iterator<Item = (u16, u16)> {
    (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
}

fn area(action: KeyAction, mode: KeyboardMode, page: u8) -> impl Iterator<Item = (u16, u16)> {
    screen_points().filter(move |(x, y)| hit_test(*x, *y, mode, page) == action)
}

fn key_centre(action: KeyAction, mode: KeyboardMode, page: u8) -> (u16, u16) {
    centre_of(area(action, mode, page))
}

fn centre_of(points: impl Iterator<Item = (u16, u16)>) -> (u16, u16) {
    let (mut count, mut sum_x, mut sum_y) = (0u32, 0u32, 0u32);
    for (x, y) in points {
        count += 1;
        sum_x += u32::from(x);
        sum_y += u32::from(y);
    }
    assert!(count > 0, "no such key on this screen");
    (
        u16::try_from(sum_x / count).unwrap(),
        u16::try_from(sum_y / count).unwrap(),
    )
}
