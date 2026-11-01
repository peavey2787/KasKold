//! The steps journeys share, written as what a person does with the device.
use crate::harness::Device;
use kaskold_hardware_core::input::keyboard::{KeyAction, KeyboardMode};

const WALLET_NAME: &str = "Main";
const ADDRESS_LINE_CHARS: usize = 25;
/// A tap anywhere that is not a control moves the recovery words on.
const NEXT_WORD_TAP: (u16, u16) = (160, 120);
const NEXT_PAGE_TAP: (u16, u16) = (300, 135);
const WARNING_ACCEPT_TAP: (u16, u16) = (238, 212);

/// Types a recovery phrase on a new device and answers the passphrase question.
pub(super) fn restore_words(device: &Device, words: &[&str], passphrase_choice: &str) {
    device.tap("Restore Wallet");
    device.tap("Words");
    type_words(device, words);
    device.tap(&format!("Restore {}-Word Wallet", words.len()));
    device.tap(passphrase_choice);
}

fn type_words(device: &Device, words: &[&str]) {
    for word in words {
        device.type_letters(word);
        device.press_key(KeyAction::Ok, KeyboardMode::Alpha, 0);
    }
}

pub(super) fn press_full_ok(device: &Device) {
    device.press_key(KeyAction::Ok, KeyboardMode::Full, 0);
}

pub(super) fn name_wallet(device: &Device) {
    device.type_text(WALLET_NAME);
    press_full_ok(device);
}

/// Names the wallet and protects it with a new PIN.
pub(super) fn save_with_pin(device: &Device, pin: &str) {
    name_wallet(device);
    protect_with_pin(device, pin);
}

/// Keeps an already named wallet on the device behind a new PIN.
pub(super) fn protect_with_pin(device: &Device, pin: &str) {
    device.tap("Save Securely on Device");
    device.tap("Use PIN / Password");
    device.tap("Use PIN");
    device.enter_pin(pin);
    device.enter_pin(pin);
    device.expect_text("Scan QR");
}

pub(super) fn unlock_with_pin(device: &Device, pin: &str) {
    device.expect_text("UNLOCK PIN");
    device.enter_pin(pin);
    device.expect_text("Scan QR");
}

pub(super) fn enter_password(device: &Device, password: &str) {
    device.type_text(password);
    press_full_ok(device);
}

/// Turns to the next page of a menu.
pub(super) fn next_menu_page(device: &Device) {
    device.tap_at(NEXT_PAGE_TAP.0, NEXT_PAGE_TAP.1);
}

/// Answers yes on a warning that asks twice before anything irreversible.
pub(super) fn accept_warning(device: &Device) {
    device.tap_at(WARNING_ACCEPT_TAP.0, WARNING_ACCEPT_TAP.1);
}

pub(super) fn open_receive(device: &Device) {
    device.tap("Wallet");
    device.tap("Receive");
}

/// Waits out the derivation, then reads the whole address off the screen.
pub(super) fn expect_address(device: &Device, address: &str) {
    for line in address.as_bytes().chunks(ADDRESS_LINE_CHARS) {
        device.wait_for_text(core::str::from_utf8(line).expect("ascii address"));
    }
}

/// Reads each recovery word the way a person copies them onto paper.
pub(super) fn write_down_words(device: &Device, count: usize) -> Vec<String> {
    (1..=count)
        .map(|number| {
            device.expect_text(&format!("WORD {number}/{count}"));
            let word = device
                .screen()
                .read_recovery_word()
                .expect("a recovery word on screen");
            device.tap_at(NEXT_WORD_TAP.0, NEXT_WORD_TAP.1);
            word
        })
        .collect()
}
