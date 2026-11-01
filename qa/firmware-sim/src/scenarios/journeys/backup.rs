//! Recovery material: what the device shows, and whether it really brings a wallet back.
use super::known::{self, ABOUT_WORDS, PASSWORD, PIN};
use super::walk::{self, restore_the_known_wallet};
use crate::harness::{app_state, discard_board, hold_up_qr, lower_qr, note, recall, Device};
use crate::runtime::input::AppState;
use esp_hal::board::sd;
use std::time::Duration;

const FORMAT_HOLD: Duration = Duration::from_secs(5);
const BACKUP_FILE_ON_DISK: &[u8] = b"SD000001KAS";
const WRONG_PASSWORD: &str = "wrong4horse";

journey!(the_words_shown_are_the_words_restored: restore_the_known_wallet, read_back_the_words);
journey!(
    a_seedqr_read_off_the_screen_restores_the_wallet_on_a_new_device:
    restore_the_known_wallet,
    read_the_seedqr,
    restore_from_the_seedqr,
);
journey!(
    a_card_backup_brings_the_wallet_back_after_a_factory_reset:
    restore_the_known_wallet,
    back_up_to_a_card,
    walk::factory_reset,
    restore_from_the_card,
);

fn open_backup_menu(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    device.tap("Wallet");
    device.tap("Backup");
}

fn read_back_the_words() {
    let device = Device::boot();
    open_backup_menu(&device);
    device.tap("View Words");
    assert_eq!(
        walk::write_down_words(&device, ABOUT_WORDS.len()),
        ABOUT_WORDS
    );
}

fn read_the_seedqr() {
    let device = Device::boot();
    open_backup_menu(&device);
    device.tap("SeedQR Backup");
    let digits = String::from_utf8(device.wait_for_qr()).expect("SeedQR digits");
    assert_eq!(digits, known::ABOUT_SEEDQR);
    note("seedqr", &digits);
    discard_board();
}

fn restore_from_the_seedqr() {
    let device = Device::boot();
    device.tap("Restore Wallet");
    device.tap("SeedQR");
    hold_up_qr(recall("seedqr").as_bytes());
    device.wait_for_text("No Passphrase");
    lower_qr();
    device.tap("No Passphrase");
    walk::save_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn back_up_to_a_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Settings");
    device.tap("Storage");
    device.tap("Format FAT32");
    device.hold("HOLD 4s TO FORMAT", FORMAT_HOLD);
    device.wait_for_text("Format FAT32");
    walk::go_home(&device);
    device.tap("Wallet");
    device.tap("Backup");
    device.tap("Encrypted SD Card");
    device.tap("Yes");
    walk::press_full_ok(&device);
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("View Words");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::WalletBackupMethodsMenu
    );
    let card = sd::image();
    assert!(
        walk::contains(&card, BACKUP_FILE_ON_DISK),
        "the backup never reached the card"
    );
    for word in ABOUT_WORDS {
        assert!(
            !walk::contains(&card, word.as_bytes()),
            "{word:?} is readable on the card"
        );
    }
}

fn restore_from_the_card() {
    let device = Device::boot();
    device.tap("Restore Wallet");
    device.tap("SD");
    device.tap("SD000001.KAS");
    walk::enter_password(&device, WRONG_PASSWORD);
    assert!(
        !app_state().wallet.seeds.seed_loaded,
        "a wrong password opened the backup"
    );
    device.wait_for_text("SD000001.KAS");
    device.tap("SD000001.KAS");
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("No Passphrase");
    device.tap("No Passphrase");
    walk::save_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}
