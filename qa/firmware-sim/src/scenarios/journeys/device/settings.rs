//! The storage screen: what it claims about a card is what the card holds.
use crate::harness::{card, Device};
use crate::scenarios::journeys::known::PIN;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use esp_hal::board::sd;

const TEST_FILE: &[u8; 11] = b"TEST    TXT";
const STORAGE_TEST_TAP: (u16, u16) = (235, 120);

journey!(the_card_test_proves_the_card_and_leaves_no_file: restore_the_known_wallet, test_a_good_card);
journey!(a_blank_card_fails_the_card_test: restore_the_known_wallet, test_a_blank_card);
journey!(storage_settings_without_a_card_offer_nothing: restore_the_known_wallet, open_storage_without_a_card);

fn open_storage(device: &Device) {
    device.tap("Settings");
    device.tap("Storage");
    device.wait_for_text("SD CARD");
}

fn test_a_good_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    open_storage(&device);
    device.expect_text("Card detected");
    device.tap_at(STORAGE_TEST_TAP.0, STORAGE_TEST_TAP.1);
    device.wait_for_text("SD CARD");
    device.expect_was_shown("Data verify: match");
    assert_eq!(
        card::read_file(TEST_FILE),
        None,
        "the test left its file behind"
    );
}

fn test_a_blank_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    open_storage(&device);
    device.tap_at(STORAGE_TEST_TAP.0, STORAGE_TEST_TAP.1);
    device.wait_for_text("SD CARD");
    device.expect_was_shown("No FAT32 filesystem found");
}

fn open_storage_without_a_card() {
    sd::set_present(false);
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    open_storage(&device);
    device.expect_absent("Format FAT32");
    device.tap_at(STORAGE_TEST_TAP.0, STORAGE_TEST_TAP.1);
    device.expect_text("SD CARD");
    walk::go_back(&device);
    device.wait_for_text("Storage");
}
