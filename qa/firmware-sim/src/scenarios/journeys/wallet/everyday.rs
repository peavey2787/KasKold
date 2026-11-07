//! What a person does day to day once a wallet exists.
use crate::harness::{app_state, card, hold_up_qr, lower_qr, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use crate::ui::layout::ADDRESS_QR_ZONE;
use kaspa_portal::crypto::message::message_digest;
use kaspa_portal::crypto::schnorr::{schnorr_verify, SchnorrSignature};
use kaspa_portal::primitives::address::decode_address;
use kaspa_portal::wallet::account::derivation::import_kpub;

/// The event loop counts one idle tick per pass, and the default dim is 60,000 ticks.
const POLLS_UNTIL_DIMMED: u64 = 60_100;
const REQUIRE_PIN_ROW: (u16, u16) = (160, 195);
/// Blank space on the unlock screen: touching a dimmed screen only wakes it.
const WAKE_TAP: (u16, u16) = (160, 60);
const MESSAGE: &str = "hello";
const SCANNED_MESSAGE: &str = "pay the plumber";
const CARD_MESSAGE: &str = "rent is paid";
const MESSAGE_FILE: &[u8; 11] = b"MESSAGE TXT";
const SIGNATURE_QR_TAP: (u16, u16) = (235, 173);
const SIGNATURE_BYTES: usize = 64;
const TAP_FOR_QR: (u16, u16) = (160, 120);
const P2SH_VERSION: u8 = 0x08;

journey!(a_receive_qr_scans_as_the_address_on_screen: restore_the_known_wallet, scan_the_receive_qr);
journey!(a_hidden_key_address_commits_only_to_a_script_hash: restore_the_known_wallet, show_the_hidden_key_address);
journey!(a_kpub_saved_to_a_card_derives_the_wallet_addresses: restore_the_known_wallet, save_the_kpub_to_a_card);
journey!(a_kpub_is_not_saved_without_a_card: restore_the_known_wallet, refuse_the_kpub_without_a_card);
journey!(a_signed_message_verifies_against_the_wallet_key: restore_the_known_wallet, sign_a_message);
journey!(a_message_from_a_qr_signs_what_it_says: restore_the_known_wallet, sign_a_scanned_message);
journey!(a_message_from_a_card_signs_what_it_says: restore_the_known_wallet, sign_a_message_from_the_card);
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

fn show_the_hidden_key_address() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_wallet_tools(&device);
    device.tap("Hidden Key Addr");
    device.expect_text("HIDDEN ADDRESS");
    walk::expect_address(&device, known::HIDDEN_KEY_RECEIVE_0);
    device.tap_at(TAP_FOR_QR.0, TAP_FOR_QR.1);
    let scanned = String::from_utf8(device.wait_for_qr()).expect("address text");
    assert_eq!(scanned.to_lowercase(), known::HIDDEN_KEY_RECEIVE_0);
    let (version, _) = decode_address(&scanned.to_lowercase()).expect("an address");
    assert_eq!(
        version, P2SH_VERSION,
        "the address must be pay-to-script-hash"
    );
    walk::go_back(&device);
    walk::expect_address(&device, known::HIDDEN_KEY_RECEIVE_0);
    walk::go_back(&device);
    device.wait_for_text("Hidden Key Addr");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::WalletAdvancedMenu
    );
    device.tap("Hidden Key Addr");
    walk::expect_address(&device, known::HIDDEN_KEY_RECEIVE_0);
    walk::go_home(&device);
    walk::open_receive(&device);
    device.wait_for_text("RECEIVE");
    walk::expect_address(&device, known::RECEIVE_0);
}

fn save_the_kpub_to_a_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    walk::open_wallet_tools(&device);
    device.tap("kpub to SD");
    device.wait_for_text("KPUB FILENAME");
    walk::go_back(&device);
    device.wait_for_text("kpub to SD");
    device.tap("kpub to SD");
    device.wait_for_text("KPUB FILENAME");
    walk::press_full_ok(&device);
    device.wait_for_text("ENCRYPT FILE?");
    walk::go_back(&device);
    device.wait_for_text("KPUB FILENAME");
    walk::press_full_ok(&device);
    device.wait_for_text("ENCRYPT FILE?");
    device.tap("No");
    device.wait_for_text("kpub to SD");
    let wallet = import_kpub(&walk::kpub_on_the_card(), "kaspa").expect("a readable kpub");
    assert_eq!(wallet.receive_addresses[0], known::RECEIVE_0);
}

fn refuse_the_kpub_without_a_card() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_wallet_tools(&device);
    device.tap("kpub to SD");
    device.expect_was_shown("Insert an SD card");
    device.wait_for_text("kpub to SD");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::WalletAdvancedMenu
    );
}

fn open_sign_message(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    device.tap("Wallet");
    walk::next_menu_page(device);
    device.tap("Advanced");
    device.tap("Sign Message");
}

fn sign_a_message() {
    let device = Device::boot();
    open_sign_message(&device);
    device.tap("Type manually");
    device.type_text(MESSAGE);
    walk::press_full_ok(&device);
    sign_the_reviewed_message(&device, MESSAGE);
}

fn sign_a_scanned_message() {
    let device = Device::boot();
    open_sign_message(&device);
    device.tap("Scan message QR");
    hold_up_qr(SCANNED_MESSAGE.as_bytes());
    device.wait_for_text(SCANNED_MESSAGE);
    lower_qr();
    sign_the_reviewed_message(&device, SCANNED_MESSAGE);
}

fn sign_a_message_from_the_card() {
    let device = Device::boot_with_card();
    card::format_like_a_computer();
    card::put_file(MESSAGE_FILE, format!("{CARD_MESSAGE}\r\n").as_bytes());
    open_sign_message(&device);
    device.tap("Load .TXT from SD");
    device.tap("MESSAGE.TXT");
    device.wait_for_text(CARD_MESSAGE);
    sign_the_reviewed_message(&device, CARD_MESSAGE);
}

fn sign_the_reviewed_message(device: &Device, message: &str) {
    device.expect_text(message);
    device.tap("SIGN");
    device.wait_for_text("SIGNATURE");
    device.tap_at(SIGNATURE_QR_TAP.0, SIGNATURE_QR_TAP.1);
    let scanned = device.wait_for_qr();
    let (signature, digest) = scanned.split_at(SIGNATURE_BYTES);
    assert_eq!(digest, message_digest(message.as_bytes()));
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
