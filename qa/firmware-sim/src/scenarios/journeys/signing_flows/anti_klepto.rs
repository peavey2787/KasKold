//! Anti-klepto signing: the device commits to its nonces before it learns the host's secret.
use crate::harness::{app_state, hold_up_frames, hold_up_qr, lower_qr, Device};
use crate::runtime::data::AntiKleptoPhase;
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::signing_flows::signing::read_every_frame;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use kaspa_portal::transaction::interchange::kspt::verify_host_transcript_wire;
use kaspa_portal::transaction::signing::anti_klepto::protocol::{
    encode_request, encode_reveal, parse_commitment, parse_request, parse_signed,
};

const HOST_SECRET: [u8; 32] = [0x5a; 32];
const ANOTHER_SECRET: [u8; 32] = [0x5b; 32];
const MESSAGE_SPACE: usize = 8 * 1024;
const REVEAL_SPACE: usize = 128;

journey!(a_signature_is_bound_to_the_secret_the_host_revealed: restore_the_known_wallet, sign_with_the_revealed_secret);
journey!(a_reveal_of_another_secret_signs_nothing: restore_the_known_wallet, refuse_another_secret);
journey!(cancelling_at_review_ends_the_session: restore_the_known_wallet, cancel_the_session);

fn request_wire() -> Vec<u8> {
    let mut wire = vec![0u8; MESSAGE_SPACE];
    let length =
        encode_request(&HOST_SECRET, &known::unsigned_transaction(), &mut wire).expect("a request");
    wire.truncate(length);
    wire
}

fn scan_the_request(device: &Device, request: &[u8]) {
    walk::unlock_with_pin(device, PIN);
    device.tap("Scan QR");
    hold_up_frames(request);
    device.wait_for_text("CONFIRM SEND");
    lower_qr();
}

/// Confirms, reads the commitment off the screen, and raises the camera for the reveal.
fn read_the_commitment(device: &Device) -> Vec<u8> {
    device.tap("CONFIRM");
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Manual");
    let commitment = read_every_frame(device);
    device.wait_for_text("SCAN COMPANION QR");
    device.tap("OK");
    device.settle();
    commitment
}

fn hold_up_the_reveal(request: &[u8], secret: &[u8; 32]) {
    let session = parse_request(request).expect("a request").session_id;
    let mut reveal = [0u8; REVEAL_SPACE];
    let length = encode_reveal(&session, secret, &mut reveal).expect("a reveal");
    hold_up_qr(&reveal[..length]);
}

fn sign_with_the_revealed_secret() {
    let device = Device::boot();
    let request = request_wire();
    scan_the_request(&device, &request);
    let commitment = read_the_commitment(&device);
    hold_up_the_reveal(&request, &HOST_SECRET);
    device.wait_for_text("QR DISPLAY MODE");
    lower_qr();
    device.tap("Manual");
    let signed = read_every_frame(&device);

    let commitment = parse_commitment(&commitment).expect("a commitment");
    let signed = parse_signed(&signed).expect("a signed answer");
    let unsigned = parse_request(&request).expect("a request").transaction;
    verify_host_transcript_wire(
        unsigned,
        signed.transaction,
        &commitment,
        &signed,
        &HOST_SECRET,
    )
    .expect("signatures bound to the commitment and the secret");
    assert!(
        verify_host_transcript_wire(
            unsigned,
            signed.transaction,
            &commitment,
            &signed,
            &ANOTHER_SECRET
        )
        .is_err(),
        "the check accepted a secret the device never saw"
    );
}

fn refuse_another_secret() {
    let device = Device::boot();
    let request = request_wire();
    scan_the_request(&device, &request);
    read_the_commitment(&device);
    hold_up_the_reveal(&request, &ANOTHER_SECRET);
    device.wait_for_text("Anti-klepto request rejected");
    lower_qr();
    assert_eq!(
        app_state().signing.anti_klepto.phase,
        AntiKleptoPhase::Inactive
    );
    assert_eq!(
        app_state().signing.transaction.active.inputs[0].sig_count,
        0
    );
    device.tap("BACK");
    device.settle();
    device.expect_absent("QR DISPLAY MODE");
}

fn cancel_the_session() {
    let device = Device::boot();
    scan_the_request(&device, &request_wire());
    device.tap("CANCEL");
    device.wait_for_text("TX Cancelled");
    device.tap("OK");
    device.wait_for_text("Scan QR");
    assert_eq!(
        app_state().signing.anti_klepto.phase,
        AntiKleptoPhase::Inactive
    );
    assert_eq!(
        app_state().signing.transaction.active.inputs[0].sig_count,
        0
    );
}
