//! Private swaps: an isolated key, bound to one covenant, whose claim only completes as agreed.
use crate::harness::{hold_up_frames, hold_up_qr, lower_qr, note, recall, Device};
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::signing_flows::covenant::{hex, unhex};
use crate::scenarios::journeys::signing_flows::signing::read_framed_answer;
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use kaspa_portal::contract::covenant::build_private_swap_script;
use kaspa_portal::crypto::adaptor::{verify_adaptor_presignature, AdaptorPreSignature};
use kaspa_portal::crypto::schnorr::{schnorr_verify, SchnorrSignature};
use kaspa_portal::transaction::signing::anti_klepto::protocol::host_commitment;
use kaspa_portal::transaction::signing::covenant::protocol::private_swap::{
    claim_sighash, encode_request, encode_reveal, parse_response, session_id, PrivateSwapRequest,
    PrivateSwapResponse, PrivateSwapReveal, RequestKind, ResponseKind, RESPONSE_LEN, REVEAL_LEN,
    SESSION_ID_LEN,
};

const HOST_SECRET: [u8; 32] = [0x39; 32];
const REFUND_AFTER_DAA: u64 = 1_000_000;
const SALT: [u8; 16] = [0x42; 16];
const NO_KEY: [u8; 32] = [0; 32];
const REQUEST_ROOM: usize = 1024;

journey!(
    a_bound_swap_key_pre_signs_and_completes_after_a_power_cycle:
    restore_the_known_wallet,
    bind_a_swap_key,
    claim_with_the_bound_key,
);
journey!(a_swap_key_refuses_a_covenant_paying_someone_else: restore_the_known_wallet, refuse_another_claimer);
journey!(a_swap_claim_refuses_a_covenant_it_was_not_bound_to: restore_the_known_wallet, refuse_an_unbound_claim);
journey!(a_swap_pre_signature_needs_the_committed_host_secret: restore_the_known_wallet, refuse_the_wrong_reveal);

fn blank_request<'a>(kind: RequestKind, payload: &'a [u8]) -> PrivateSwapRequest<'a> {
    PrivateSwapRequest {
        kind,
        session_id: [0; SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: NO_KEY,
        binding_token: [0; 32],
        adaptor_point: [0; 32],
        presignature: [0; 64],
        presignature_negated: false,
        payload,
    }
}

fn scan_request(device: &Device, request: &PrivateSwapRequest<'_>, screen: &str) {
    let mut wire = [0u8; REQUEST_ROOM];
    let length = encode_request(request, &mut wire).expect("a request");
    device.tap("Scan QR");
    hold_up_frames(&wire[..length]);
    device.wait_for_text(screen);
    lower_qr();
}

fn read_answer(device: &Device, label: &str, expected: ResponseKind) -> PrivateSwapResponse {
    let shown = read_framed_answer(device, label);
    assert_eq!(shown.len(), RESPONSE_LEN, "a whole answer is on screen");
    let answer = parse_response(&shown).expect("a private swap answer");
    assert_eq!(answer.kind, expected);
    answer
}

fn new_swap_key(device: &Device) -> PrivateSwapResponse {
    scan_request(
        device,
        &blank_request(RequestKind::KeyInfo, &[]),
        "PRIVATE SWAP KEY",
    );
    let key = read_answer(device, "SHOW QR", ResponseKind::KeyInfo);
    assert_ne!(key.key_id, NO_KEY, "the device picks the key instance");
    assert_ne!(
        key.claim_pubkey,
        known::about_key(false, 0),
        "a swap key is not a wallet key"
    );
    walk::go_home(device);
    key
}

fn covenant_claimed_by(claimer: &[u8; 32]) -> Vec<u8> {
    build_private_swap_script(
        &known::about_key(false, 1),
        claimer,
        &known::p2pk_script(known::STRANGER_KEY),
        REFUND_AFTER_DAA,
        &SALT,
    )
    .expect("a canonical covenant")
}

fn bind(device: &Device, key: &PrivateSwapResponse, covenant: &[u8]) -> [u8; 32] {
    let request = PrivateSwapRequest {
        key_id: key.key_id,
        adaptor_point: key.adaptor_point,
        ..blank_request(RequestKind::Bind, covenant)
    };
    scan_request(device, &request, "BIND SWAP KEY");
    device.tap("BIND KEY");
    device.wait_for_text("SWAP KEY BOUND");
    let bound = read_answer(device, "SHOW QR", ResponseKind::Binding);
    assert_eq!(
        bound.claim_pubkey, key.claim_pubkey,
        "binding keeps the key"
    );
    assert_ne!(bound.binding_token, [0; 32]);
    walk::go_home(device);
    bound.binding_token
}

fn bind_a_swap_key() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key = new_swap_key(&device);
    let token = bind(&device, &key, &covenant_claimed_by(&key.claim_pubkey));
    note("key", &hex(&key.key_id));
    note("pubkey", &hex(&key.claim_pubkey));
    note("adaptor", &hex(&key.adaptor_point));
    note("token", &hex(&token));
}

struct BoundKey {
    key_id: [u8; 32],
    pubkey: [u8; 32],
    adaptor: [u8; 32],
    token: [u8; 32],
}

impl BoundKey {
    fn pre_sign_request<'a>(&self, claim: &'a [u8]) -> PrivateSwapRequest<'a> {
        let commitment = host_commitment(&HOST_SECRET);
        PrivateSwapRequest {
            session_id: session_id(&commitment, claim, &self.key_id, &self.adaptor),
            host_commitment: commitment,
            key_id: self.key_id,
            binding_token: self.token,
            adaptor_point: self.adaptor,
            ..blank_request(RequestKind::PreSign, claim)
        }
    }
}

/// Reviews a claim and commits to its nonce, leaving the device waiting for the host's reveal.
fn start_pre_signing(device: &Device, request: &PrivateSwapRequest<'_>) {
    scan_request(device, request, "SWAP CLAIM");
    device.tap("PRE-SIGN");
    device.wait_for_text("NONCE COMMITTED");
    let nonce = read_answer(device, "SHOW NONCE QR", ResponseKind::Nonce);
    assert_eq!(
        nonce.signature, [0; 64],
        "nothing is signed before the host reveals"
    );
}

fn reveal(request: &PrivateSwapRequest<'_>, sighash: [u8; 32], secret: [u8; 32]) -> Vec<u8> {
    let mut wire = [0u8; REVEAL_LEN];
    let reveal = PrivateSwapReveal {
        session_id: request.session_id,
        key_id: request.key_id,
        sighash,
        host_secret: secret,
    };
    encode_reveal(&reveal, &mut wire).expect("a reveal");
    wire.to_vec()
}

fn claim_with_the_bound_key() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key = BoundKey {
        key_id: unhex(&recall("key")),
        pubkey: unhex(&recall("pubkey")),
        adaptor: unhex(&recall("adaptor")),
        token: unhex(&recall("token")),
    };
    let claim = known::transaction_claiming_a_swap(&covenant_claimed_by(&key.pubkey));
    let sighash = claim_sighash(&claim).expect("a claim");
    let request = key.pre_sign_request(&claim);
    start_pre_signing(&device, &request);
    device.tap("SCAN REVEAL");
    hold_up_qr(&reveal(&request, sighash, HOST_SECRET));
    device.wait_for_text("PRE-SIGNED");
    lower_qr();
    let signed = read_answer(&device, "SHOW QR", ResponseKind::PreSignature);
    let presignature = AdaptorPreSignature {
        bytes: signed.signature,
        negated: signed.negated,
    };
    verify_adaptor_presignature(&key.pubkey, &sighash, &presignature, &key.adaptor)
        .expect("a pre-signature locked to the adaptor point");
    walk::go_home(&device);

    let complete = PrivateSwapRequest {
        key_id: key.key_id,
        binding_token: key.token,
        adaptor_point: key.adaptor,
        presignature: signed.signature,
        presignature_negated: signed.negated,
        ..blank_request(RequestKind::Complete, &claim)
    };
    scan_request(&device, &complete, "FINISH CLAIM");
    device.tap("COMPLETE");
    device.wait_for_text("CLAIM COMPLETE");
    let completed = read_answer(&device, "SHOW QR", ResponseKind::Completed);
    let signature = SchnorrSignature {
        bytes: completed.signature,
    };
    schnorr_verify(&key.pubkey, &sighash, &signature).expect("a signature from the swap key");
}

fn refuse_another_claimer() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key = new_swap_key(&device);
    let covenant = covenant_claimed_by(&known::about_key(false, 2));
    let request = PrivateSwapRequest {
        key_id: key.key_id,
        adaptor_point: key.adaptor_point,
        ..blank_request(RequestKind::Bind, &covenant)
    };
    scan_request(&device, &request, "ERROR");
    device.expect_absent("BIND SWAP KEY");
}

fn bound_key(device: &Device) -> (BoundKey, Vec<u8>) {
    let key = new_swap_key(device);
    let covenant = covenant_claimed_by(&key.claim_pubkey);
    let token = bind(device, &key, &covenant);
    let bound = BoundKey {
        key_id: key.key_id,
        pubkey: key.claim_pubkey,
        adaptor: key.adaptor_point,
        token,
    };
    (bound, covenant)
}

fn refuse_an_unbound_claim() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let (key, _) = bound_key(&device);
    let other_covenant = build_private_swap_script(
        &known::about_key(false, 1),
        &key.pubkey,
        &known::p2pk_script(known::STRANGER_KEY),
        REFUND_AFTER_DAA + 1,
        &SALT,
    )
    .expect("a canonical covenant");
    let claim = known::transaction_claiming_a_swap(&other_covenant);
    scan_request(&device, &key.pre_sign_request(&claim), "ERROR");
    device.expect_absent("SWAP CLAIM");
}

fn refuse_the_wrong_reveal() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let (key, covenant) = bound_key(&device);
    let claim = known::transaction_claiming_a_swap(&covenant);
    let sighash = claim_sighash(&claim).expect("a claim");
    let request = key.pre_sign_request(&claim);
    start_pre_signing(&device, &request);
    device.tap("SCAN REVEAL");
    hold_up_qr(&reveal(&request, sighash, [0x3A; 32]));
    device.wait_for_text("ERROR");
    lower_qr();
    device.expect_absent("PRE-SIGNED");
}
