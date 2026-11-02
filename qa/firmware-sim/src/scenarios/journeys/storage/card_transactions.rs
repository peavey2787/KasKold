//! Keys and transactions kept on a card, and what comes back from it.
use crate::harness::{app_state, card, Device};
use crate::scenarios::journeys::known::{self, PASSWORD, PIN};
use crate::scenarios::journeys::signing_flows::signing::read_every_frame;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use kaspa_portal::transaction::interchange::kspt::decode_compact_kspt;

const SAVED_TRANSACTION: &[u8; 11] = b"TX000001KSP";
const WRONG_PASSWORD: &str = "wrong4horse";

journey!(
    an_extended_key_on_a_card_brings_the_wallet_back:
    restore_the_known_wallet,
    save_the_extended_key,
    walk::factory_reset,
    restore_the_extended_key,
    the_extended_key_survives_a_reboot,
);
journey!(a_signed_transaction_saved_to_a_card_is_the_signed_one: restore_the_known_wallet, save_the_signed_transaction);
journey!(
    a_saved_transaction_can_be_encrypted_and_opened_again:
    restore_the_known_wallet,
    save_the_signed_transaction_encrypted,
    open_the_encrypted_transaction,
);
journey!(a_transaction_on_a_card_is_reviewed_before_signing: restore_the_known_wallet, sign_from_a_card);

fn save_the_extended_key() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    device.tap("Wallet");
    device.tap("Backup");
    device.tap("Advanced");
    device.tap("XPrv Backup");
    device.tap("Encrypt to SD");
    walk::press_full_ok(&device);
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("ADVANCED BACKUP");
    let file = card::read_file(b"XP000001KAS").expect("the key never reached the card");
    assert!(
        !walk::contains(&file, b"kprv"),
        "the key is readable on the card"
    );
}

fn restore_the_extended_key() {
    let device = Device::boot();
    device.tap("Restore Wallet");
    device.tap("SD");
    device.tap("XP000001.KAS");
    walk::enter_password(&device, WRONG_PASSWORD);
    device.wait_for_text("XP000001.KAS");
    device.tap("XP000001.KAS");
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("Save Securely on Device");
    walk::go_back(&device);
    device.expect_text("Raw Private Key");
    assert_eq!(app_state().wallet.seeds.seed_mgr.count(), 0);
    walk::go_back(&device);
    device.tap("SD");
    device.tap("XP000001.KAS");
    walk::enter_password(&device, PASSWORD);
    walk::protect_with_pin(&device, PIN);
}

fn the_extended_key_survives_a_reboot() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn sign_and_open_the_save_prompt(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::format_card(device);
    device.tap("Scan QR");
    crate::harness::hold_up_qr(&known::unsigned_transaction());
    device.wait_for_text("CONFIRM SEND");
    crate::harness::lower_qr();
    device.tap("CONFIRM");
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Auto Cycle");
    device.wait_for_qr();
    device.tap_at(160, 120);
    device.wait_for_text("SIGNED TX");
    device.tap("Save to SD");
    walk::press_full_ok(device);
    device.wait_for_text("ENCRYPT FILE?");
}

fn save_the_signed_transaction() {
    let device = Device::boot_with_card();
    sign_and_open_the_save_prompt(&device);
    device.tap("No");
    device.wait_for_text("Scan QR");
    let file = card::read_file(SAVED_TRANSACTION).expect("the transaction never reached the card");
    let saved = decode_compact_kspt(&file).expect("a signed transaction");
    assert_eq!(saved.outputs[0].value, 150_000_000);
    assert_eq!(saved.outputs[1].value, 49_000_000);
}

fn save_the_signed_transaction_encrypted() {
    let device = Device::boot_with_card();
    sign_and_open_the_save_prompt(&device);
    device.tap("Yes");
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("Scan QR");
    let file = card::read_file(SAVED_TRANSACTION).expect("the transaction never reached the card");
    assert!(
        decode_compact_kspt(&file).is_err(),
        "the transaction is readable on the card"
    );
}

fn open_the_encrypted_transaction() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    walk::open_wallet_menu(&device);
    device.tap("Recovery");
    device.tap("Transaction");
    device.tap("TX000001.KSP");
    walk::enter_password(&device, WRONG_PASSWORD);
    device.wait_for_text("TX000001.KSP");
    device.tap("TX000001.KSP");
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("CONFIRM SEND");
    device.expect_text("Send: 1.5 KAS");
}

fn sign_from_a_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    card::put_file(SAVED_TRANSACTION, &known::unsigned_transaction());
    walk::open_wallet_menu(&device);
    device.tap("Recovery");
    device.tap("Transaction");
    device.tap("TX000001.KSP");
    device.wait_for_text("CONFIRM SEND");
    device.expect_text("Send: 1.5 KAS");
    device.expect_text("Change: 0.49 KAS");
    device.tap("CONFIRM");
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Manual");
    let signed = decode_compact_kspt(&read_every_frame(&device)).expect("a signed transaction");
    assert_eq!(signed.outputs[0].value, 150_000_000);
}
