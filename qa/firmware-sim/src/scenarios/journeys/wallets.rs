//! Several wallets on one device: each stays itself, and all stay behind the PIN.
use super::known::{self, PIN};
use super::walk::{self, restore_the_known_wallet};
use crate::harness::Device;
use std::time::Duration;

const SECOND_WALLET: &str = "Second";
const SECOND_WALLET_PIN: &str = "264817";
const ADD_WALLET_ROW: (u16, u16) = (160, 70);
const BACKOFF_OVER: Duration = Duration::from_secs(65);

journey!(
    a_second_wallet_keeps_the_first_and_the_pin:
    restore_the_known_wallet,
    add_a_wallet_without_its_own_pin,
    both_wallets_stay_behind_the_pin,
);
journey!(
    a_second_wallet_with_its_own_pin_asks_for_that_pin:
    restore_the_known_wallet,
    add_a_wallet_with_its_own_pin,
    open_the_second_wallet_with_its_own_pin,
);

fn open_wallet_list(device: &Device) {
    device.tap("Wallet");
    walk::next_menu_page(device);
    device.tap("Switch / Add Wallet");
}

/// Restores a second wallet up to the question of how it is protected.
fn begin_adding_a_second_wallet(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    open_wallet_list(device);
    device.tap_at(ADD_WALLET_ROW.0, ADD_WALLET_ROW.1);
    device.tap("Restore Wallet");
    device.tap("Words");
    walk::type_words(device, &known::TWENTY_FOUR_WORDS);
    device.tap("No Passphrase");
    walk::name_wallet_as(device, SECOND_WALLET);
    device.tap("Save Securely on Device");
}

fn add_a_wallet_without_its_own_pin() {
    let device = Device::boot();
    begin_adding_a_second_wallet(&device);
    device.tap("No PIN / Password");
    device.wait_for_text("WALLETS");
    device.expect_text("Main");
    walk::go_home(&device);
    walk::open_receive(&device);
    walk::expect_address(&device, known::TWENTY_FOUR_WORD_RECEIVE_0);
}

fn add_a_wallet_with_its_own_pin() {
    let device = Device::boot();
    begin_adding_a_second_wallet(&device);
    device.tap("Use PIN / Password");
    device.tap("Use PIN");
    device.enter_pin(SECOND_WALLET_PIN);
    device.enter_pin(SECOND_WALLET_PIN);
    device.wait_for_text("WALLETS");
}

fn open_the_list_after_the_device_pin(device: &Device) {
    device.expect_text("UNLOCK PIN");
    device.enter_pin(PIN);
    device.wait_for_text("WALLETS");
}

fn both_wallets_stay_behind_the_pin() {
    let device = Device::boot();
    open_the_list_after_the_device_pin(&device);
    device.tap(SECOND_WALLET);
    device.wait_for_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, known::TWENTY_FOUR_WORD_RECEIVE_0);
    walk::go_home(&device);
    open_wallet_list(&device);
    device.tap("Main");
    device.wait_for_text("Multisig");
    walk::go_home(&device);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn open_the_second_wallet_with_its_own_pin() {
    let device = Device::boot();
    open_the_list_after_the_device_pin(&device);
    device.tap(SECOND_WALLET);
    device.wait_for_text("UNLOCK PIN");
    device.enter_pin(PIN);
    device.expect_absent("Scan QR");
    device.let_time_pass(BACKOFF_OVER);
    device.settle();
    device.enter_pin(SECOND_WALLET_PIN);
    device.wait_for_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, known::TWENTY_FOUR_WORD_RECEIVE_0);
}
