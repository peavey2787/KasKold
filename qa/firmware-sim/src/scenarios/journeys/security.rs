//! What the device does to someone who should not be using it.
use super::known::{self, ABOUT_WORDS, PIN};
use super::walk;
use crate::harness::Device;
use std::time::Duration;

const WRONG_PIN: &str = "111111";
const DURESS_PIN: &str = "771144";
const LONGEST_BACKOFF: Duration = Duration::from_secs(60);
/// The two sectors holding saved wallets, at the top of the 16 MiB flash.
const WALLET_RECORDS: core::ops::Range<usize> = 0xFF_E000..0x100_0000;
/// The Duress card on the Security screen, which keeps its place as the card text changes.
const DURESS_CARD: (u16, u16) = (160, 55);

journey!(
    wrong_pins_are_remembered_across_reboots_and_the_tenth_erases_the_wallet:
    restore_the_wallet,
    four_wrong_pins_then_the_right_one,
    five_wrong_pins,
    the_warning_survives_a_reboot_and_four_more_end_in_one_try,
    the_tenth_wrong_pin_erases_the_wallet,
    welcomed_like_a_new_device,
);
journey!(
    a_pin_chosen_at_setup_is_asked_at_every_power_on_and_guards_the_wallet:
    restore_the_wallet,
    the_pin_opens_the_wallet,
    the_pin_opens_the_wallet,
    a_wrong_pin_shows_no_wallet,
);
journey!(a_pin_added_later_guards_the_wallet_after_a_reboot: restore_without_a_pin_then_add_one, only_its_pin_opens_it);
journey!(
    a_duress_pin_erases_the_wallet_and_the_real_pin_keeps_working:
    restore_the_wallet,
    set_a_duress_pin,
    the_pin_opens_the_wallet,
    the_duress_pin_erases_the_wallet,
    welcomed_like_a_new_device,
);
journey!(a_factory_reset_leaves_a_new_device: restore_the_wallet, factory_reset, welcomed_like_a_new_device);
journey!(a_wrong_pin_during_the_backoff_is_not_even_tried: restore_the_wallet, retry_too_soon);
journey!(modified_firmware_never_reaches_a_wallet: restore_the_wallet, refuse_modified_firmware);
journey!(firmware_with_a_bad_signature_never_reaches_a_wallet: restore_the_wallet, refuse_bad_signature);

fn restore_the_wallet() {
    let device = Device::boot();
    walk::restore_words(&device, ABOUT_WORDS, "No Passphrase");
    walk::save_with_pin(&device, PIN);
}

fn the_pin_opens_the_wallet() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn a_wrong_pin_shows_no_wallet() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    device.enter_pin(WRONG_PIN);
    device.expect_text("Invalid PIN");
    assert!(
        !crate::harness::app_state().wallet.seeds.seed_loaded,
        "a wrong PIN opened the wallet"
    );
}

fn restore_without_a_pin_then_add_one() {
    let device = Device::boot();
    walk::restore_words(&device, ABOUT_WORDS, "No Passphrase");
    walk::name_wallet(&device);
    device.tap("Save Securely on Device");
    device.tap("No PIN / Password");
    device.tap("Wallet");
    device.tap("Wallet Details");
    device.tap("Add PIN / Password");
    device.tap("Use PIN");
    device.enter_pin(PIN);
    device.enter_pin(PIN);
    device.expect_text("PIN Enabled");
}

fn only_its_pin_opens_it() {
    let device = Device::boot();
    device.tap("Main");
    device.expect_text("UNLOCK PIN");
    device.enter_pin(WRONG_PIN);
    device.expect_text("Invalid PIN");
    device.let_time_pass(LONGEST_BACKOFF);
    device.enter_pin(PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn set_a_duress_pin() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Settings");
    device.tap("Security");
    device.tap_at(DURESS_CARD.0, DURESS_CARD.1);
    walk::accept_warning(&device);
    device.enter_on_number_row(PIN);
    device.enter_on_number_row(PIN);
    device.wait_for_text("SET DURESS PIN");
    device.enter_on_number_row(DURESS_PIN);
    device.enter_on_number_row(DURESS_PIN);
    device.wait_for_text("ENABLED PIN");
}

fn the_duress_pin_erases_the_wallet() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    device.enter_pin(DURESS_PIN);
    device.wait_until_stopped_or_text("WELCOME");
    assert_no_saved_wallet();
}

fn factory_reset() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Settings");
    walk::next_menu_page(&device);
    device.tap("Advanced");
    device.tap("Factory Reset");
    walk::accept_warning(&device);
    walk::accept_warning(&device);
    device.wait_until_stopped_or_text("WELCOME");
    assert_no_saved_wallet();
}

fn assert_no_saved_wallet() {
    let saved = esp_hal::board::flash::image();
    let erased = saved[WALLET_RECORDS].iter().all(|byte| *byte == 0xFF);
    assert!(erased, "a wallet record survived the erase");
}

fn wrong_pin(device: &Device) {
    device.let_time_pass(LONGEST_BACKOFF);
    device.enter_pin(WRONG_PIN);
}

fn four_wrong_pins_then_the_right_one() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    (0..4).for_each(|_| wrong_pin(&device));
    device.expect_text("Invalid PIN");
    device.let_time_pass(LONGEST_BACKOFF);
    device.enter_pin(PIN);
    device.expect_text("Scan QR");
}

fn five_wrong_pins() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    (0..5).for_each(|_| wrong_pin(&device));
    device.expect_text("5 TRIES LEFT");
}

fn the_warning_survives_a_reboot_and_four_more_end_in_one_try() {
    let device = Device::boot();
    device.expect_text("5 TRIES LEFT");
    (0..4).for_each(|_| wrong_pin(&device));
    device.expect_text("1 TRY LEFT");
}

fn the_tenth_wrong_pin_erases_the_wallet() {
    let device = Device::boot();
    device.expect_text("1 TRY LEFT");
    wrong_pin(&device);
    device.wait_until_stopped_or_text("WELCOME");
}

fn welcomed_like_a_new_device() {
    Device::boot().expect_text("WELCOME");
}

fn retry_too_soon() {
    let device = Device::boot();
    device.enter_pin(WRONG_PIN);
    device.expect_text("Invalid PIN");
    let failures = crate::harness::app_state()
        .storage
        .persistence
        .unlock_failures;
    device.enter_pin(WRONG_PIN);
    assert_eq!(
        crate::harness::app_state()
            .storage
            .persistence
            .unlock_failures,
        failures
    );
}

fn refuse_modified_firmware() {
    Device::boot_tampered().expect_refusal("HASH INVALID");
}

fn refuse_bad_signature() {
    Device::boot_with_bad_signature().expect_refusal("SIGNATURE INVALID");
}
