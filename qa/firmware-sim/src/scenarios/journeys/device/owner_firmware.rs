//! Owner firmware: nothing is enrolled or installed until the file on the card checks out.
use crate::harness::{app_state, card, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::PIN;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use sha2::{Digest, Sha256};

const OWNER_KEY_FILE: &[u8; 11] = b"OWNERKEYKAS";
const OWNER_FIRMWARE_FILE: &[u8; 11] = b"OWNERFW BIN";
const OWNER_KEY_MAGIC: &[u8; 8] = b"KSOWNR01";
const OWNER_KEY_DIGEST: [u8; 32] = [0x5A; 32];
const OWNER_KEY_CHECKSUMMED: usize = 44;
const ENROLL_PHRASE: &str = "ENROLL OWNER";
const INSTALL_PHRASE: &str = "INSTALL OWNER";
const WRONG_PHRASE: &str = "ENROL OWNER";
const WARNING_CONTINUE: (u16, u16) = (238, 212);

journey!(an_owner_key_is_validated_before_any_enrollment: restore_the_known_wallet, enroll_an_owner_key);
journey!(owner_firmware_is_read_only_after_its_phrase: restore_the_known_wallet, install_owner_firmware);

fn owner_key_record() -> Vec<u8> {
    let mut record = OWNER_KEY_MAGIC.to_vec();
    record.extend_from_slice(&[1, 0, 0, 0]);
    record.extend_from_slice(&OWNER_KEY_DIGEST);
    let checksum = Sha256::digest(&record[..OWNER_KEY_CHECKSUMMED]);
    record.extend_from_slice(&checksum);
    record
}

fn open_owner_menu(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::format_card(device);
    device.tap("Settings");
    walk::next_menu_page(device);
    device.tap("Advanced");
    device.tap("Owner Firmware");
}

fn past_the_warning(device: &Device) {
    device.settle();
    device.tap_at(WARNING_CONTINUE.0, WARNING_CONTINUE.1);
    device.settle();
}

fn enter_phrase(device: &Device, phrase: &str) {
    device.type_text(phrase);
    walk::press_full_ok(device);
    device.settle();
}

fn enroll_an_owner_key() {
    let device = Device::boot_with_card();
    open_owner_menu(&device);
    device.tap("Enroll Owner Key");
    past_the_warning(&device);
    assert_eq!(app_state().navigation.app.state, AppState::OwnerKeyConfirm);
    enter_phrase(&device, ENROLL_PHRASE);
    assert_eq!(
        app_state().pop_it.error,
        Some("File not found"),
        "no key file is on the card yet"
    );
    let mut damaged = owner_key_record();
    damaged[20] ^= 1;
    card::put_file(OWNER_KEY_FILE, &damaged);
    press_ok_again(&device);
    assert_eq!(app_state().pop_it.error, Some("OWNERKEY.KAS is invalid"));
    card::put_file(OWNER_KEY_FILE, &owner_key_record());
    press_ok_again(&device);
    device.wait_for_text("Enroll Owner Key");
    device.expect_was_shown("Owner key validated");
    device.tap("Enroll Owner Key");
    past_the_warning(&device);
    enter_phrase(&device, WRONG_PHRASE);
    assert_eq!(
        app_state().navigation.app.state,
        AppState::OwnerKeyConfirm,
        "a misspelt phrase must not start anything"
    );
}

fn press_ok_again(device: &Device) {
    walk::press_full_ok(device);
    device.settle();
}

fn install_owner_firmware() {
    let device = Device::boot_with_card();
    open_owner_menu(&device);
    device.tap("Install from SD");
    past_the_warning(&device);
    assert_eq!(
        app_state().navigation.app.state,
        AppState::OwnerInstallConfirm
    );
    enter_phrase(&device, INSTALL_PHRASE);
    assert_eq!(app_state().pop_it.error, Some("File not found"));
    card::put_file(
        OWNER_FIRMWARE_FILE,
        b"an owner image the bootloader will judge",
    );
    press_ok_again(&device);
    device.wait_for_text("Install from SD");
    device.expect_was_shown("OWNERFW.BIN validated");
}
