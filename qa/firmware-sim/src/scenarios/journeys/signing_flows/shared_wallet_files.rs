//! A shared wallet's description and address leave for a card and come back from it.
use crate::harness::{app_state, card, hold_up_qr, lower_qr, Device};
use crate::scenarios::journeys::known::{
    self, ABOUT_WORDS, LEGAL_WORDS, PASSWORD, PIN, TWENTY_FOUR_WORDS,
};
use crate::scenarios::journeys::signing_flows::multisig::{
    add_all_cosigners, open_multisig_menu, read_own_key, shared_config, ADDRESS_TAP, SAVE_YES_TAP,
};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use kaskold_protocol::wire::multisig_descriptor::looks_like_descriptor;

const DESCRIPTOR_ROW: (u16, u16) = (160, 95);
const ADDRESS_FILE: &[u8; 11] = b"MS000001TXT";
const DESCRIPTOR_FILE: &[u8; 11] = b"MD000001TXT";
const DESCRIPTOR_LABEL: &str = "2-of-3 45' S1/C0 multisig";
const WRONG_PASSWORD: &str = "wrong4horse";
const LETTER_WORDS: &[&str] = &[
    "letter", "advice", "cage", "absurd", "amount", "doctor", "acoustic", "avoid", "letter",
    "advice", "cage", "above",
];
const ZOO_WORDS: &[&str] = &[
    "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "wrong",
];

journey!(
    a_shared_wallet_survives_on_a_card_after_being_deleted:
    restore_the_known_wallet,
    save_the_shared_wallet_and_its_address,
    back_up_view_and_delete_the_description,
    restore_the_description_from_the_card,
    the_kept_description_is_still_there,
);

journey!(
    encrypted_shared_wallet_files_open_with_their_password:
    restore_the_known_wallet,
    save_both_files_encrypted,
    open_the_encrypted_address,
    open_the_encrypted_description,
);

journey!(
    a_scanned_description_waits_for_its_owner:
    restore_the_known_wallet,
    scan_and_keep_a_description,
    the_kept_description_is_still_there,
);

journey!(
    a_description_that_leaves_this_wallet_out_cannot_be_kept:
    restore_the_known_wallet,
    open_a_description_of_other_wallets,
);

journey!(a_broken_description_is_refused: restore_the_known_wallet, refuse_a_broken_description);
journey!(the_largest_description_fits_on_a_card: restore_the_known_wallet, open_the_largest_description);

fn saved_descriptions() -> usize {
    app_state().signing.multisig.store.wallets().count()
}

fn open_descriptors(device: &Device) {
    open_multisig_menu(device);
    device.tap("Descriptors");
}

fn descriptor_of(wallets: &[&[&str]]) -> Vec<u8> {
    let mut text = [0u8; 1024];
    let length = shared_config(wallets)
        .write_descriptor(&mut text)
        .expect("a description");
    text[..length].to_vec()
}

fn prepare_card(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    walk::format_card(device);
}

/// Ends with the device asking whether to encrypt the address file.
fn create_the_shared_wallet(device: &Device) {
    open_multisig_menu(device);
    let own_key = read_own_key(device, ABOUT_WORDS);
    add_all_cosigners(device, &own_key);
    device.tap_at(ADDRESS_TAP.0, ADDRESS_TAP.1);
    device.wait_for_qr();
    device.tap_at(ADDRESS_TAP.0, ADDRESS_TAP.1);
    device.wait_for_text("SAVE MULTISIG?");
    device.tap_at(SAVE_YES_TAP.0, SAVE_YES_TAP.1);
    device.wait_for_text("SAVE ADDRESS?");
    device.tap_at(SAVE_YES_TAP.0, SAVE_YES_TAP.1);
    walk::press_full_ok(device);
    device.wait_for_text("ENCRYPT FILE?");
}

fn save_the_shared_wallet_and_its_address() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    device.tap("Settings");
    let clean_corner = device.screen().back_corner();
    walk::go_home(&device);
    create_the_shared_wallet(&device);
    device.tap("No");
    device.settle();
    let file = card::read_file(ADDRESS_FILE).expect("the address never reached the card");
    assert_eq!(file, known::SHARED_RECEIVE_0.as_bytes());
    assert_eq!(saved_descriptions(), 1);
    device.expect_text(DESCRIPTOR_LABEL);
    assert_eq!(
        device.screen().back_corner(),
        clean_corner,
        "an earlier screen shows through the back icon"
    );
}

fn back_up_view_and_delete_the_description() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    open_descriptors(&device);
    device.tap("View");
    device.tap_at(DESCRIPTOR_ROW.0, DESCRIPTOR_ROW.1);
    device.wait_for_text("DESCRIPTOR");
    device.expect_text(DESCRIPTOR_LABEL);
    walk::go_back(&device);
    device.wait_for_text("MULTISIG WALLET");
    walk::go_back(&device);
    device.wait_for_text("Create Multisig");
    device.tap("Descriptors");
    device.tap("Backup to SD");
    device.tap_at(DESCRIPTOR_ROW.0, DESCRIPTOR_ROW.1);
    device.settle();
    walk::press_full_ok(&device);
    device.wait_for_text("ENCRYPT FILE?");
    device.tap("No");
    device.settle();
    let file = card::read_file(DESCRIPTOR_FILE).expect("the description never reached the card");
    assert!(looks_like_descriptor(&file));
    walk::go_home(&device);
    open_descriptors(&device);
    device.tap("Delete");
    device.tap_at(DESCRIPTOR_ROW.0, DESCRIPTOR_ROW.1);
    device.settle();
    device.expect_was_shown("Descriptor deleted");
    assert_eq!(saved_descriptions(), 0);
}

fn keep_the_description(device: &Device) {
    device.tap("SAVE");
    device.settle();
    device.expect_was_shown("Descriptor saved");
    device.expect_text("SD CARD");
    assert_eq!(saved_descriptions(), 1);
}

fn restore_the_description_from_the_card() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    assert_eq!(saved_descriptions(), 0);
    open_descriptors(&device);
    device.tap("Restore from SD");
    device.tap("MD000001.TXT");
    device.expect_was_shown("Descriptor loaded!");
    device.wait_for_text("DESCRIPTOR");
    device.expect_text(DESCRIPTOR_LABEL);
    assert_eq!(
        saved_descriptions(),
        0,
        "a file is trusted before it is reviewed"
    );
    keep_the_description(&device);
}

fn the_kept_description_is_still_there() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    assert_eq!(saved_descriptions(), 1);
    open_descriptors(&device);
    device.tap("View");
    device.tap_at(DESCRIPTOR_ROW.0, DESCRIPTOR_ROW.1);
    device.wait_for_text("DESCRIPTOR");
    device.expect_text(DESCRIPTOR_LABEL);
}

fn save_both_files_encrypted() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    create_the_shared_wallet(&device);
    device.tap("Yes");
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("DESCRIPTOR");
    let address = card::read_file(ADDRESS_FILE).expect("the address never reached the card");
    assert!(
        !address.starts_with(b"kaspa:"),
        "the address is readable on the card"
    );
    device.tap("SD CARD");
    walk::press_full_ok(&device);
    device.wait_for_text("ENCRYPT FILE?");
    device.tap("Yes");
    walk::enter_password(&device, PASSWORD);
    device.settle();
    let description =
        card::read_file(DESCRIPTOR_FILE).expect("the description never reached the card");
    assert!(
        !looks_like_descriptor(&description),
        "the description is readable on the card"
    );
}

fn open_the_encrypted_address() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    walk::open_wallet_menu(&device);
    device.tap("Recovery");
    device.tap("Multisig Address");
    device.tap("MS000001.TXT");
    walk::enter_password(&device, WRONG_PASSWORD);
    device.wait_for_text("MS000001.TXT");
    device.tap("MS000001.TXT");
    walk::enter_password(&device, PASSWORD);
    walk::expect_address(&device, known::SHARED_RECEIVE_0);
    device.tap_at(ADDRESS_TAP.0, ADDRESS_TAP.1);
    assert_eq!(device.wait_for_qr(), known::SHARED_RECEIVE_0.as_bytes());
}

fn open_the_encrypted_description() {
    let device = Device::boot_with_card();
    walk::unlock_with_pin(&device, PIN);
    let saved_before = saved_descriptions();
    open_descriptors(&device);
    device.tap("Restore from SD");
    device.tap("MD000001.TXT");
    walk::enter_password(&device, WRONG_PASSWORD);
    device.wait_for_text("MD000001.TXT");
    device.tap("MD000001.TXT");
    walk::enter_password(&device, PASSWORD);
    device.wait_for_text("DESCRIPTOR");
    device.expect_text(DESCRIPTOR_LABEL);
    assert_eq!(saved_descriptions(), saved_before);
}

fn scan_and_keep_a_description() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Scan QR");
    hold_up_qr(&descriptor_of(&[
        ABOUT_WORDS,
        LEGAL_WORDS,
        &TWENTY_FOUR_WORDS,
    ]));
    device.wait_for_text("DESCRIPTOR");
    lower_qr();
    device.expect_text(DESCRIPTOR_LABEL);
    assert_eq!(
        saved_descriptions(),
        0,
        "a scanned code is trusted before it is reviewed"
    );
    keep_the_description(&device);
}

fn open_a_description_of_other_wallets() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(
        b"MDOTHERSTXT",
        &descriptor_of(&[LEGAL_WORDS, &TWENTY_FOUR_WORDS]),
    );
    open_descriptors(&device);
    device.tap("Restore from SD");
    device.tap("MDOTHERS.TXT");
    device.expect_was_shown("Descriptor loaded!");
    device.wait_for_text("DESCRIPTOR");
    device.expect_text("SD CARD");
    device.expect_absent("SAVE");
    walk::go_back(&device);
    device.wait_for_text("Restore from SD");
    assert_eq!(saved_descriptions(), 0);
}

fn refuse_a_broken_description() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(b"MDBROKENTXT", b"multi_hd45(2,bad)");
    open_descriptors(&device);
    device.tap("Restore from SD");
    device.tap("MDBROKEN.TXT");
    device.settle();
    device.expect_was_shown("Bad descriptor");
    device.expect_absent("SAVE");
    assert_eq!(saved_descriptions(), 0);
}

fn open_the_largest_description() {
    let device = Device::boot_with_card();
    prepare_card(&device);
    card::put_file(
        b"MDLARGESTXT",
        &descriptor_of(&[
            ABOUT_WORDS,
            LEGAL_WORDS,
            &TWENTY_FOUR_WORDS,
            LETTER_WORDS,
            ZOO_WORDS,
        ]),
    );
    open_descriptors(&device);
    device.tap("Restore from SD");
    device.tap("MDLARGES.TXT");
    device.wait_for_text("DESCRIPTOR");
    keep_the_description(&device);
}
