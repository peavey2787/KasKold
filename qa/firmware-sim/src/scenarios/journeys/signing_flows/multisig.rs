//! A shared wallet: cosigners exchange keys by QR and everyone must arrive at one address.
use crate::harness::{app_state, discard_board, hold_up_qr, lower_qr, note, recall, Device};
use crate::scenarios::journeys::known::{self, ABOUT_WORDS, LEGAL_WORDS, PIN, TWENTY_FOUR_WORDS};
use crate::scenarios::journeys::signing_flows::signing::read_every_frame;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use kaskold_protocol::wire::qr_payload::unwrap_v1_raw;
use kaspa_portal::crypto::schnorr::{schnorr_verify, SchnorrSignature};
use kaspa_portal::primitives::address::script_to_p2sh_address;
use kaspa_portal::transaction::interchange::kspt::decode_compact_kspt;
use kaspa_portal::transaction::model::{Ms45Hint, MultisigConfig, SigHashType, Transaction};
use kaspa_portal::transaction::sighash::calculate_sighash;
use kaspa_portal::wallet::derivation::bip32::{
    derive_multisig_account_key, derive_multisig_address_key,
};
use kaspa_portal::wallet::key::xpub::{
    derive_and_serialize_multisig_kpub, parse_kpub_parts, KPUB_MAX_LEN,
};
use offline_signer::derivation::xpub::{decode_kpub_or_xpub, XPUB_PAYLOAD_LEN};

const BACK_TAP: (u16, u16) = (18, 18);
pub(super) const ADDRESS_TAP: (u16, u16) = (160, 120);
const NEXT_ADDRESS_TAP: (u16, u16) = (290, 224);
const PREVIOUS_ADDRESS_TAP: (u16, u16) = (30, 224);
pub(super) const SAVE_YES_TAP: (u16, u16) = (90, 160);
const SAVE_NO_TAP: (u16, u16) = (225, 160);
/// The about wallet sorts second of the three cosigners, so its addresses use branch 1.
const ABOUT_COSIGNER_BRANCH: u32 = 1;
const PARTIAL_NOTE: &str = "partially-signed";
const WRONG_PIN: &str = "111111";
const WRONG_PIN_BACKOFF: core::time::Duration = core::time::Duration::from_secs(60);

journey!(
    two_devices_complete_a_two_of_three_spend:
    restore_the_known_wallet,
    create_the_shared_wallet_and_sign_first,
    sign_second_on_another_device,
);

journey!(
    a_saved_shared_wallet_signs_after_a_power_cycle:
    restore_the_known_wallet,
    save_the_shared_wallet,
    sign_with_the_saved_shared_wallet,
    write_the_device_record_again,
    the_saved_shared_wallet_is_still_there,
);

fn cosigner_words() -> [&'static [&'static str]; 3] {
    [ABOUT_WORDS, LEGAL_WORDS, &TWENTY_FOUR_WORDS]
}

fn kpub_of(words: &[&str]) -> Vec<u8> {
    let mut text = [0u8; KPUB_MAX_LEN];
    let seed = known::multisig_seed_of(words);
    let length = derive_and_serialize_multisig_kpub(&seed.bytes, &mut text).expect("a kpub");
    text[..length].to_vec()
}

fn signing_key_of(words: &[&str]) -> [u8; 32] {
    let seed = known::multisig_seed_of(words);
    let account = derive_multisig_account_key(&seed.bytes, 0).expect("an account");
    derive_multisig_address_key(&account, ABOUT_COSIGNER_BRANCH, 0, 0)
        .expect("an address key")
        .public_key_x_only()
        .expect("a public key")
}

/// The 2-of-n shared wallet these wallets make together.
pub(super) fn shared_config(wallets: &[&[&str]]) -> MultisigConfig {
    let mut config = MultisigConfig::new();
    config.m = 2;
    config.n = wallets.len() as u8;
    config.v45 = true;
    for (slot, words) in wallets.iter().enumerate() {
        let parts = parse_kpub_parts(&kpub_of(words)).expect("kpub parts");
        assert!(config.set_cosigner(slot, &parts));
    }
    config.sort_cosigners();
    config
}

fn shared_redeem_script() -> Vec<u8> {
    let mut config = shared_config(&cosigner_words());
    config.cosigner_index = ABOUT_COSIGNER_BRANCH as u8;
    assert_ne!(config.build_script(), 0);
    config.script[..config.script_len].to_vec()
}

fn shared_transaction() -> Vec<u8> {
    let hint = Ms45Hint {
        present: true,
        cosigner: ABOUT_COSIGNER_BRANCH,
        chain: 0,
        index: 0,
    };
    known::transaction_spending_shared(&shared_redeem_script(), hint)
}

pub(super) fn open_multisig_menu(device: &Device) {
    device.tap("Wallet");
    walk::next_menu_page(device);
    device.tap("Multisig");
}

fn scan_cosigner(device: &Device, position: usize, kpub: &[u8]) {
    device.wait_for_text(&format!("KEY {position}/3"));
    device.tap("Scan kpub QR");
    hold_up_qr(kpub);
    device.settle();
    lower_qr();
}

/// The device's own key arrives through its own QR, as it would from another device.
pub(super) fn add_all_cosigners(device: &Device, own_key: &[u8]) {
    device.tap("Create Multisig");
    device.tap("NEXT");
    scan_cosigner(device, 1, own_key);
    let others = cosigner_words().map(kpub_of);
    let mut position = 2;
    for kpub in others.iter().filter(|kpub| !is_same_key(kpub, own_key)) {
        scan_cosigner(device, position, kpub);
        position += 1;
    }
    device.wait_for_text("MULTISIG WALLET");
}

fn is_same_key(kpub: &[u8], own_key: &[u8]) -> bool {
    let mut payload = [0u8; XPUB_PAYLOAD_LEN];
    decode_kpub_or_xpub(kpub, &mut payload).expect("a kpub");
    unwrap_v1_raw(own_key) == Some(&payload[..])
}

pub(super) fn read_own_key(device: &Device, words: &[&str]) -> Vec<u8> {
    device.tap("kpub Multisig QR");
    let own_key = device.wait_for_qr();
    assert!(is_same_key(&kpub_of(words), &own_key));
    device.tap_at(BACK_TAP.0, BACK_TAP.1);
    device.wait_for_text("Create Multisig");
    own_key
}

fn save_the_descriptor(device: &Device) {
    device.tap_at(ADDRESS_TAP.0, ADDRESS_TAP.1);
    let scanned = String::from_utf8(device.wait_for_qr()).expect("address text");
    device.tap_at(ADDRESS_TAP.0, ADDRESS_TAP.1);
    device.wait_for_text("SAVE MULTISIG?");
    device.tap_at(SAVE_YES_TAP.0, SAVE_YES_TAP.1);
    device.wait_for_text("SAVE ADDRESS?");
    device.tap_at(SAVE_NO_TAP.0, SAVE_NO_TAP.1);
    device.settle();
    note("shared-address-qr", &scanned.to_lowercase());
}

fn scan_and_sign(device: &Device, transaction: &[u8]) -> Transaction {
    walk::go_home(device);
    device.tap("Scan QR");
    hold_up_qr(transaction);
    device.wait_for_text("CONFIRM SEND");
    lower_qr();
    device.tap("CONFIRM");
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Manual");
    let payload = read_every_frame(device);
    note(PARTIAL_NOTE, &to_hex(&payload));
    decode_compact_kspt(&payload).expect("a signed transaction")
}

fn expect_signed_by(signed: &Transaction, signers: &[&[&str]]) {
    let input = &signed.inputs[0];
    let digest = calculate_sighash(signed, 0, SigHashType::All);
    let present: Vec<SchnorrSignature> = input
        .sigs
        .iter()
        .filter(|signature| signature.present)
        .map(|signature| SchnorrSignature {
            bytes: signature.signature,
        })
        .collect();
    assert_eq!(present.len(), signers.len());
    for words in signers {
        let key = signing_key_of(words);
        assert!(
            present
                .iter()
                .any(|signature| schnorr_verify(&key, &digest, signature).is_ok()),
            "a required cosigner did not sign"
        );
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn from_hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|start| u8::from_str_radix(&text[start..start + 2], 16).expect("hex"))
        .collect()
}

fn create_the_shared_wallet_and_sign_first() {
    assert_eq!(
        script_to_p2sh_address(&shared_redeem_script(), "kaspa"),
        known::SHARED_RECEIVE_0
    );
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    open_multisig_menu(&device);
    let own_key = read_own_key(&device, ABOUT_WORDS);
    add_all_cosigners(&device, &own_key);
    walk::expect_address(&device, known::SHARED_RECEIVE_0);
    device.tap_at(NEXT_ADDRESS_TAP.0, NEXT_ADDRESS_TAP.1);
    walk::expect_address(&device, known::SHARED_RECEIVE_1);
    device.tap_at(PREVIOUS_ADDRESS_TAP.0, PREVIOUS_ADDRESS_TAP.1);
    walk::expect_address(&device, known::SHARED_RECEIVE_0);
    save_the_descriptor(&device);
    assert_eq!(recall("shared-address-qr"), known::SHARED_RECEIVE_0);
    let signed = scan_and_sign(&device, &shared_transaction());
    expect_signed_by(&signed, &[ABOUT_WORDS]);
    discard_board();
}

fn save_the_shared_wallet() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    open_multisig_menu(&device);
    let own_key = read_own_key(&device, ABOUT_WORDS);
    add_all_cosigners(&device, &own_key);
    save_the_descriptor(&device);
}

fn expect_the_shared_wallet_saved() {
    let saved = app_state()
        .signing
        .multisig
        .store
        .configs
        .iter()
        .filter(|config| config.active)
        .count();
    assert_eq!(saved, 1, "the saved shared wallet did not come back");
}

fn sign_with_the_saved_shared_wallet() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    expect_the_shared_wallet_saved();
    let signed = scan_and_sign(&device, &shared_transaction());
    expect_signed_by(&signed, &[ABOUT_WORDS]);
}

/// A wrong PIN rewrites the device record, which must carry the saved wallet forward.
fn write_the_device_record_again() {
    let device = Device::boot();
    device.expect_text("UNLOCK PIN");
    device.enter_pin(WRONG_PIN);
    device.settle();
    device.expect_text("Invalid PIN");
    device.let_time_pass(WRONG_PIN_BACKOFF);
    device.enter_pin(PIN);
    device.expect_text("Scan QR");
    expect_the_shared_wallet_saved();
}

fn the_saved_shared_wallet_is_still_there() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    expect_the_shared_wallet_saved();
}

fn sign_second_on_another_device() {
    let device = Device::boot();
    walk::restore_words(&device, LEGAL_WORDS, "No Passphrase");
    walk::save_with_pin(&device, PIN);
    open_multisig_menu(&device);
    let own_key = read_own_key(&device, LEGAL_WORDS);
    add_all_cosigners(&device, &own_key);
    save_the_descriptor(&device);
    let signed = scan_and_sign(&device, &from_hex(&recall(PARTIAL_NOTE)));
    expect_signed_by(&signed, &[ABOUT_WORDS, LEGAL_WORDS]);
}
