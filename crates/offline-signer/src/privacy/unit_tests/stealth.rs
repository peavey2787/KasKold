use crate::derivation::bip32::derive_account_key;
use crate::privacy::stealth::{
    scan_request, validate_request, StealthError, REQUEST_MAGIC, RESPONSE_MAGIC,
};

#[test]
fn validates_stlh_shape_and_bounds() {
    assert_eq!(validate_request(b"STLH\0"), Err(StealthError::InvalidCount));
    assert_eq!(
        validate_request(b"NOPE\x01"),
        Err(StealthError::InvalidMagic)
    );
    let mut request = [0u8; 37];
    request[..4].copy_from_slice(&REQUEST_MAGIC);
    request[4] = 1;
    assert_eq!(validate_request(&request), Ok(1));
}

#[test]
fn response_is_deterministic_and_preserves_invalid_candidate_as_zero_record() {
    let seed = [0x42u8; 64];
    let account = derive_account_key(&seed).expect("account key");
    let mut request = [0u8; 37];
    request[..4].copy_from_slice(&REQUEST_MAGIC);
    request[4] = 1;
    let response = scan_request(&account, &request).expect("scan response");
    assert_eq!(&response[..4], &RESPONSE_MAGIC);
    assert_eq!(response[4], 1);
    assert_eq!(&response[5..], &[0u8; 64]);
}

#[test]
fn valid_ephemeral_candidate_exercises_scalar_and_one_time_key_path() {
    let account = derive_account_key(&[0x24u8; 64]).expect("account key");
    let candidate = account.public_key_x_only().expect("candidate public key");
    let mut request = [0u8; 37];
    request[..4].copy_from_slice(&REQUEST_MAGIC);
    request[4] = 1;
    request[5..37].copy_from_slice(&candidate);
    let response = scan_request(&account, &request).expect("scan response");
    assert_eq!(&response[..4], &RESPONSE_MAGIC);
    assert_eq!(response[4], 1);
    assert_ne!(&response[5..37], &[0u8; 32]);
    assert_ne!(&response[37..69], &[0u8; 32]);
}

#[test]
fn stealth_error_messages_cover_every_stable_variant() {
    for error in [
        StealthError::RequestTooShort,
        StealthError::InvalidMagic,
        StealthError::InvalidCount,
        StealthError::ResponseTooLarge,
        StealthError::Derivation,
        StealthError::InvalidAccountKey,
    ] {
        assert!(!error.message().is_empty());
    }
}

#[test]
fn stlh_request_rejects_short_oversized_and_truncated_candidate_lists() {
    use crate::privacy::stealth::{CANDIDATE_LEN, MAX_CANDIDATES, REQUEST_HEADER_LEN};

    assert_eq!(
        validate_request(b"STLH"),
        Err(StealthError::RequestTooShort)
    );
    let mut header = [0u8; REQUEST_HEADER_LEN];
    header[..4].copy_from_slice(&REQUEST_MAGIC);
    header[4] = 2;
    let mut truncated = header.to_vec();
    truncated.extend_from_slice(&[0u8; CANDIDATE_LEN]);
    assert_eq!(
        validate_request(&truncated),
        Err(StealthError::InvalidCount)
    );
    truncated.extend_from_slice(&[0u8; CANDIDATE_LEN]);
    assert_eq!(validate_request(&truncated), Ok(2));

    let count = MAX_CANDIDATES + 1;
    let mut oversized = std::vec![0u8; REQUEST_HEADER_LEN + count * CANDIDATE_LEN];
    oversized[..4].copy_from_slice(&REQUEST_MAGIC);
    oversized[4] = u8::try_from(count).expect("fits u8");
    assert_eq!(
        validate_request(&oversized),
        Err(StealthError::InvalidCount)
    );
}
