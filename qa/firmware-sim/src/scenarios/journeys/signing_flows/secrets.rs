//! A secret committed to now and opened later: only this wallet can read it back.
use crate::harness::{app_state, hold_up_qr, lower_qr, Device};
use crate::runtime::input::AppState;
use crate::scenarios::journeys::known::PIN;
use crate::scenarios::journeys::signing_flows::covenant::hex;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};

const SECRET: &str = "the vault code is 4711";
/// How the screen wraps it.
const SECRET_FIRST_LINE: &str = "the vault code is";
const SECRET_SECOND_LINE: &str = "4711";
const HASH_BYTES: usize = 32;

journey!(
    a_committed_secret_comes_back_only_from_its_own_ciphertext:
    restore_the_known_wallet,
    commit_and_open_a_secret,
);
journey!(a_scan_that_is_no_ciphertext_opens_nothing: restore_the_known_wallet, refuse_what_is_no_ciphertext);

fn open_wallet_advanced(device: &Device, tool: &str) {
    device.tap("Wallet");
    walk::next_menu_page(device);
    device.tap("Advanced");
    device.tap(tool);
}

fn commit_a_secret(device: &Device) -> Vec<u8> {
    open_wallet_advanced(device, "Commit Secret");
    device.type_text(SECRET);
    walk::press_full_ok(device);
    device.wait_for_text("COMMIT SECRET");
    device.tap("ENCRYPT & EXPORT");
    device.wait_for_text("COMMITTED");
    device.expect_text("Secret encrypted (ECIES)");
    device.tap("SHOW QR");
    let shown = device.wait_for_qr();
    assert!(
        !walk::contains(&shown, SECRET.as_bytes()),
        "the secret is readable in its own commitment"
    );
    shown
}

fn leave_the_commitment(device: &Device) {
    device.tap_at(160, 120);
    device.wait_for_text("COMMITTED");
    walk::go_back(device);
    walk::go_home(device);
}

fn hold_up_ciphertext(device: &Device, ciphertext: &[u8]) {
    open_wallet_advanced(device, "Decrypt Secret");
    hold_up_qr(hex(ciphertext).as_bytes());
    device.settle();
    lower_qr();
}

fn commit_and_open_a_secret() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let shown = commit_a_secret(&device);
    leave_the_commitment(&device);

    let mut damaged = shown[HASH_BYTES..].to_vec();
    *damaged.last_mut().expect("a ciphertext has bytes") ^= 1;
    hold_up_ciphertext(&device, &damaged);
    device.expect_was_shown("Wrong key or corrupt");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::DecryptSecretScan
    );
    walk::go_back(&device);
    walk::go_home(&device);

    hold_up_ciphertext(&device, &shown[HASH_BYTES..]);
    device.wait_for_text("DECRYPTED");
    device.expect_text(SECRET_FIRST_LINE);
    device.expect_text(SECRET_SECOND_LINE);
    assert_eq!(
        app_state().navigation.app.state,
        AppState::DecryptSecretResult
    );
}

fn refuse_what_is_no_ciphertext() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    open_wallet_advanced(&device, "Decrypt Secret");
    hold_up_qr(b"this is no ciphertext");
    device.settle();
    lower_qr();
    device.expect_was_shown("Invalid ciphertext hex");
    assert_eq!(
        app_state().navigation.app.state,
        AppState::DecryptSecretScan
    );
}
