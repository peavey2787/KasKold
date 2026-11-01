//! A wallet moved onto the card: it opens only with that card and the PIN, and survives every reboot.
use super::known::{self, PIN};
use super::walk::{self, restore_the_known_wallet};
use crate::harness::Device;
use esp_hal::board::{flash, sd};
use std::time::Duration;

/// The Device-Bound SD card on the Security screen.
const SD_STORAGE_CARD: (u16, u16) = (160, 169);
const WARNING_ENABLE_TAP: (u16, u16) = (238, 212);
const MORE_TRIES_THAN_THE_LIMIT: usize = 12;
const CARD_ERROR_RETRY_DELAY: Duration = Duration::from_secs(5);
/// The two sectors holding saved wallets, at the top of the 16 MiB flash.
const WALLET_RECORDS: core::ops::Range<usize> = 0xFF_E000..0x100_0000;

journey!(
    a_wallet_moved_to_the_card_opens_only_with_the_card:
    restore_the_known_wallet,
    move_the_wallet_to_the_card,
    open_from_the_card,
    refuse_to_open_without_the_card,
    open_when_the_card_returns,
);

fn move_the_wallet_to_the_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    device.tap("Settings");
    device.tap("Security");
    device.wait_for_text("SECURITY");
    device.tap_at(SD_STORAGE_CARD.0, SD_STORAGE_CARD.1);
    device.tap("I BACKED UP MY WORDS");
    device.settle();
    device.tap_at(WARNING_ENABLE_TAP.0, WARNING_ENABLE_TAP.1);
    device.wait_for_text("SECURITY");
    let erased = flash::image()[WALLET_RECORDS]
        .iter()
        .all(|byte| *byte == 0xFF);
    assert!(
        erased,
        "the wallet stayed on the device after moving to the card"
    );
    assert!(
        !walk::contains(&sd::image(), b"about"),
        "the card holds readable words"
    );
}

fn expect_the_wallet_opens(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::open_receive(device);
    walk::expect_address(device, known::RECEIVE_0);
}

fn open_from_the_card() {
    expect_the_wallet_opens(&Device::boot_with_card());
}

/// More tries than the wrong-PIN limit: none of them reached the card, so none may count.
fn refuse_to_open_without_the_card() {
    sd::set_present(false);
    let device = Device::boot();
    device.expect_absent("Scan QR");
    for _ in 0..MORE_TRIES_THAN_THE_LIMIT {
        device.expect_text("UNLOCK PIN");
        device.enter_pin(PIN);
        device.settle();
        device.expect_absent("Scan QR");
        device.expect_absent("Invalid PIN");
        device.expect_text("STORE-SD-01");
        device.let_time_pass(CARD_ERROR_RETRY_DELAY);
        device.tap("OK");
        device.settle();
    }
}

fn open_when_the_card_returns() {
    expect_the_wallet_opens(&Device::boot_with_card());
}
