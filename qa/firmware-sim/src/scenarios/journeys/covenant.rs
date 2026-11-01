//! Covenant keys: isolated from the wallet, bound to one script, and only signing with the host's nonce.
use super::known::{self, PIN};
use super::signing::read_every_frame;
use super::walk::{self, restore_the_known_wallet};
use crate::harness::{hold_up_frames, hold_up_qr, lower_qr, note, recall, Device};
use kaspa_portal::crypto::schnorr::{schnorr_verify, SchnorrSignature};
use kaspa_portal::transaction::signing::anti_klepto::protocol::host_commitment;
use kaspa_portal::transaction::signing::covenant::protocol::{
    encode_request, encode_reveal, parse_response, recompute_known_commitment, BindingHint,
    CovenantSignRequest, CovenantSignResponse, CovenantSignReveal, KnownScheme, RequestKind,
    ResponseKind, RESPONSE_LEN, REVEAL_LEN, SESSION_ID_LEN,
};

const SCRIPT: &[u8] = b"an opaque covenant script";
const OTHER_SCRIPT: &[u8] = b"a different covenant script";
const COMMITMENT: [u8; 32] = [0x51; 32];
const SESSION: [u8; SESSION_ID_LEN] = [0x47; SESSION_ID_LEN];
const HOST_SECRET: [u8; 32] = [0x39; 32];
const KNOWN_CONTEXT: &[u8] = b"a known covenant context";

journey!(
    a_bound_covenant_key_signs_after_a_power_cycle_with_the_hosts_nonce:
    restore_the_known_wallet,
    bind_the_key_to_the_script,
    sign_with_the_bound_key,
);
journey!(a_covenant_key_refuses_a_script_it_was_not_bound_to: restore_the_known_wallet, refuse_the_other_script);
journey!(a_covenant_signature_needs_the_committed_host_secret: restore_the_known_wallet, refuse_the_wrong_reveal);
journey!(a_known_covenant_shows_what_it_signs: restore_the_known_wallet, sign_a_known_covenant);

fn request<'a>(
    kind: RequestKind,
    key_id: [u8; 32],
    token: [u8; 32],
    script: &'a [u8],
) -> CovenantSignRequest<'a> {
    let signing = kind == RequestKind::Opaque;
    CovenantSignRequest {
        kind,
        scheme: KnownScheme::None,
        binding: BindingHint::None,
        session_id: if signing {
            SESSION
        } else {
            [0; SESSION_ID_LEN]
        },
        host_commitment: if signing {
            host_commitment(&HOST_SECRET)
        } else {
            [0; 32]
        },
        key_id,
        binding_token: token,
        commitment: if signing { COMMITMENT } else { [0; 32] },
        script,
        context: &[],
    }
}

fn show_request(device: &Device, request: &CovenantSignRequest<'_>, screen: &str) {
    let mut wire = [0u8; 512];
    let length = encode_request(request, &mut wire).expect("a request");
    hold_up_frames(&wire[..length]);
    device.wait_for_text(screen);
    lower_qr();
}

fn scan_request(device: &Device, request: &CovenantSignRequest<'_>, screen: &str) {
    device.tap("Scan QR");
    show_request(device, request, screen);
}

fn read_response(device: &Device, label: &str, expected: ResponseKind) -> CovenantSignResponse {
    device.tap(label);
    device.wait_for_text("QR DISPLAY MODE");
    device.tap("Manual");
    let shown = read_every_frame(device);
    assert_eq!(shown.len(), RESPONSE_LEN, "a whole response is on screen");
    let response = parse_response(&shown).expect("a covenant response");
    assert_eq!(response.kind, expected);
    device.settle();
    response
}

fn new_covenant_key(device: &Device) -> CovenantSignResponse {
    scan_request(
        device,
        &request(RequestKind::KeyInfo, [0; 32], [0; 32], &[]),
        "COVENANT KEY",
    );
    let key = read_response(device, "SHOW QR", ResponseKind::KeyInfo);
    assert_ne!(key.key_id, [0; 32], "the device picks the key instance");
    assert_ne!(
        key.pubkey_x,
        known::about_key(false, 0),
        "a covenant key is not a wallet key"
    );
    walk::go_home(device);
    key
}

fn bind(device: &Device, key: &CovenantSignResponse, script: &[u8]) -> [u8; 32] {
    scan_request(
        device,
        &request(RequestKind::Bind, key.key_id, [0; 32], script),
        "BIND OPAQUE KEY",
    );
    device.tap("I UNDERSTAND");
    device.wait_for_text("CONFIRM BIND");
    device.tap("BIND KEY");
    device.wait_for_text("KEY BOUND");
    let bound = read_response(device, "SHOW QR", ResponseKind::Binding);
    assert_eq!(bound.pubkey_x, key.pubkey_x, "binding keeps the key");
    assert_ne!(bound.binding_token, [0; 32]);
    walk::go_home(device);
    bound.binding_token
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).expect("hex");
    }
    out
}

fn bind_the_key_to_the_script() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key = new_covenant_key(&device);
    let token = bind(&device, &key, SCRIPT);
    note("key", &hex(&key.key_id));
    note("pubkey", &hex(&key.pubkey_x));
    note("token", &hex(&token));
}

fn start_opaque_signing(device: &Device, key_id: [u8; 32], token: [u8; 32]) {
    scan_request(
        device,
        &request(RequestKind::Opaque, key_id, token, SCRIPT),
        "OPAQUE COVENANT",
    );
    device.tap("I UNDERSTAND");
    device.wait_for_text("CONFIRM SIGN");
    device.tap("SIGN OPAQUE");
    device.wait_for_text("NONCE COMMITTED");
    let nonce = read_response(device, "SHOW NONCE QR", ResponseKind::NonceCommitment);
    assert_eq!(
        nonce.signature, [0; 64],
        "no signature leaves before the host reveals"
    );
}

fn reveal(key_id: [u8; 32], commitment: [u8; 32], secret: [u8; 32]) -> Vec<u8> {
    let mut wire = [0u8; REVEAL_LEN];
    let reveal = CovenantSignReveal {
        session_id: SESSION,
        key_id,
        commitment,
        host_secret: secret,
    };
    encode_reveal(&reveal, &mut wire).expect("a reveal");
    wire.to_vec()
}

fn sign_with_the_bound_key() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key_id = unhex(&recall("key"));
    let pubkey = unhex(&recall("pubkey"));
    start_opaque_signing(&device, key_id, unhex(&recall("token")));
    device.tap("SCAN REVEAL");
    hold_up_qr(&reveal(key_id, COMMITMENT, HOST_SECRET));
    device.wait_for_text("COVENANT SIGNED");
    lower_qr();
    let signed = read_response(&device, "SHOW QR", ResponseKind::Signature);
    let signature = SchnorrSignature {
        bytes: signed.signature,
    };
    schnorr_verify(&pubkey, &COMMITMENT, &signature).expect("a signature from the covenant key");
}

fn refuse_the_other_script() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key = new_covenant_key(&device);
    let token = bind(&device, &key, SCRIPT);
    device.tap("Scan QR");
    let wrong = request(RequestKind::Opaque, key.key_id, token, OTHER_SCRIPT);
    show_request(&device, &wrong, "ERROR");
    device.expect_absent("OPAQUE COVENANT");
}

fn refuse_the_wrong_reveal() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key = new_covenant_key(&device);
    let token = bind(&device, &key, SCRIPT);
    start_opaque_signing(&device, key.key_id, token);
    device.tap("SCAN REVEAL");
    hold_up_qr(&reveal(key.key_id, COMMITMENT, [0x3A; 32]));
    device.wait_for_text("ERROR");
    lower_qr();
    device.expect_absent("COVENANT SIGNED");
}

fn sign_a_known_covenant() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    let key = new_covenant_key(&device);
    let commitment = recompute_known_commitment(KnownScheme::Sha256Preimage, KNOWN_CONTEXT)
        .expect("a commitment");
    let mut script = vec![0x20];
    script.extend_from_slice(&commitment);
    script.push(0x20);
    script.extend_from_slice(&key.pubkey_x);
    script.push(0xD7);
    let bind = CovenantSignRequest {
        kind: RequestKind::Bind,
        scheme: KnownScheme::Sha256Preimage,
        binding: BindingHint::FixedCheckSigFromStack,
        session_id: [0; SESSION_ID_LEN],
        host_commitment: [0; 32],
        key_id: key.key_id,
        binding_token: [0; 32],
        commitment,
        script: &script,
        context: KNOWN_CONTEXT,
    };
    scan_request(&device, &bind, "BIND KEY");
    device.expect_text("SHA-256 CONTEXT VERIFIED");
    device.tap("BIND KEY");
    device.wait_for_text("KEY BOUND");
    let bound = read_response(&device, "SHOW QR", ResponseKind::Binding);
    walk::go_home(&device);
    let sign = CovenantSignRequest {
        kind: RequestKind::Known,
        session_id: SESSION,
        host_commitment: host_commitment(&HOST_SECRET),
        binding_token: bound.binding_token,
        ..bind
    };
    scan_request(&device, &sign, "COVENANT SIGN");
    device.expect_text("SHA-256 CONTEXT VERIFIED");
    device.tap("SIGN");
    device.wait_for_text("NONCE COMMITTED");
    read_response(&device, "SHOW NONCE QR", ResponseKind::NonceCommitment);
    device.tap("SCAN REVEAL");
    hold_up_qr(&reveal(key.key_id, commitment, HOST_SECRET));
    device.wait_for_text("COVENANT SIGNED");
    lower_qr();
    let signed = read_response(&device, "SHOW QR", ResponseKind::Signature);
    let signature = SchnorrSignature {
        bytes: signed.signature,
    };
    schnorr_verify(&key.pubkey_x, &commitment, &signature)
        .expect("a signature from the covenant key");
}
