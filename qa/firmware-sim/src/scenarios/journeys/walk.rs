//! The steps journeys share, written as what a person does with the device.
use super::known;
use crate::harness::Device;
use kaskold_hardware_core::input::keyboard::{KeyAction, KeyboardMode};
use std::time::Duration;

const WALLET_NAME: &str = "Main";
const ADDRESS_LINE_CHARS: usize = 25;
const REVIEW_ADDRESS_LINES: usize = 3;
const REVIEW_HIGHLIGHT_CHARS: usize = 8;
/// A tap anywhere that is not a control moves the recovery words on.
const NEXT_WORD_TAP: (u16, u16) = (160, 120);
const NEXT_PAGE_TAP: (u16, u16) = (300, 135);
const PREVIOUS_PAGE_TAP: (u16, u16) = (18, 135);
const KPUB_PREFIX: &[u8] = b"kpub";
const WARNING_ACCEPT_TAP: (u16, u16) = (238, 212);
const HOME_TAP: (u16, u16) = (295, 15);
const ADD_WALLET_ROW: (u16, u16) = (160, 70);
const BACK_TAP: (u16, u16) = (20, 15);
const FORMAT_HOLD: Duration = Duration::from_secs(5);

/// Restores the wallet whose addresses `known` lists, behind its PIN.
pub(super) fn restore_the_known_wallet() {
    let device = Device::boot();
    restore_words(&device, known::ABOUT_WORDS, "No Passphrase");
    save_with_pin(&device, known::PIN);
}

/// Erases the device through its own menus, from the unlock screen.
pub(super) fn factory_reset() {
    let device = Device::boot();
    unlock_with_pin(&device, known::PIN);
    device.tap("Settings");
    next_menu_page(&device);
    device.tap("Advanced");
    device.tap("Factory Reset");
    accept_warning(&device);
    accept_warning(&device);
    device.wait_until_stopped_or_text("WELCOME");
}

/// Goes back to the main menu from wherever the home icon is shown.
pub(super) fn go_home(device: &Device) {
    device.tap_at(HOME_TAP.0, HOME_TAP.1);
}

/// The kpub text the device saved on the card.
pub(super) fn kpub_on_the_card() -> String {
    let card = esp_hal::board::sd::image();
    let start = card
        .windows(KPUB_PREFIX.len())
        .position(|window| window == KPUB_PREFIX)
        .expect("the kpub reached the card");
    card[start..]
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b':')
        .map(|&byte| char::from(byte))
        .collect()
}

/// Goes back one screen.
pub(super) fn go_back(device: &Device) {
    device.tap_at(BACK_TAP.0, BACK_TAP.1);
}

/// Prepares the inserted card through Settings and returns to the main menu.
pub(super) fn format_card(device: &Device) {
    device.tap("Settings");
    device.tap("Storage");
    device.tap("Format FAT32");
    device.hold("HOLD 4s TO FORMAT", FORMAT_HOLD);
    device.wait_for_text("Format FAT32");
    go_home(device);
}

/// Opens the second page of the wallet's Advanced menu from the main menu.
pub(super) fn open_wallet_tools(device: &Device) {
    device.tap("Wallet");
    next_menu_page(device);
    device.tap("Advanced");
    next_menu_page(device);
}

/// True when a person looking through raw storage would find `needle`.
pub(super) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Types a recovery phrase on a new device and answers the passphrase question.
pub(super) fn restore_words(device: &Device, words: &[&str], passphrase_choice: &str) {
    device.tap("Restore Wallet");
    device.tap("Words");
    type_words(device, words);
    device.tap(&format!("Restore {}-Word Wallet", words.len()));
    device.tap(passphrase_choice);
}

pub(super) fn type_words(device: &Device, words: &[&str]) {
    for word in words {
        device.type_letters(word);
        device.press_key(KeyAction::Ok, KeyboardMode::Alpha, 0);
    }
}

pub(super) fn press_full_ok(device: &Device) {
    device.press_key(KeyAction::Ok, KeyboardMode::Full, 0);
}

pub(super) fn name_wallet(device: &Device) {
    name_wallet_as(device, WALLET_NAME);
}

pub(super) fn name_wallet_as(device: &Device, name: &str) {
    device.type_text(name);
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

/// From the home menu to the list of wallets.
pub(super) fn open_wallet_list(device: &Device) {
    device.tap("Wallet");
    next_menu_page(device);
    device.tap("Switch / Add Wallet");
}

/// From the wallet list to the choice of how a new wallet arrives.
pub(super) fn tap_add_wallet(device: &Device) {
    device.tap_at(ADD_WALLET_ROW.0, ADD_WALLET_ROW.1);
}

pub(super) fn open_add_wallet(device: &Device) {
    open_wallet_list(device);
    tap_add_wallet(device);
}

/// Turns to the next page of a menu.
pub(super) fn next_menu_page(device: &Device) {
    device.tap_at(NEXT_PAGE_TAP.0, NEXT_PAGE_TAP.1);
}

/// Answers yes on a warning that asks twice before anything irreversible.
pub(super) fn accept_warning(device: &Device) {
    device.tap_at(WARNING_ACCEPT_TAP.0, WARNING_ACCEPT_TAP.1);
}

pub(super) fn open_wallet_menu(device: &Device) {
    device.tap("Wallet");
    // The menu reopens on the page it was left on.
    if device.screen().find("Receive").is_none() {
        device.tap_at(PREVIOUS_PAGE_TAP.0, PREVIOUS_PAGE_TAP.1);
    }
}

pub(super) fn open_receive(device: &Device) {
    open_wallet_menu(device);
    device.tap("Receive");
}

/// Waits out the derivation, then reads the whole address off the screen.
pub(super) fn expect_address(device: &Device, address: &str) {
    for line in address.as_bytes().chunks(ADDRESS_LINE_CHARS) {
        device.wait_for_text(core::str::from_utf8(line).expect("ascii address"));
    }
}

/// Reads a whole address off a transaction review. It spans three lines with the
/// start and end of the address highlighted, and each coloured run is its own text.
pub(super) fn expect_review_address(device: &Device, address: &str) {
    let line = address.len().div_ceil(REVIEW_ADDRESS_LINES);
    let body = address.find(':').expect("an address prefix") + 1;
    let end = address.len() - REVIEW_HIGHLIGHT_CHARS;
    let mut cuts = [
        0,
        body,
        body + REVIEW_HIGHLIGHT_CHARS,
        line,
        2 * line,
        end,
        address.len(),
    ];
    cuts.sort_unstable();
    for run in cuts.windows(2) {
        device.wait_for_text(&address[run[0]..run[1]]);
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
