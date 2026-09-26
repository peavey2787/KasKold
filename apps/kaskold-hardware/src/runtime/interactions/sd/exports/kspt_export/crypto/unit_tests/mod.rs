use super::*;

const DESCRIPTOR: &str = concat!(
    "multi_hd45(2,",
    "kpub2J937qL9n85s7HrhYyYYdMkzq1kaMiAf9PAcJzRW3jV7NgntNfGGrNgut7ZxcVrJqH42BCT2WyjfnxJh3SBDjLhXHe3UC2RJUu5tcjsViuK,",
    "kpub2Jtuqt6WJWZv3fQUnKhuEaCxbAyzLsFn3UEEaM4g7CXa2LZjQZH4o6tpj83tFaewMEyX56qrAF4Q64uqunVyBayuuRNwjru5DWchDEcq5vz",
    ")"
);
const PASSWORD: &[u8] = b"CorrectHorse9";
const SALT: [u8; SALT_SIZE] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];
const NONCE: [u8; NONCE_LEN] = [0x24; NONCE_LEN];

fn current_envelope() -> ([u8; 1024], usize) {
    let mut encrypted = [0u8; 1024];
    let length = seal_envelope(
        DESCRIPTOR.as_bytes(), PASSWORD, &SALT, &NONCE, &mut encrypted,
    )
    .expect("encrypt current envelope");
    (encrypted, length)
}

#[test]
fn current_envelope_round_trip_authenticates_kdf_metadata_and_has_no_fallback() {
    let (encrypted, length) = current_envelope();
    assert_eq!(&encrypted[..4], CURRENT_MAGIC);
    let mut plaintext = [0u8; 1024];
    let n = open_envelope(&encrypted, length, PASSWORD, &mut plaintext).expect("decrypt");
    assert_eq!(&plaintext[..n], DESCRIPTOR.as_bytes());

    for offset in [6usize, 7, 8, 9, 10, 14, 18, 25, 39, CURRENT_CIPHERTEXT_START] {
        let mut tampered = encrypted;
        tampered[offset] ^= 1;
        assert!(open_envelope(&tampered, length, PASSWORD, &mut plaintext).is_err());
    }
    assert_eq!(
        open_envelope(&encrypted, length, b"WrongHorse9", &mut plaintext),
        Err(DecryptError::Authentication)
    );
    let mut unsupported = encrypted;
    unsupported[6] = 0xff;
    assert_eq!(
        open_envelope(&unsupported, length, PASSWORD, &mut plaintext),
        Err(DecryptError::InvalidEnvelope)
    );
}

#[test]
fn current_envelope_rejects_weak_params_malformed_lengths_and_zero_material() {
    let (encrypted, length) = current_envelope();
    let mut plaintext = [0xa5u8; 1024];

    let mut weak = encrypted;
    weak[10..14].copy_from_slice(&(password_kdf::PasswordKdfParams::current().m_cost_kib - 1).to_le_bytes());
    assert_eq!(
        open_envelope(&weak, length, PASSWORD, &mut plaintext),
        Err(DecryptError::InvalidEnvelope)
    );

    for bad_length in [0usize, 3, CURRENT_HEADER_LEN + TAG_LEN, length - 1, length + 1] {
        assert_eq!(
            open_envelope(&encrypted, bad_length, PASSWORD, &mut plaintext),
            Err(DecryptError::InvalidEnvelope)
        );
    }
    let mut zero_salt = encrypted;
    let salt_start = 6 + METADATA_SIZE;
    zero_salt[salt_start..salt_start + SALT_SIZE].fill(0);
    assert_eq!(
        open_envelope(&zero_salt, length, PASSWORD, &mut plaintext),
        Err(DecryptError::InvalidEnvelope)
    );
    let mut zero_nonce = encrypted;
    let nonce_start = salt_start + SALT_SIZE;
    zero_nonce[nonce_start..CURRENT_HEADER_LEN].fill(0);
    assert_eq!(
        open_envelope(&zero_nonce, length, PASSWORD, &mut plaintext),
        Err(DecryptError::InvalidEnvelope)
    );
}

#[test]
fn authentication_failure_clears_current_plaintext_region() {
    let (mut encrypted, length) = current_envelope();
    encrypted[length - 1] ^= 1;
    let mut plaintext = [0xa5u8; 1024];
    assert_eq!(
        open_envelope(&encrypted, length, PASSWORD, &mut plaintext),
        Err(DecryptError::Authentication)
    );
    assert!(plaintext[..DESCRIPTOR.len()].iter().all(|byte| *byte == 0));
}

#[test]
fn historical_pbkdf2_transport_magic_is_rejected() {
    let mut legacy = [0u8; 64];
    legacy[..4].copy_from_slice(b"KAS\x03");
    legacy[4..6].copy_from_slice(&1u16.to_le_bytes());
    let mut plaintext = [0u8; 64];
    assert_eq!(
        open_envelope(&legacy, legacy.len(), PASSWORD, &mut plaintext),
        Err(DecryptError::InvalidEnvelope)
    );
}
