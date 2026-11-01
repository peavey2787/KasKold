//! What a person does day to day once a wallet exists.
use super::known::{self, PIN};
use super::walk::{self, restore_the_known_wallet};
use crate::harness::Device;
use crate::ui::layout::ADDRESS_QR_ZONE;
use kaspa_portal::crypto::message::message_digest;
use kaspa_portal::crypto::schnorr::{schnorr_verify, SchnorrSignature};

/// The event loop counts one idle tick per pass, and the default dim is 60,000 ticks.
const POLLS_UNTIL_DIMMED: u64 = 60_100;
const REQUIRE_PIN_ROW: (u16, u16) = (160, 195);
/// Blank space on the unlock screen: touching a dimmed screen only wakes it.
const WAKE_TAP: (u16, u16) = (160, 60);
const MESSAGE: &str = "hello";
const SIGNATURE_QR_TAP: (u16, u16) = (235, 173);
const SIGNATURE_BYTES: usize = 64;

journey!(a_receive_qr_scans_as_the_address_on_screen: restore_the_known_wallet, scan_the_receive_qr);
journey!(a_signed_message_verifies_against_the_wallet_key: restore_the_known_wallet, sign_a_message);
journey!(an_idle_screen_stays_open_unless_the_lock_is_on: restore_the_known_wallet, idle_without_the_lock);
journey!(
    the_screen_lock_asks_for_the_pin_and_survives_a_reboot:
    restore_the_known_wallet,
    lock_after_idle,
    lock_setting_survives,
);

fn scan_the_receive_qr() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
    let zone = ADDRESS_QR_ZONE;
    device.tap_at(zone.x + zone.w / 2, zone.y + zone.h / 2);
    let scanned = String::from_utf8(device.wait_for_qr()).expect("address text");
    assert_eq!(scanned.to_lowercase(), known::RECEIVE_0);
}

fn sign_a_message() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Wallet");
    walk::next_menu_page(&device);
    device.tap("Advanced");
    device.tap("Sign Message");
    device.tap("Type manually");
    device.type_text(MESSAGE);
    walk::press_full_ok(&device);
    device.expect_text(MESSAGE);
    device.tap("SIGN");
    device.wait_for_text("SIGNATURE");
    device.tap_at(SIGNATURE_QR_TAP.0, SIGNATURE_QR_TAP.1);
    let scanned = device.wait_for_qr();
    let (signature, digest) = scanned.split_at(SIGNATURE_BYTES);
    assert_eq!(digest, message_digest(MESSAGE.as_bytes()));
    let signature = SchnorrSignature {
        bytes: signature.try_into().expect("a signature"),
    };
    let digest: [u8; 32] = digest.try_into().expect("a digest");
    schnorr_verify(&known::about_message_key(), &digest, &signature)
        .expect("the signature verifies against the wallet key");
}

fn idle_without_the_lock() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.idle(POLLS_UNTIL_DIMMED);
    device.settle();
    device.expect_text("Scan QR");
    device.expect_absent("UNLOCK PIN");
}

fn lock_after_idle() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Settings");
    device.tap("Display");
    device.tap_at(REQUIRE_PIN_ROW.0, REQUIRE_PIN_ROW.1);
    device.expect_text("ON");
    walk::go_home(&device);
    device.idle(POLLS_UNTIL_DIMMED);
    device.settle();
    device.expect_text("UNLOCK PIN");
    device.tap_at(WAKE_TAP.0, WAKE_TAP.1);
    device.enter_pin(PIN);
    device.wait_for_text("Scan QR");
}

fn lock_setting_survives() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.idle(POLLS_UNTIL_DIMMED);
    device.settle();
    device.expect_text("UNLOCK PIN");
}
