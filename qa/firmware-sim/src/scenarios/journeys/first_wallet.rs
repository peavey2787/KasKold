//! Getting a first wallet onto a new device, and finding it there after a reboot.
use super::known::{self, ABOUT_WORDS, PIN};
use super::walk;
use crate::harness::{app_state, discard_board, note, recall, Device};
use crate::runtime::input::AppState;
use esp_hal::board::flash;

journey!(a_new_device_boots_to_the_welcome_screen: welcome_screen);
journey!(a_restored_wallet_shows_its_known_addresses_after_a_reboot: restore_the_known_wallet, read_the_known_addresses);
journey!(a_wallet_with_a_passphrase_is_a_different_wallet: restore_with_passphrase, read_the_passphrase_address);
journey!(a_session_only_wallet_is_gone_after_a_reboot: restore_for_this_session, welcomed_again);
journey!(
    a_created_wallet_can_be_restored_from_the_words_it_showed:
    create_and_write_down_words,
    note_the_created_address,
    restore_the_written_words_on_a_new_device,
);
journey!(a_password_protected_wallet_asks_for_its_password_after_a_reboot: restore_with_password, unlock_with_password);

fn welcome_screen() {
    let device = Device::boot();
    device.expect_text("WELCOME");
    device.expect_text("Create Wallet");
    device.expect_text("Restore Wallet");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::StorageModeChoice
    );
}

fn restore_the_known_wallet() {
    let device = Device::boot();
    walk::restore_words(&device, ABOUT_WORDS, "No Passphrase");
    walk::save_with_pin(&device, PIN);
    device.expect_text("Scan QR");
    let stored = flash::image();
    for word in ["abandon", "about"] {
        assert!(
            !walk::contains(&stored, word.as_bytes()),
            "{word:?} is readable in flash"
        );
    }
}

fn read_the_known_addresses() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
    device.tap(">");
    device.expect_text("RECEIVE #1");
    walk::expect_address(&device, known::RECEIVE_1);
    device.tap("<");
    device.tap("Receive");
    device.expect_text("CHANGE #0");
    walk::expect_address(&device, known::CHANGE_0);
}

fn restore_with_passphrase() {
    let device = Device::boot();
    walk::restore_words(&device, ABOUT_WORDS, "Use Passphrase");
    device.type_text(known::PASSPHRASE);
    walk::press_full_ok(&device);
    walk::save_with_pin(&device, PIN);
}

fn read_the_passphrase_address() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::PASSPHRASE_RECEIVE_0);
}

fn restore_for_this_session() {
    let device = Device::boot();
    walk::restore_words(&device, ABOUT_WORDS, "No Passphrase");
    walk::name_wallet(&device);
    device.tap("Use for This Session Only");
    device.expect_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn welcomed_again() {
    Device::boot().expect_text("WELCOME");
}

fn create_and_write_down_words() {
    let device = Device::boot();
    device.tap("Create Wallet");
    walk::name_wallet(&device);
    device.tap("12 Words");
    device.tap("No Dice");
    device.tap("No Touch Entropy");
    device.tap("No Passphrase");
    let words = walk::write_down_words(&device, 12);
    device.tap("I BACKED UP MY WORDS");
    walk::protect_with_pin(&device, PIN);
    note("words", &words.join(" "));
}

fn note_the_created_address() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    device.wait_for_text("RECEIVE #0");
    let words = recall("words");
    let expected = known::address_of(&words);
    walk::expect_address(&device, &expected);
    note("address", &expected);
    discard_board();
}

fn restore_the_written_words_on_a_new_device() {
    let device = Device::boot();
    device.expect_text("WELCOME");
    let words = recall("words");
    let words: Vec<&str> = words.split(' ').collect();
    walk::restore_words(&device, &words, "No Passphrase");
    walk::save_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, &recall("address"));
}

fn restore_with_password() {
    let device = Device::boot();
    walk::restore_words(&device, ABOUT_WORDS, "No Passphrase");
    walk::name_wallet(&device);
    device.tap("Save Securely on Device");
    device.tap("Use PIN / Password");
    device.tap("Use Password");
    walk::enter_password(&device, known::PASSWORD);
    walk::enter_password(&device, known::PASSWORD);
    device.expect_text("Scan QR");
}

fn unlock_with_password() {
    let device = Device::boot();
    device.expect_text("UNLOCK PASSWORD");
    walk::enter_password(&device, known::PASSWORD);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}
