//! Device-facing credential confirmation, retry, and wipe policy.
//!
//! Which PINs and passwords are acceptable is shared with the software vault
//! in `shared-signer::credential_policy` and re-exported here. Encrypted-container
//! metadata such as the persisted credential discriminator and salt width lives
//! in `offline-signer::crypto::credential`.

use sha2::{Digest, Sha256};
pub use shared_signer::credential_policy::{
    validate, validate_entered_password, CredentialError, CredentialPolicyKind,
};

pub const RETRY_BASE_MILLIS: u32 = 1_000;
pub const RETRY_MAX_MILLIS: u32 = 60_000;
/// Consecutive failed unlock attempts, counted durably across reboots, after
/// which the device erases every saved wallet instead of accepting more guesses.
pub const MAX_UNLOCK_FAILURES: u8 = 10;

/// Online-guess backoff for the durable failure count. The delay is a
/// non-blocking "retry not before" deadline, so it never holds the watchdog.
#[must_use]
pub fn retry_delay_millis(failures: u8) -> u32 {
    (RETRY_BASE_MILLIS << failures.min(6)).min(RETRY_MAX_MILLIS)
}

/// Unlock attempts left before the device wipes its saved wallets.
#[must_use]
pub const fn unlock_attempts_remaining(failures: u8) -> u8 {
    MAX_UNLOCK_FAILURES.saturating_sub(failures)
}

/// True once the durable failure count forbids any further unlock attempt.
#[must_use]
pub const fn unlock_attempt_limit_reached(failures: u8) -> bool {
    failures >= MAX_UNLOCK_FAILURES
}

const CONFIRM_DOMAIN: &[u8] = b"KasSigner/persistent-credential-confirm/v1";

#[inline]
const fn confirmation_tag(kind: CredentialPolicyKind) -> u8 {
    match kind {
        CredentialPolicyKind::Pin => 1,
        CredentialPolicyKind::Password => 2,
    }
}

/// Non-secret one-session comparison digest used only to check that the user
/// typed the same credential twice before first save.
pub fn confirmation_digest(kind: CredentialPolicyKind, secret: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(CONFIRM_DOMAIN);
    hasher.update([confirmation_tag(kind)]);
    hasher.update((secret.len() as u16).to_le_bytes());
    hasher.update(secret);
    hasher.finalize().into()
}

#[must_use]
pub fn confirmation_matches(expected: &[u8; 32], actual: &[u8; 32]) -> bool {
    shared_signer::bytes::constant_time_eq_32(expected, actual)
}
