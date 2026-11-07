//! A card that has lived in a computer or a phone: a partition table, large clusters,
//! long file names, folders, a label and old deleted files.
use super::stego::{
    expect_the_backup_was_made, open_stego, set_the_portable_password, DESCRIPTOR, LOWER_CARD,
    NOISE_PHOTO, NO_HINT, OVERWRITE, PREVIEW_NEXT, TYPE_MANUALLY, UPPER_CARD,
};
use crate::harness::{card, Device};
use crate::scenarios::journeys::known::{self, PASSWORD, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use std::time::Duration;

const LONG_PHOTO_NAME: &str = "IMG_20240601_183000.jpg";
/// How that name fits its row, and the next photo the same camera took.
const LONG_PHOTO_ROW: &str = "IMG_20..3000.jpg";
const NEXT_PHOTO_NAME: &str = "IMG_20240601_190500.jpg";
const NEXT_PHOTO_ROW: &str = "IMG_20..0500.jpg";
const SHORT_NEXT_PHOTO_NAME: &[u8; 11] = b"IMG_20~2JPG";
const SHORT_PHOTO_NAME: &[u8; 11] = b"IMG_20~1JPG";
const LONG_NOTE_NAME: &str = "groceries and other notes.txt";
const SHORT_NOTE_NAME: &[u8; 11] = b"GROCER~1TXT";
const BYSTANDER: &[u8] = b"the shopping list stays exactly as it was";
const LONG_TX_NAME: &str = "rent payment november.ksp";
const SHORT_TX_NAME: &[u8; 11] = b"RENTPA~1KSP";
/// The list shows the name a computer keeps beside the long one.
const TX_ROW: &str = "RENTPA~1.KSP";
const DELETE_ICON: (u16, u16) = (250, 67);
const DESCRIPTION_FILE: &[u8; 11] = b"CAPTION TXT";
const LOAD_TEXT_FILE: &str = "Load .TXT from SD";
const DELETE_BUTTON: (u16, u16) = (230, 205);
const DELETE_HOLD: Duration = Duration::from_secs(5);

journey!(
    a_phone_card_keeps_a_phrase_in_its_scattered_photo:
    restore_the_known_wallet,
    a_computer_prepares_the_card,
    hide_in_the_long_named_photo,
    walk::factory_reset,
    restore_from_the_long_named_photo,
);
journey!(
    a_transaction_with_a_long_name_is_signed_from_a_computer_card:
    restore_the_known_wallet,
    a_computer_prepares_the_card,
    sign_the_long_named_transaction,
);
journey!(
    deleting_a_long_named_file_leaves_no_trace_of_its_name_or_contents:
    restore_the_known_wallet,
    a_computer_prepares_the_card,
    delete_the_long_named_transaction,
);
journey!(
    a_card_full_of_photos_says_it_lists_only_some:
    restore_the_known_wallet,
    a_computer_fills_the_card_with_photos,
    open_the_crowded_picker,
);
journey!(
    a_full_list_of_transactions_says_it_lists_only_some:
    restore_the_known_wallet,
    a_computer_fills_the_card_with_transactions,
    open_the_crowded_list,
);
journey!(
    a_description_kept_in_a_text_file_hides_and_restores_a_phrase:
    restore_the_known_wallet,
    a_computer_leaves_a_photo_and_its_description,
    hide_with_the_description_from_the_card,
    walk::factory_reset,
    restore_with_the_description_from_the_card,
);
journey!(
    a_text_file_that_cannot_be_a_description_says_why:
    restore_the_known_wallet,
    a_computer_leaves_a_photo_and_unusable_descriptions,
    choose_the_unusable_descriptions,
);
journey!(
    a_full_list_of_text_files_says_it_lists_only_some:
    restore_the_known_wallet,
    a_computer_fills_the_card_with_notes,
    open_the_crowded_note_list,
);

fn a_computer_prepares_the_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    card::format_like_a_computer();
    card::put_volume_label(b"NIKON D3500");
    card::put_folder("System Volume Information", b"SYSTEM~1   ");
    card::put_deleted_file("an old holiday video.mp4", b"ANOLDH~1MP4");
    card::put_long_named_file(LONG_NOTE_NAME, SHORT_NOTE_NAME, BYSTANDER);
    card::put_scattered_file(LONG_PHOTO_NAME, SHORT_PHOTO_NAME, NOISE_PHOTO);
    card::put_long_named_file(NEXT_PHOTO_NAME, SHORT_NEXT_PHOTO_NAME, NOISE_PHOTO);
    card::put_long_named_file(LONG_TX_NAME, SHORT_TX_NAME, &known::unsigned_transaction());
    card::expect_a_sound_volume();
}

fn hide_in_the_long_named_photo() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    open_stego(&device);
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    device.expect_text(NEXT_PHOTO_ROW);
    device.tap(LONG_PHOTO_ROW);
    device.tap(TYPE_MANUALLY);
    device.type_text(DESCRIPTOR);
    walk::press_full_ok(&device);
    device.tap_at(PREVIEW_NEXT.0, PREVIEW_NEXT.1);
    device.wait_for_text("HIDE A HINT?");
    device.tap_at(NO_HINT.0, NO_HINT.1);
    set_the_portable_password(&device);
    device.expect_text(LONG_PHOTO_NAME);
    device.tap_at(OVERWRITE.0, OVERWRITE.1);
    expect_the_backup_was_made(&device);
    card::expect_a_sound_volume();
    assert_eq!(
        card::read_file(SHORT_NOTE_NAME).as_deref(),
        Some(BYSTANDER),
        "another file on the card changed"
    );
}

fn restore_from_the_long_named_photo() {
    let device = Device::boot_with_card();
    device.tap("Restore Wallet");
    device.tap("Advanced");
    device.tap("Steganographic");
    device.tap(LONG_PHOTO_ROW);
    device.tap(TYPE_MANUALLY);
    device.type_text(DESCRIPTOR);
    walk::press_full_ok(&device);
    device.settle();
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("No Passphrase");
    device.tap("No Passphrase");
    walk::save_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn open_the_transaction_list(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::open_wallet_menu(device);
    device.tap("Recovery");
    device.tap("Transaction");
    device.wait_for_text(TX_ROW);
}

fn sign_the_long_named_transaction() {
    let device = Device::boot_with_card();
    open_the_transaction_list(&device);
    device.tap(TX_ROW);
    device.wait_for_text("CONFIRM SEND");
    device.expect_text("Send: 1.5 KAS");
}

fn delete_the_long_named_transaction() {
    let device = Device::boot_with_card();
    let held = card::blocks_of(SHORT_TX_NAME);
    open_the_transaction_list(&device);
    device.tap_at(DELETE_ICON.0, DELETE_ICON.1);
    device.wait_for_text("DELETE BACKUP?");
    device.expect_text(TX_ROW);
    device.tap_at(DELETE_BUTTON.0, DELETE_BUTTON.1);
    device.wait_for_text("HOLD 4s");
    device.hold("HOLD 4s", DELETE_HOLD);
    device.settle();
    assert!(card::read_file(SHORT_TX_NAME).is_none());
    card::expect_blank(&held);
    assert_eq!(
        card::read_file(SHORT_NOTE_NAME).as_deref(),
        Some(BYSTANDER),
        "another file on the card changed"
    );
    card::expect_a_sound_volume();
}

fn a_computer_fills_the_card_with_photos() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    card::format_like_a_computer();
    for number in 0..=8u8 {
        let mut name = *b"PHOTO0  JPG";
        name[5] += number;
        card::put_file(&name, NOISE_PHOTO);
    }
}

fn open_the_crowded_picker() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    open_stego(&device);
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    device.wait_for_text("SELECT JPEG");
    device.expect_was_shown("Listing the first 8 photos");
}

fn a_computer_fills_the_card_with_transactions() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    card::format_like_a_computer();
    for number in 0..=8u8 {
        let mut name = *b"TX00000 KSP";
        name[6] += number;
        card::put_file(&name, &known::unsigned_transaction());
    }
}

fn open_the_crowded_list() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::open_wallet_menu(&device);
    device.tap("Recovery");
    device.tap("Transaction");
    device.wait_for_text("SELECT BACKUP");
    device.expect_was_shown("Listing the first 8 files");
}

fn a_computer_fills_the_card_with_notes() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    card::format_like_a_computer();
    card::put_file(b"PHOTO   JPG", NOISE_PHOTO);
    for number in 0..=8u8 {
        let mut name = *b"NOTE0   TXT";
        name[4] += number;
        card::put_file(&name, BYSTANDER);
    }
}

fn open_the_crowded_note_list() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    open_stego(&device);
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    device.wait_for_text("SELECT JPEG");
    device.tap("PHOTO");
    device.tap(LOAD_TEXT_FILE);
    device.settle();
    device.expect_was_shown("Listing the first 8 files");
}

fn a_computer_leaves_a_photo_and_its_description() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    card::format_like_a_computer();
    card::put_file(b"PHOTO   JPG", NOISE_PHOTO);
    card::put_file(DESCRIPTION_FILE, format!("{DESCRIPTOR}\r\n").as_bytes());
}

fn hide_with_the_description_from_the_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    open_stego(&device);
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    device.tap("PHOTO");
    device.tap(LOAD_TEXT_FILE);
    device.tap("CAPTION");
    device.wait_for_text(DESCRIPTOR);
    device.tap_at(PREVIEW_NEXT.0, PREVIEW_NEXT.1);
    device.wait_for_text("HIDE A HINT?");
    device.tap_at(NO_HINT.0, NO_HINT.1);
    set_the_portable_password(&device);
    device.tap_at(OVERWRITE.0, OVERWRITE.1);
    expect_the_backup_was_made(&device);
    card::expect_a_sound_volume();
}

fn restore_with_the_description_from_the_card() {
    let device = Device::boot_with_card();
    device.tap("Restore Wallet");
    device.tap("Advanced");
    device.tap("Steganographic");
    device.tap("PHOTO");
    device.tap(LOAD_TEXT_FILE);
    device.tap("CAPTION");
    device.wait_for_text(DESCRIPTOR);
    walk::press_full_ok(&device);
    device.settle();
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("No Passphrase");
    device.tap("No Passphrase");
    walk::save_with_pin(&device, PIN);
    walk::open_receive(&device);
    walk::expect_address(&device, known::RECEIVE_0);
}

fn a_computer_leaves_a_photo_and_unusable_descriptions() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    card::format_like_a_computer();
    card::put_file(b"PHOTO   JPG", NOISE_PHOTO);
    card::put_file(b"BINARY  TXT", &[0xFF, 0xFE, 0x01, 0x02]);
    card::put_file(b"SPACES  TXT", b"  \r\n");
}

fn choose_the_unusable_descriptions() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    open_stego(&device);
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    device.tap("PHOTO");
    device.tap(LOAD_TEXT_FILE);
    device.tap("BINARY");
    device.settle();
    device.expect_was_shown("Description must be readable");
    device.wait_for_text("SELECT TXT");
    device.tap("SPACES");
    device.settle();
    device.expect_was_shown("Text file is empty");
    device.wait_for_text("SELECT TXT");
}
