//! What else a card or a keypad brings in: raw keys, watch-only wallets, addresses and covenants.
use crate::harness::{app_state, card, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use kaskold_hardware_core::input::keyboard::{KeyAction, KeyboardMode};
use kaskold_protocol::wire::qr_payload::PAYLOAD_V1_RAW;
use kaspa_portal::primitives::address::{encode_p2pk, MAX_ADDR_LEN};
use kaspa_portal::wallet::account::derivation::import_kpub;
use offline_signer::derivation::xpub::{decode_kpub_or_xpub, XPUB_PAYLOAD_LEN};
use sha2::{Digest, Sha256};

const RAW_KEY: &str = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318";
const OFF_THE_CURVE: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const NOT_A_KPUB: &[u8] = b"this is no kpub";
const NOT_AN_ADDRESS: &[u8] = b"this is no address";
const FAKE_ADDRESS: &[u8] = b"kaspa:thisonlylookslikeanaddress";
const ADDRESS_QR_TAP: (u16, u16) = (160, 120);
const COVENANT_BACKUP: &[u8] = b"COVB a covenant someone must keep to spend";

journey!(a_raw_key_typed_at_restore_becomes_the_wallet: type_a_raw_key_at_restore);
journey!(a_raw_key_off_the_curve_is_refused: refuse_a_key_off_the_curve);
journey!(a_kpub_on_a_card_opens_as_watch_only: restore_the_known_wallet, open_a_saved_kpub);
journey!(a_file_that_is_no_kpub_is_refused: restore_the_known_wallet, refuse_a_bad_kpub);
journey!(a_multisig_address_on_a_card_opens: restore_the_known_wallet, open_an_address_file);
journey!(a_file_that_is_no_address_is_refused: restore_the_known_wallet, refuse_a_bad_address);
journey!(a_file_that_only_looks_like_an_address_is_refused: restore_the_known_wallet, refuse_a_fake_address);
journey!(a_covenant_backup_on_a_card_shows_as_its_code: restore_the_known_wallet, show_a_covenant_backup);
journey!(a_key_in_a_covenant_file_joins_the_wallets_once: restore_the_known_wallet, import_a_key_file);
journey!(a_file_of_no_known_format_is_refused: restore_the_known_wallet, refuse_an_unknown_file);
journey!(a_card_without_files_of_a_kind_says_so: restore_the_known_wallet, say_what_is_missing);

fn prepare_card(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::format_card(device);
}

fn open_recovery(device: &Device, row: &str) {
    walk::open_wallet_menu(device);
    device.tap("Recovery");
    device.tap(row);
}

fn open_covenant_files(device: &Device) {
    walk::open_wallet_menu(device);
    device.tap("Recovery");
    walk::next_menu_page(device);
    device.tap("Covenant Restore");
}

fn open_raw_key_entry(device: &Device) {
    device.tap("Restore Wallet");
    device.tap("Advanced");
    device.tap("Raw Private Key");
    device.settle();
    assert_eq!(app_state().navigation.app.state, AppState::ImportPrivKey);
}

fn enter_hex(device: &Device, hex: &str) {
    for character in hex.bytes() {
        let key = KeyAction::Char(character.to_ascii_uppercase());
        device.press_key(key, KeyboardMode::Hex, 0);
    }
    device.press_key(KeyAction::Ok, KeyboardMode::Hex, 0);
}

/// Types the known raw key at the prompt that asks for one.
pub(super) fn enter_raw_key(device: &Device) {
    enter_hex(device, RAW_KEY);
}

pub(super) fn address_of_raw_key() -> String {
    let secret: Vec<u8> = (0..RAW_KEY.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&RAW_KEY[at..at + 2], 16).expect("hex"))
        .collect();
    let secret: [u8; 32] = secret.try_into().expect("32 bytes");
    let key = k256::schnorr::SigningKey::from_bytes((&secret).into()).expect("a valid key");
    let public: [u8; 32] = key.verifying_key().to_bytes().into();
    let mut address = [0u8; MAX_ADDR_LEN];
    let length = encode_p2pk(&public, &mut address);
    core::str::from_utf8(&address[..length])
        .expect("text")
        .to_string()
}

fn type_a_raw_key_at_restore() {
    let device = Device::boot();
    open_raw_key_entry(&device);
    enter_hex(&device, RAW_KEY);
    device.expect_was_shown("Key imported!");
    device.settle();
    assert_eq!(
        app_state().navigation.app.state,
        AppState::StorageFinalizeChoice
    );
    device.tap("Use for This Session Only");
    device.wait_for_text("Scan QR");
    walk::open_receive(&device);
    walk::expect_address(&device, &address_of_raw_key());
}

fn refuse_a_key_off_the_curve() {
    let device = Device::boot();
    open_raw_key_entry(&device);
    enter_hex(&device, OFF_THE_CURVE);
    device.expect_was_shown("Invalid key (not on curve)");
    assert_eq!(app_state().navigation.app.state, AppState::ImportPrivKey);
    assert!(!app_state().wallet.seeds.seed_loaded);
}

fn save_the_kpub(device: &Device) {
    walk::open_wallet_tools(device);
    device.tap("kpub to SD");
    device.wait_for_text("KPUB FILENAME");
    walk::press_full_ok(device);
    device.wait_for_text("ENCRYPT FILE?");
    device.tap("No");
    device.wait_for_text("kpub to SD");
    walk::go_home(device);
}

fn open_a_saved_kpub() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    save_the_kpub(&device);
    open_recovery(&device, "kpub (Watch-Only)");
    device.tap(".TXT");
    device.settle();
    assert_eq!(app_state().navigation.app.state, AppState::ExportKpub);
    let saved = walk::kpub_on_the_card();
    let wallet = import_kpub(&saved, "kaspa").expect("a readable kpub");
    assert_eq!(wallet.receive_addresses[0], known::RECEIVE_0);
    let mut payload = [0u8; XPUB_PAYLOAD_LEN];
    decode_kpub_or_xpub(saved.as_bytes(), &mut payload).expect("a kpub");
    let shown = device.wait_for_qr();
    assert_eq!(shown[0], PAYLOAD_V1_RAW);
    assert_eq!(&shown[1..], &payload[..]);
}

fn refuse_a_bad_kpub() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"NOTAKPUBTXT", NOT_A_KPUB);
    open_recovery(&device, "kpub (Watch-Only)");
    device.tap("NOTAKPUB.TXT");
    device.expect_was_shown("Not a valid kpub");
}

fn open_an_address_file() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"SHARED  TXT", known::SHARED_RECEIVE_0.as_bytes());
    open_recovery(&device, "Multisig Address");
    device.tap("SHARED.TXT");
    device.expect_was_shown("Address loaded!");
    device.settle();
    assert_eq!(
        app_state().navigation.app.state,
        AppState::MultisigShowAddress
    );
    walk::expect_address(&device, known::SHARED_RECEIVE_0);
    device.tap_at(ADDRESS_QR_TAP.0, ADDRESS_QR_TAP.1);
    assert_eq!(device.wait_for_qr(), known::SHARED_RECEIVE_0.as_bytes());
    device.tap_at(ADDRESS_QR_TAP.0, ADDRESS_QR_TAP.1);
    device.wait_for_text("Multisig Address");
}

fn refuse_a_bad_address() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"NOTADDR TXT", NOT_AN_ADDRESS);
    open_recovery(&device, "Multisig Address");
    device.tap("NOTADDR.TXT");
    device.expect_was_shown("Not a valid address");
}

fn refuse_a_fake_address() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"FAKEADDRTXT", FAKE_ADDRESS);
    open_recovery(&device, "Multisig Address");
    device.tap("FAKEADDR.TXT");
    device.settle();
    device.expect_was_shown("Not a valid address");
}

fn show_a_covenant_backup() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"CONTRACTCOV", COVENANT_BACKUP);
    open_covenant_files(&device);
    device.tap("CONTRACT.COV");
    assert_eq!(device.wait_for_qr(), COVENANT_BACKUP);
    walk::go_back(&device);
    device.settle();
    assert_eq!(app_state().navigation.app.state, AppState::SdFileList);
}

fn import_a_key_file() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"RAWKEY  COV", RAW_KEY.as_bytes());
    open_covenant_files(&device);
    device.tap("RAWKEY.COV");
    device.expect_was_shown("Key imported!");
    device.settle();
    let wallets = app_state().wallet.seeds.seed_mgr.count();
    assert_eq!(app_state().navigation.app.state, AppState::SdFileList);
    device.tap("RAWKEY.COV");
    device.expect_was_shown("Key imported!");
    device.settle();
    assert_eq!(
        app_state().wallet.seeds.seed_mgr.count(),
        wallets,
        "the same key must not take a second slot"
    );
}

fn refuse_an_unknown_file() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"MYSTERY COV", &Sha256::digest(b"nothing the device knows"));
    open_covenant_files(&device);
    device.tap("MYSTERY.COV");
    device.expect_was_shown("Unknown file format");
}

fn say_what_is_missing() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    open_recovery(&device, "kpub (Watch-Only)");
    device.expect_was_shown("No .TXT files found");
    device.settle();
    walk::next_menu_page(&device);
    device.tap("Covenant Restore");
    device.expect_was_shown("No .COV files on SD");
}
