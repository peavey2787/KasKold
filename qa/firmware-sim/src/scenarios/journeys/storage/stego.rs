//! A recovery phrase hidden in a photo: it comes back only with the right words,
//! and the photo still opens as a photo.
use crate::harness::{app_state, card, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::{self, ABOUT_WORDS, PASSWORD, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use esp_hal::board::{efuse, sd};

pub(super) const NOISE_PHOTO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/kaskold-hardware/src/runtime/workflow_tests/fixtures/stego_noise.jpg"
));
const FLAT_PHOTO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/kaskold-hardware/src/runtime/workflow_tests/fixtures/stego_flat.jpg"
));
pub(super) const PHOTO_FILE: &[u8; 11] = b"PHOTO1  JPG";
const PHOTO_NAME: &str = "PHOTO1.JPG";
pub(super) const DESCRIPTOR: &str = "sunset over the bay";
const WRONG_DESCRIPTOR: &str = "sunrise over the bay";
const WRONG_PASSWORD: &str = "wrong4horse";
const JPEG_START: [u8; 2] = [0xFF, 0xD8];
/// The upper and lower cards of the carrier and security screens.
pub(super) const UPPER_CARD: (u16, u16) = (160, 90);
pub(super) const LOWER_CARD: (u16, u16) = (160, 170);
const TYPE_MANUALLY: &str = "Type manually";
/// NEXT on the descriptor preview, and NO then YES on the hint question.
const PREVIEW_NEXT: (u16, u16) = (230, 205);
const NO_HINT: (u16, u16) = (80, 195);
const WANT_HINT: (u16, u16) = (230, 195);
const FIRST_PRESET_HINT: (u16, u16) = (100, 80);
/// The title of the same screen also says OVERWRITE.
const OVERWRITE: (u16, u16) = (235, 205);
const BACKUP_MADE: &str = "Stego Backup Created";
const HINT_FOUND: &str = "A recovery hint was found:";
const FIRST_PRESET_HINT_TEXT: &str = "My favorite place I lived?";
const HINT_CONTINUE: &str = "Tap to continue";

journey!(
    a_phrase_hidden_in_a_photo_descriptor_comes_back_on_a_new_device:
    restore_the_known_wallet,
    hide_portably_in_the_descriptor,
    walk::factory_reset,
    restore_portably_from_the_photo,
);
journey!(
    a_phrase_hidden_in_the_picture_itself_comes_back_with_its_password:
    restore_the_known_wallet,
    hide_portably_in_the_picture,
    walk::factory_reset,
    restore_portably_from_the_photo,
);
journey!(
    a_device_bound_photo_opens_on_its_device_and_shows_its_hint:
    restore_the_known_wallet,
    hide_bound_to_this_device,
    walk::factory_reset,
    restore_on_the_same_device,
);
journey!(
    the_answer_to_a_photo_hint_opens_the_passphrase_wallet:
    restore_the_known_wallet,
    hide_bound_to_this_device,
    walk::factory_reset,
    restore_answering_the_hint,
);
journey!(
    a_device_bound_photo_never_opens_on_another_device:
    restore_the_known_wallet,
    hide_bound_to_this_device,
    walk::factory_reset,
    refuse_on_another_device,
);
journey!(
    a_photo_too_plain_to_hide_anything_is_left_untouched:
    restore_the_known_wallet,
    refuse_a_plain_photo,
);
journey!(
    a_card_without_photos_cannot_hide_a_phrase:
    restore_the_known_wallet,
    refuse_a_card_without_photos,
);
journey!(
    a_restore_from_a_photo_needs_a_card_with_photos:
    refuse_a_restore_without_a_card,
    refuse_a_restore_without_photos,
);

fn open_stego(device: &Device) {
    device.tap("Wallet");
    device.tap("Backup");
    device.tap("Advanced");
    device.tap("Steganographic");
    device.wait_for_text("BACKUP CARRIER");
}

fn prepare_card_with(device: &Device, photo: &[u8]) {
    walk::unlock_with_pin(device, PIN);
    walk::format_card(device);
    card::put_file(PHOTO_FILE, photo);
}

fn describe_the_photo(device: &Device) {
    device.tap(PHOTO_NAME);
    device.tap(TYPE_MANUALLY);
    device.type_text(DESCRIPTOR);
    walk::press_full_ok(device);
    device.tap_at(PREVIEW_NEXT.0, PREVIEW_NEXT.1);
    device.wait_for_text("HIDE A HINT?");
}

fn set_the_portable_password(device: &Device) {
    walk::enter_password(device, PASSWORD);
    walk::enter_password(device, PASSWORD);
    device.wait_for_text("CONFIRM OVERWRITE");
}

pub(super) fn hide_portably(photo: &[u8], carrier: (u16, u16)) {
    let device = Device::boot_with_card();
    prepare_card_with(&device, photo);
    open_stego(&device);
    device.tap_at(carrier.0, carrier.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    describe_the_photo(&device);
    device.tap_at(NO_HINT.0, NO_HINT.1);
    set_the_portable_password(&device);
    device.tap_at(OVERWRITE.0, OVERWRITE.1);
    expect_the_photo_holds_a_secret(&device);
}

fn expect_the_photo_holds_a_secret(device: &Device) {
    device.wait_for_text(BACKUP_MADE);
    assert_eq!(app_state().navigation.app.state, AppState::StegoResult);
    let photo = card::read_file(PHOTO_FILE).expect("the photo is still on the card");
    assert!(
        photo.starts_with(&JPEG_START),
        "the photo stopped being a JPEG"
    );
    for word in ABOUT_WORDS {
        assert!(
            !walk::contains(&photo, word.as_bytes()),
            "{word:?} is readable in the photo"
        );
    }
}

fn hide_portably_in_the_descriptor() {
    hide_portably(NOISE_PHOTO, UPPER_CARD);
}

fn hide_portably_in_the_picture() {
    hide_portably(NOISE_PHOTO, LOWER_CARD);
}

fn hide_bound_to_this_device() {
    let device = Device::boot_with_card();
    prepare_card_with(&device, NOISE_PHOTO);
    open_stego(&device);
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    describe_the_photo(&device);
    device.tap_at(WANT_HINT.0, WANT_HINT.1);
    device.tap_at(FIRST_PRESET_HINT.0, FIRST_PRESET_HINT.1);
    device.wait_for_text("Hint: HIDDEN");
    device.tap_at(OVERWRITE.0, OVERWRITE.1);
    expect_the_photo_holds_a_secret(&device);
}

fn open_the_photo(device: &Device, descriptor: &str) {
    open_the_restore_picker(device);
    device.tap(PHOTO_NAME);
    device.tap(TYPE_MANUALLY);
    device.type_text(descriptor);
    walk::press_full_ok(device);
    device.settle();
}

fn finish_the_restore(device: &Device) {
    device.wait_for_text("No Passphrase");
    device.tap("No Passphrase");
    walk::save_with_pin(device, PIN);
    walk::open_receive(device);
    walk::expect_address(device, known::RECEIVE_0);
}

pub(super) fn restore_portably_from_the_photo() {
    let device = Device::boot();
    open_the_photo(&device, DESCRIPTOR);
    walk::enter_password(&device, WRONG_PASSWORD);
    expect_the_password_refused(&device);
    walk::enter_password(&device, PASSWORD);
    finish_the_restore(&device);
}

fn expect_the_password_refused(device: &Device) {
    device.expect_was_shown("Wrong password or damaged backup");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::StegoImportPortablePassword,
        "a wrong password may be tried again"
    );
    assert!(
        !app_state().wallet.seeds.seed_loaded,
        "a wrong password opened the photo"
    );
}

fn reveal_the_hint(device: &Device) {
    open_the_photo(device, DESCRIPTOR);
    device.wait_for_text(HINT_FOUND);
    device.expect_text(FIRST_PRESET_HINT_TEXT);
}

fn finish_with_the_saved_wallet(device: &Device, address: &str) {
    walk::save_with_pin(device, PIN);
    walk::open_receive(device);
    walk::expect_address(device, address);
}

fn restore_on_the_same_device() {
    let device = Device::boot();
    reveal_the_hint(&device);
    walk::go_back(&device);
    finish_with_the_saved_wallet(&device, known::RECEIVE_0);
}

fn restore_answering_the_hint() {
    let device = Device::boot();
    reveal_the_hint(&device);
    device.tap(HINT_CONTINUE);
    device.type_text(known::PASSPHRASE);
    walk::press_full_ok(&device);
    finish_with_the_saved_wallet(&device, known::PASSPHRASE_RECEIVE_0);
}

fn refuse_on_another_device() {
    efuse::become_another_device();
    let device = Device::boot();
    open_the_photo(&device, DESCRIPTOR);
    assert_eq!(
        app_state().navigation.app.state,
        AppState::StegoImportPortablePassword,
        "only a password could still be tried here"
    );
    walk::enter_password(&device, PASSWORD);
    expect_the_password_refused(&device);
    walk::go_back(&device);
    device.tap(TYPE_MANUALLY);
    device.type_text(WRONG_DESCRIPTOR);
    walk::press_full_ok(&device);
    device.settle();
    assert_eq!(
        app_state().navigation.app.state,
        AppState::StegoImportPortablePassword,
        "a wrong description must not reveal that the photo holds nothing for it"
    );
    walk::enter_password(&device, PASSWORD);
    expect_the_password_refused(&device);
    assert!(!app_state().wallet.seeds.seed_loaded);
}

fn refuse_a_plain_photo() {
    let device = Device::boot_with_card();
    prepare_card_with(&device, FLAT_PHOTO);
    open_stego(&device);
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(LOWER_CARD.0, LOWER_CARD.1);
    describe_the_photo(&device);
    device.tap_at(NO_HINT.0, NO_HINT.1);
    set_the_portable_password(&device);
    device.tap_at(OVERWRITE.0, OVERWRITE.1);
    device.wait_for_text("ADVANCED BACKUP");
    device.expect_was_shown("Photo has insufficient capacity");
    assert_eq!(
        card::read_file(PHOTO_FILE).expect("the photo is still on the card"),
        FLAT_PHOTO,
        "a refused photo must stay as it was"
    );
}

fn refuse_a_card_without_photos() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::format_card(&device);
    open_stego(&device);
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.wait_for_text("BACKUP SECURITY");
    device.tap_at(UPPER_CARD.0, UPPER_CARD.1);
    device.expect_was_shown("No .JPG files on SD");
    device.wait_for_text("BACKUP SECURITY");
}

fn open_the_restore_picker(device: &Device) {
    device.tap("Restore Wallet");
    device.tap("Advanced");
    device.tap("Steganographic");
}

fn refuse_a_restore_without_a_card() {
    sd::set_present(false);
    let device = Device::boot();
    open_the_restore_picker(&device);
    device.expect_was_shown("Insert SD card and restart");
    device.expect_text("Steganographic");
}

fn refuse_a_restore_without_photos() {
    let device = Device::boot_with_card();
    open_the_restore_picker(&device);
    device.expect_was_shown("No .JPG files on SD");
    device.expect_text("Steganographic");
}
