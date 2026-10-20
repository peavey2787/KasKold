use crate::security::credential::{
    confirmation_digest, confirmation_matches, retry_delay_millis, unlock_attempt_limit_reached,
    unlock_attempts_remaining, CredentialPolicyKind, MAX_UNLOCK_FAILURES, RETRY_MAX_MILLIS,
};

#[test]
fn credential_retry_delay_doubles_and_saturates_at_one_minute() {
    assert_eq!(retry_delay_millis(0), 1_000);
    assert_eq!(retry_delay_millis(1), 2_000);
    assert_eq!(retry_delay_millis(2), 4_000);
    assert_eq!(retry_delay_millis(3), 8_000);
    assert_eq!(retry_delay_millis(4), 16_000);
    assert_eq!(retry_delay_millis(5), 32_000);
    assert_eq!(retry_delay_millis(6), RETRY_MAX_MILLIS);
    assert_eq!(retry_delay_millis(7), RETRY_MAX_MILLIS);
    assert_eq!(retry_delay_millis(u8::MAX), RETRY_MAX_MILLIS);
    assert_eq!(RETRY_MAX_MILLIS, 60_000);
}

#[test]
fn unlock_attempt_limit_is_ten_consecutive_failures() {
    assert_eq!(MAX_UNLOCK_FAILURES, 10);
    assert_eq!(unlock_attempts_remaining(0), 10);
    assert_eq!(unlock_attempts_remaining(9), 1);
    assert_eq!(unlock_attempts_remaining(10), 0);
    assert_eq!(unlock_attempts_remaining(u8::MAX), 0);
    assert!(!unlock_attempt_limit_reached(0));
    assert!(!unlock_attempt_limit_reached(9));
    assert!(unlock_attempt_limit_reached(10));
    assert!(unlock_attempt_limit_reached(u8::MAX));
}

#[test]
fn credential_confirmation_digest_and_constant_time_comparison_are_exact() {
    let digest = confirmation_digest(CredentialPolicyKind::Password, b"correct7horse");
    assert_eq!(
        digest,
        [
            0x76, 0x2a, 0x3a, 0xbc, 0x6c, 0x2d, 0xd4, 0x42, 0xb5, 0x9f, 0xa9, 0x2e, 0x0d, 0xc5,
            0xe1, 0xab, 0xa1, 0xe0, 0xa0, 0x03, 0x3a, 0x4c, 0x3f, 0x51, 0x2f, 0x9b, 0xfc, 0x6c,
            0xcf, 0x0b, 0xcf, 0x40,
        ],
    );
    assert!(confirmation_matches(&digest, &digest));
    let mut changed = digest;
    changed[3] ^= 0x08;
    changed[21] ^= 0x08;
    assert!(!confirmation_matches(&digest, &changed));
    assert_ne!(
        digest,
        confirmation_digest(CredentialPolicyKind::Pin, b"correct7horse")
    );
    assert_ne!(
        digest,
        confirmation_digest(CredentialPolicyKind::Password, b"correct7horsf")
    );
}
