//! A transaction carried on a card instead of a camera: read, reviewed, signed and saved.
use crate::harness::{app_state, card, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::{self, PASSWORD, PIN};
use crate::scenarios::journeys::signing_flows::signing;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use kaspa_portal::transaction::interchange::kspt::decode_compact_kspt;
use offline_signer::crypto::container_framing::TRANSPORT_CURRENT_MAGIC;

const SOURCE_FILE: &[u8; 11] = b"PAYMENT KSP";
const SOURCE_NAME: &str = "PAYMENT.KSP";
const SAVED_FILE: &[u8; 11] = b"TX000001KSP";
const SAVED_NAME: &str = "TX000001.KSP";
const WRONG_PASSWORD: &str = "wrong4horse";

journey!(a_transaction_on_a_card_is_signed_and_saved_back: restore_the_known_wallet, sign_a_plain_file);
journey!(a_signed_transaction_saved_encrypted_opens_only_with_its_password: restore_the_known_wallet, save_encrypted);
journey!(a_card_without_transactions_says_so: restore_the_known_wallet, no_files_on_the_card);
journey!(a_damaged_file_never_reaches_review: restore_the_known_wallet, refuse_a_damaged_file);

fn prepare_card(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::format_card(device);
}

fn open_transaction_list(device: &Device) {
    device.tap("Wallet");
    device.tap("Recovery");
    device.tap("Transaction");
}

fn review_and_sign(device: &Device) {
    open_transaction_list(device);
    device.tap(SOURCE_NAME);
    device.wait_for_text("CONFIRM SEND");
    device.expect_text("Send: 1.5 KAS");
    device.tap("CONFIRM");
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Manual");
    signing::read_every_frame(device);
    device.wait_for_text("Save to SD");
}

fn save_under_its_offered_name(device: &Device) {
    device.tap("Save to SD");
    walk::press_full_ok(device);
    device.wait_for_text("ENCRYPT FILE?");
}

fn sign_a_plain_file() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(SOURCE_FILE, &known::unsigned_transaction());
    review_and_sign(&device);
    save_under_its_offered_name(&device);
    device.tap("No");
    device.wait_for_text("Scan QR");
    let saved = card::read_file(SAVED_FILE).expect("the signed file on the card");
    let signed = decode_compact_kspt(&saved).expect("a signed transaction");
    assert!(
        !signed.inputs[0].sigs.is_empty(),
        "the saved file is unsigned"
    );
    assert_eq!(signed.outputs[0].value, 150_000_000);
}

fn save_encrypted() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(SOURCE_FILE, &known::unsigned_transaction());
    review_and_sign(&device);
    save_under_its_offered_name(&device);
    device.tap("Yes");
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("Scan QR");
    let saved = card::read_file(SAVED_FILE).expect("the encrypted file on the card");
    assert!(
        decode_compact_kspt(&saved).is_err(),
        "an encrypted file must not read as a transaction"
    );
    assert!(
        saved.starts_with(&TRANSPORT_CURRENT_MAGIC),
        "the file is not encrypted"
    );
    open_transaction_list(&device);
    device.tap(SAVED_NAME);
    walk::enter_password(&device, WRONG_PASSWORD);
    assert_eq!(app_state().navigation.app.state, AppState::SdKsptFileList);
    device.expect_was_shown("Wrong password");
    device.tap(SAVED_NAME);
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("CONFIRM SEND");
}

fn no_files_on_the_card() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    open_transaction_list(&device);
    device.expect_was_shown("No .KSP files found");
}

fn refuse_a_damaged_file() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(SOURCE_FILE, b"this is no transaction at all");
    open_transaction_list(&device);
    device.tap(SOURCE_NAME);
    device.settle();
    device.expect_no_transaction_review();
}
