//! Several wallets on one device: each stays itself, and all stay behind the PIN.
use crate::harness::{app_state, Device};
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use std::time::Duration;

const SECOND_WALLET: &str = "Second";
const SECOND_WALLET_PIN: &str = "264817";
const BACKOFF_OVER: Duration = Duration::from_secs(65);
const DELETE_BUTTON: (u16, u16) = (230, 205);
const DELETE_HOLD: Duration = Duration::from_secs(5);

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
journey!(
    a_second_wallet_for_this_session_is_forgotten_at_power_off:
    restore_the_known_wallet,
    add_a_wallet_for_this_session,
    only_the_kept_wallet_comes_back,
);
journey!(
    a_session_wallet_the_device_already_keeps_is_refused:
    restore_the_known_wallet,
    add_the_kept_wallet_for_this_session,
);

journey!(
    deleting_a_wallet_takes_a_long_hold_and_can_be_backed_out_of:
    restore_the_known_wallet,
    change_my_mind_about_deleting,
    delete_the_wallet,
    a_deleted_wallet_stays_gone,
);

fn begin_deleting(device: &Device) {
    walk::open_wallet_menu(device);
    device.tap("Wallet Details");
    device.tap("Delete Wallet");
    device.expect_text("DELETE WALLET?");
    device.tap_at(DELETE_BUTTON.0, DELETE_BUTTON.1);
    device.expect_text("HOLD 4s");
}

fn change_my_mind_about_deleting() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    begin_deleting(&device);
    walk::go_back(&device);
    device.wait_for_text("WALLETS");
    device.expect_text("Main");
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 1);
}

fn delete_the_wallet() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    begin_deleting(&device);
    device.hold("HOLD 4s", DELETE_HOLD);
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 0);
}

/// With nothing to use, the device opens on the wallet list and Back cannot escape it.
fn a_deleted_wallet_stays_gone() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    device.enter_pin(PIN);
    device.wait_for_text("WALLETS");
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 0);
    walk::go_back(&device);
    device.expect_text("WALLETS");
    walk::tap_add_wallet(&device);
    walk::restore_words(&device, known::ABOUT_WORDS, "No Passphrase");
    walk::name_wallet_as(&device, SECOND_WALLET);
    device.tap("Save Securely on Device");
    device.tap("No PIN / Password");
    device.wait_for_text("WALLETS");
    walk::go_back(&device);
    walk::go_back(&device);
    device.wait_for_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

/// Restores a second wallet up to the question of whether the device keeps it.
fn begin_restoring_a_second_wallet(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::open_add_wallet(device);
    device.tap("Restore Wallet");
    device.tap("Words");
    walk::type_words(device, &known::TWENTY_FOUR_WORDS);
    device.tap("No Passphrase");
    walk::name_wallet_as(device, SECOND_WALLET);
}

/// Restores a second wallet up to the question of how it is protected.
fn begin_adding_a_second_wallet(device: &Device) {
    begin_restoring_a_second_wallet(device);
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

fn add_a_wallet_for_this_session() {
    let device = Device::boot();
    begin_restoring_a_second_wallet(&device);
    device.tap("Use for This Session Only");
    device.wait_for_text("WALLETS");
    device.expect_text(SECOND_WALLET);
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 2);
    walk::go_home(&device);
    walk::open_receive(&device);
    walk::expect_address(&device, known::TWENTY_FOUR_WORD_RECEIVE_0);
}

fn only_the_kept_wallet_comes_back() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 1);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn add_the_kept_wallet_for_this_session() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_add_wallet(&device);
    walk::restore_words(&device, known::ABOUT_WORDS, "No Passphrase");
    walk::name_wallet_as(&device, SECOND_WALLET);
    device.tap("Use for This Session Only");
    device.settle();
    device.expect_was_shown("Wallet already exists");
    device.wait_for_text("WALLETS");
    device.expect_text("Main");
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 1);
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
    walk::open_wallet_list(&device);
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
