//! Finding coins sent to a one-time address: the device answers only for keys it can spend.
use crate::harness::{hold_up_qr, lower_qr, Device};
use crate::scenarios::journeys::known::{self, PIN};
use crate::scenarios::journeys::walk::{self, restore_the_known_wallet};
use k256::elliptic_curve::ops::Reduce;
use k256::elliptic_curve::sec1::ToSec1Point;
use k256::{FieldBytes, ProjectivePoint, Scalar};
use sha2::{Digest, Sha256};

const SENDER_SECRET: [u8; 32] = [0x31; 32];
const NOT_A_KEY: [u8; 32] = [0xff; 32];
const HEADER: usize = 5;
const RECORD: usize = 64;

journey!(a_stealth_scan_finds_the_key_a_sender_paid: restore_the_known_wallet, scan_for_a_payment);

fn scalar_of(bytes: &[u8; 32]) -> Scalar {
    <Scalar as Reduce<FieldBytes>>::reduce(&(*bytes).into())
}

fn x_only(point: ProjectivePoint) -> [u8; 32] {
    let encoded = point.to_affine().to_sec1_point(true);
    encoded.as_bytes()[1..33]
        .try_into()
        .expect("an x coordinate")
}

fn point_of(x: &[u8; 32]) -> ProjectivePoint {
    let mut compressed = [0u8; 33];
    compressed[0] = 0x02;
    compressed[1..].copy_from_slice(x);
    k256::PublicKey::from_sec1_bytes(&compressed)
        .expect("a point")
        .to_projective()
}

/// What a sender computes knowing only the wallet's scan and spend keys.
fn one_time_key(sender: &Scalar, scan: &[u8; 32], spend: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let shared = x_only(point_of(scan) * sender);
    let mut hasher = Sha256::new();
    hasher.update(b"KasStealth");
    hasher.update(shared);
    hasher.update(0u32.to_be_bytes());
    let tweak: [u8; 32] = hasher.finalize().into();
    let one_time = point_of(spend) + ProjectivePoint::GENERATOR * scalar_of(&tweak);
    (x_only(one_time), tweak)
}

fn scan_for_a_payment() {
    let device = Device::boot();
    walk::unlock_with_pin(&device, PIN);
    device.tap("Scan QR");
    let sender = scalar_of(&SENDER_SECRET);
    let (scan, spend) = known::about_stealth_keys();
    let mut request = b"STLH\x02".to_vec();
    request.extend_from_slice(&x_only(ProjectivePoint::GENERATOR * sender));
    request.extend_from_slice(&NOT_A_KEY);
    hold_up_qr(&request);
    let answer = device.wait_for_qr();
    lower_qr();
    assert_eq!(&answer[..HEADER], b"STLR\x02");
    let (expected_key, expected_tweak) = one_time_key(&sender, &scan, &spend);
    assert_eq!(answer[HEADER..HEADER + 32], expected_key);
    assert_eq!(answer[HEADER + 32..HEADER + RECORD], expected_tweak);
    assert_eq!(
        answer[HEADER + RECORD..HEADER + 2 * RECORD],
        [0u8; RECORD],
        "an invalid candidate gets an empty record"
    );
}
