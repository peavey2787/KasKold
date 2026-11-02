//! A key rather than a phrase joins a device that already holds a wallet.
use super::card_imports::{address_of_raw_key, enter_raw_key};
use crate::harness::{app_state, card, Device};
use crate::scenarios::journeys::known::{self, LEGAL_WORDS, PASSWORD, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};

const SECOND_WALLET: &str = "Second";
const SECOND_WALLET_PIN: &str = "264817";
const WRONG_PASSWORD: &str = "wrong4horse";

journey!(
    an_extended_key_on_a_card_joins_as_a_second_wallet:
    restore_the_known_wallet,
    save_the_extended_key,
    walk::factory_reset,
    restore_another_wallet,
    add_the_extended_key,
    the_extended_key_opens_with_its_own_pin,
);

journey!(
    a_raw_key_joins_as_a_second_wallet:
    restore_the_known_wallet,
    add_a_raw_key,
    the_raw_key_is_still_there,
);

journey!(
    backing_out_of_adding_a_key_keeps_only_the_first_wallet:
    restore_the_known_wallet,
    back_out_of_adding_a_key,
);

fn save_the_extended_key() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    device.tap("Wallet");
    device.tap("Backup");
    device.tap("Advanced");
    device.tap("XPrv Backup");
    device.tap("Encrypt to SD");
    walk::press_full_ok(&device);
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("ADVANCED BACKUP");
    assert!(card::read_file(b"XP000001KAS").is_some());
}

fn restore_another_wallet() {
    let device = Device::boot();
    walk::restore_words(&device, LEGAL_WORDS, "No Passphrase");
    walk::save_with_pin(&device, PIN);
}

fn add_the_extended_key() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::open_add_wallet(&device);
    device.tap("Restore Wallet");
    device.tap("SD");
    device.tap("XP000001.KAS");
    walk::enter_password(&device, WRONG_PASSWORD);
    device.wait_for_text("XP000001.KAS");
    device.tap("XP000001.KAS");
    walk::enter_password(&device, PASSWORD);
    walk::name_wallet_as(&device, SECOND_WALLET);
    device.tap("Save Securely on Device");
    device.tap("Use PIN / Password");
    device.tap("Use PIN");
    device.enter_pin(SECOND_WALLET_PIN);
    device.enter_pin(SECOND_WALLET_PIN);
    device.wait_for_text("WALLETS");
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 2);
}

fn the_extended_key_opens_with_its_own_pin() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    device.enter_pin(PIN);
    device.wait_for_text("WALLETS");
    device.tap(SECOND_WALLET);
    device.wait_for_text("UNLOCK PIN");
    device.enter_pin(SECOND_WALLET_PIN);
    device.wait_for_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn add_a_raw_key() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_add_wallet(&device);
    device.tap("Restore Wallet");
    device.tap("Advanced");
    device.tap("Raw Private Key");
    enter_raw_key(&device);
    walk::name_wallet_as(&device, SECOND_WALLET);
    device.tap("Save Securely on Device");
    device.tap("No PIN / Password");
    device.wait_for_text("WALLETS");
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 2);
    walk::go_home(&device);
    walk::open_receive(&device);
    walk::expect_address(&device, &address_of_raw_key());
}

fn the_raw_key_is_still_there() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    device.enter_pin(PIN);
    device.wait_for_text("WALLETS");
    device.tap(SECOND_WALLET);
    device.wait_for_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, &address_of_raw_key());
}

fn back_out_of_adding_a_key() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_add_wallet(&device);
    device.tap("Restore Wallet");
    device.tap("Advanced");
    device.tap("Raw Private Key");
    enter_raw_key(&device);
    walk::go_back(&device);
    device.settle();
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 1);
    walk::go_home(&device);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}
