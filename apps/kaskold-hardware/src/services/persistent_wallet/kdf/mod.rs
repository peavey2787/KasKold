//! Argon2id password KDF for persistent-wallet credentials.
//!
//! KasKold has no pre-production PBKDF2 compatibility reader. Persisted wallet
//! records authenticate explicit Argon2id parameters and unsupported formats fail closed.

use offline_signer::crypto::password_kdf::{self, PasswordKdfParams, PasswordKdfPurpose};
use crate::services::credential_policy::SALT_SIZE;

use super::PersistError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CredentialKdf(PasswordKdfParams);

impl CredentialKdf {
    pub const fn current() -> Self {
        Self(PasswordKdfParams::current())
    }

    pub const fn from_parameters(parameters: PasswordKdfParams) -> Self {
        Self(parameters)
    }

    pub const fn parameters(self) -> PasswordKdfParams {
        self.0
    }
}

pub(super) fn derive(
    kdf: CredentialKdf,
    purpose: PasswordKdfPurpose,
    secret: &[u8],
    salt: &[u8; SALT_SIZE],
    liveness: &mut (impl FnMut() + ?Sized),
) -> Result<[u8; 32], PersistError> {
    liveness();
    let result = crate::services::memory::password_kdf::derive_key_32_with_params(
        purpose,
        secret,
        salt,
        kdf.parameters(),
    )
    .map_err(map_argon_error);
    liveness();
    result
}

fn map_argon_error(error: password_kdf::PasswordKdfError) -> PersistError {
    match error {
        password_kdf::PasswordKdfError::InvalidPasswordLength => PersistError::InvalidWallet,
        password_kdf::PasswordKdfError::UnsupportedParameters => PersistError::InvalidWallet,
        password_kdf::PasswordKdfError::AllocationFailed => PersistError::Crypto,
        password_kdf::PasswordKdfError::DerivationFailed => PersistError::Crypto,
    }
}
