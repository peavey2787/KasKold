//! Which PINs and passwords may be chosen or entered. Shared by the device
//! and the software vault so both enforce one policy.

pub const MIN_PIN_DIGITS: usize = 6;
pub const MAX_PIN_DIGITS: usize = 12;
/// Minimum length of a newly chosen password.
pub const MIN_PASSWORD_CHARS: usize = 12;
pub const MAX_PASSWORD_CHARS: usize = 128;

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
