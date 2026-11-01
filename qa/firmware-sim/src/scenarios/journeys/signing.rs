//! Signing: a transaction arrives by camera, a person reviews it, the answer leaves as a QR.
use super::known::{self, PIN};
use super::walk::{self, restore_the_known_wallet as restore_the_wallet};
use crate::harness::{app_state, hold_up_qr, lower_qr, read_qr_on_screen, Device};
use crate::runtime::input::AppState;
use kaspa_portal::crypto::schnorr::{schnorr_verify, SchnorrSignature};
use kaspa_portal::transaction::interchange::kspt::decode_compact_kspt;
use kaspa_portal::transaction::interchange::qr::frame::{parse_frame, session_id, SESSION_ID_LEN};
use kaspa_portal::transaction::model::{SigHashType, Transaction};
use kaspa_portal::transaction::sighash::calculate_sighash;

/// A tap on the QR code itself shows the next frame.
const NEXT_FRAME_TAP: (u16, u16) = (160, 120);

journey!(a_scanned_transaction_is_signed_and_leaves_as_a_verifiable_qr: restore_the_wallet, sign_by_camera);
journey!(cancelling_a_scanned_transaction_signs_nothing: restore_the_wallet, cancel_by_camera);
journey!(a_corrupted_transaction_never_reaches_review: restore_the_wallet, refuse_corrupted_transaction);
journey!(a_transaction_spending_other_coins_is_refused: restore_the_wallet, refuse_other_peoples_coins);
journey!(change_paying_a_stranger_never_reaches_review: restore_the_wallet, refuse_false_change);
journey!(every_output_can_be_inspected_before_signing: restore_the_wallet, inspect_every_output);

fn scan_the_known_transaction(device: &Device) {
    walk::unlock_with_pin(device, PIN);
    device.tap("Scan QR");
    hold_up_qr(&known::unsigned_transaction());
    device.wait_for_text("CONFIRM SEND");
    lower_qr();
    device.expect_text("Send: 1.5 KAS");
    device.expect_text("Fee: 0.01 KAS");
    device.expect_text("Change: 0.49 KAS");
}

fn sign_by_camera() {
    let device = Device::boot();
    scan_the_known_transaction(&device);
    device.tap("CONFIRM");
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Manual");
    let signed = decode_compact_kspt(&read_every_frame(&device)).expect("a signed transaction");
    assert_eq!(signed.outputs[0].value, 150_000_000);
    assert_eq!(signed.outputs[1].value, 49_000_000);
    expect_valid_signature(&signed, 0, known::about_key(false, 0));
    expect_valid_signature(&signed, 1, known::about_key(true, 0));
}

/// Asks for an answer too large for one QR code and reads it frame by frame.
pub(super) fn read_framed_answer(device: &Device, label: &str) -> Vec<u8> {
    device.tap(label);
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Manual");
    let shown = read_every_frame(device);
    device.settle();
    shown
}

pub(super) fn read_every_frame(device: &Device) -> Vec<u8> {
    let first = device.wait_for_qr();
    let total = usize::from(parse_frame(&first).expect("a QR frame").total_frames);
    let mut session = [0u8; SESSION_ID_LEN];
    session.copy_from_slice(&parse_frame(&first).expect("a QR frame").session_id);
    let mut payload = Vec::new();
    for index in 0..total {
        let shown = device.wait_for_qr();
        let frame = parse_frame(&shown).expect("a QR frame");
        assert_eq!(
            usize::from(frame.frame_index),
            index,
            "frames come in order"
        );
        assert_eq!(
            frame.session_id, session,
            "every frame belongs to one payload"
        );
        payload.extend_from_slice(frame.fragment);
        device.tap_at(NEXT_FRAME_TAP.0, NEXT_FRAME_TAP.1);
    }
    assert_eq!(
        session_id(&payload),
        session,
        "the frames reassemble the signed payload"
    );
    payload
}

fn expect_valid_signature(signed: &Transaction, input: usize, key: [u8; 32]) {
    let signature = &signed.inputs[input].sigs[0];
    assert!(
        signed.inputs[input].sig_count >= 1 && signature.present,
        "input {input} is unsigned"
    );
    let digest = calculate_sighash(signed, input, SigHashType::All);
    let bytes = SchnorrSignature {
        bytes: signature.signature,
    };
    schnorr_verify(&key, &digest, &bytes).expect("a signature from this wallet's key");
}

fn cancel_by_camera() {
    let device = Device::boot();
    scan_the_known_transaction(&device);
    device.tap("CANCEL");
    device.tap_at(NEXT_FRAME_TAP.0, NEXT_FRAME_TAP.1);
    assert!(
        read_qr_on_screen().is_none(),
        "a cancelled transaction shows no QR"
    );
    assert_ne!(app_state().navigation.app.state, AppState::ShowQrModeChoice);
    assert_eq!(
        app_state().signing.transaction.active.inputs[0].sig_count,
        0
    );
}

fn refuse_false_change() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Scan QR");
    hold_up_qr(&known::transaction_with_false_change());
    device.wait_for_text("TX-OWN-01");
    lower_qr();
    assert_eq!(app_state().qr.outgoing.length, 0);
    device.expect_absent("CONFIRM SEND");
}

fn inspect_every_output() {
    let device = Device::boot();
    scan_the_known_transaction(&device);
    device.tap("INSPECT");
    device.tap("Next");
    device.wait_for_text("DESTINATION 1");
    device.expect_text("1.50000000 KAS");
    walk::expect_review_address(&device, known::STRANGER_ADDRESS);
    device.tap("Next");
    device.wait_for_text("CHANGE 2");
    device.expect_text("0.49000000 KAS");
    walk::expect_review_address(&device, known::CHANGE_1);
}

fn refuse_corrupted_transaction() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Scan QR");
    let mut corrupted = known::unsigned_transaction();
    corrupted[0] ^= 0xFF;
    hold_up_qr(&corrupted);
    device.expect_no_transaction_review();
    let mut truncated = known::unsigned_transaction();
    truncated.truncate(truncated.len() / 2);
    hold_up_qr(&truncated);
    device.expect_no_transaction_review();
}

fn refuse_other_peoples_coins() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Scan QR");
    hold_up_qr(&known::transaction_spending_a_stranger());
    device.wait_for_text("CONFIRM SEND");
    lower_qr();
    device.tap("CONFIRM");
    device.wait_for_text("This wallet cannot sign this");
    assert!(
        read_qr_on_screen().is_none(),
        "nothing leaves the device looking signed"
    );
    assert_eq!(app_state().qr.outgoing.length, 0);
}
