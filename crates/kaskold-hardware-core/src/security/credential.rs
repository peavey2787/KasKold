//! Device-facing PIN/password acceptance, confirmation, and retry policy.
//!
//! Encrypted-container metadata such as the persisted credential discriminator
//! and salt width lives in `offline-signer::crypto::credential`; this module
//! deliberately owns only physical-device policy around those credentials.

use sha2::{Digest, Sha256};

pub const MIN_PIN_DIGITS: usize = 6;
pub const MAX_PIN_DIGITS: usize = 12;
/// Minimum length of a newly chosen password.
pub const MIN_PASSWORD_CHARS: usize = 12;
pub const MAX_PASSWORD_CHARS: usize = 128;
pub const RETRY_BASE_MILLIS: u32 = 1_000;
pub const RETRY_MAX_MILLIS: u32 = 60_000;
/// Consecutive failed unlock attempts, counted durably across reboots, after
/// which the device erases every saved wallet instead of accepting more guesses.
pub const MAX_UNLOCK_FAILURES: u8 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialPolicyKind {
    Pin = 1,
    Password = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialError {
    PinTooShort,
    PinTooLong,
    PinNotNumeric,
    PasswordTooShort,
    PasswordTooLong,
    PasswordNeedsLetter,
    PasswordNeedsDigit,
}

pub fn validate(kind: CredentialPolicyKind, secret: &[u8]) -> Result<(), CredentialError> {
    match kind {
        CredentialPolicyKind::Pin => validate_pin(secret),
        CredentialPolicyKind::Password => validate_password(secret),
    }
}

fn validate_pin(secret: &[u8]) -> Result<(), CredentialError> {
    if !secret.iter().all(|byte| byte.is_ascii_digit()) {
        return Err(CredentialError::PinNotNumeric);
    }
    if secret.len() < MIN_PIN_DIGITS {
        return Err(CredentialError::PinTooShort);
    }
    if secret.len() > MAX_PIN_DIGITS {
        return Err(CredentialError::PinTooLong);
    }
    Ok(())
}

fn validate_password(secret: &[u8]) -> Result<(), CredentialError> {
    if secret.len() < MIN_PASSWORD_CHARS {
        return Err(CredentialError::PasswordTooShort);
    }
    if secret.len() > MAX_PASSWORD_CHARS {
        return Err(CredentialError::PasswordTooLong);
    }
    if !secret.iter().any(|byte| byte.is_ascii_alphabetic()) {
        return Err(CredentialError::PasswordNeedsLetter);
    }
    if !secret.iter().any(|byte| byte.is_ascii_digit()) {
        return Err(CredentialError::PasswordNeedsDigit);
    }
    Ok(())
}

/// A password typed to open existing ciphertext. Backups sealed before the
/// current minimum must stay openable, so only the KDF's length bounds apply;
/// the authenticated decryption decides whether the password is right.
pub fn validate_entered_password(secret: &[u8]) -> Result<(), CredentialError> {
    if secret.is_empty() {
        return Err(CredentialError::PasswordTooShort);
    }
    if secret.len() > MAX_PASSWORD_CHARS {
        return Err(CredentialError::PasswordTooLong);
    }
    Ok(())
}

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
