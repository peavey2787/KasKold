//! Portable steganographic backup for software Vaults.
//!
//! The JPEG carrier codec is shared with hardware. The embedded payload is the
//! authenticated Argon2id/KHV3 portable backup, so confidentiality and
//! authenticity do not depend on steganography.
//!
//! v2 hides that a photo carries a backup at all: the coefficient permutation
//! key is Argon2id of the password, so each guess costs a full host-profile
//! derivation, and the fixed-length payload is embedded with no length prefix
//! or marker. A photo cannot hold a per-image salt without marking itself, so
//! the salt is a fixed domain constant; the embedded backup still seals its
//! wallet under its own random salt. v1 photos (BLAKE2b permutation key,
//! length-prefixed) are read-only and stop restoring in the next release.

use offline_signer::crypto::password_kdf::{
    derive_key_32_with_params, PasswordKdfParams, PasswordKdfPurpose, SALT_SIZE,
};
use offline_signer::hash::blake2b_256;
use zeroize::{Zeroize, Zeroizing};

use super::{portable_backup::PORTABLE_BACKUP_LEN, HotWallet, HotWalletError};

const STEGO_V2_SALT: &[u8; SALT_SIZE] = b"KasKoldStego/v2\0";
const STEGO_V1_KEY_DOMAIN: &[u8] = b"KasKold/WebStego/permutation/v1\0";

pub(crate) fn v2_permutation_key(password: &str) -> Result<Zeroizing<[u8; 32]>, HotWalletError> {
    derive_key_32_with_params(
        PasswordKdfPurpose::PortableBackup,
        password.as_bytes(),
        STEGO_V2_SALT,
        PasswordKdfParams::host(),
    )
    .map(Zeroizing::new)
    .map_err(|_| HotWalletError::BackupCryptoFailed)
}

pub(crate) fn v1_permutation_key(password: &str) -> Zeroizing<[u8; 32]> {
    let mut key_material = Zeroizing::new(Vec::with_capacity(
        STEGO_V1_KEY_DOMAIN.len() + password.len(),
    ));
    key_material.extend_from_slice(STEGO_V1_KEY_DOMAIN);
    key_material.extend_from_slice(password.as_bytes());
    Zeroizing::new(blake2b_256(&key_material))
}

fn carrier_output_len(jpeg: &[u8]) -> Result<usize, HotWalletError> {
    jpeg.len()
        .checked_mul(2)
        .and_then(|value| value.checked_add(4_096))
        .ok_or(HotWalletError::BackupCryptoFailed)
}

impl HotWallet {
    pub fn stego_backup(&self, jpeg: &[u8], password: &str) -> Result<Vec<u8>, HotWalletError> {
        let mut payload = self.portable_backup(password)?;
        let embedded = Self::embed_v2_payload(jpeg, &payload, password);
        payload.zeroize();
        embedded
    }

    fn embed_v2_payload(
        jpeg: &[u8],
        payload: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, HotWalletError> {
        let key = v2_permutation_key(password)?;
        let mut encoded = vec![0u8; carrier_output_len(jpeg)?];
        let length =
            shared_signer::stego_picture::embed_exact(jpeg, payload, &key[..], &mut encoded)
                .map_err(|_| HotWalletError::BackupCryptoFailed)?;
        encoded.truncate(length);
        Ok(encoded)
    }

    pub fn restore_stego_backup(jpeg: &[u8], password: &str) -> Result<Self, HotWalletError> {
        Self::restore_v2_stego_backup(jpeg, password)
            .or_else(|_| Self::restore_v1_stego_backup(jpeg, password))
    }

    fn restore_v2_stego_backup(jpeg: &[u8], password: &str) -> Result<Self, HotWalletError> {
        let key = v2_permutation_key(password)?;
        let mut payload = Zeroizing::new(vec![0u8; PORTABLE_BACKUP_LEN]);
        shared_signer::stego_picture::extract_exact(jpeg, &key[..], &mut payload)
            .map_err(|_| HotWalletError::BackupCryptoFailed)?;
        Self::restore_portable_backup(&payload, password)
    }

    fn restore_v1_stego_backup(jpeg: &[u8], password: &str) -> Result<Self, HotWalletError> {
        let key = v1_permutation_key(password);
        let mut payload = Zeroizing::new(vec![0u8; PORTABLE_BACKUP_LEN]);
        let length = shared_signer::stego_picture::extract(jpeg, &key[..], &mut payload)
            .map_err(|_| HotWalletError::BackupCryptoFailed)?;
        if length != PORTABLE_BACKUP_LEN {
            return Err(HotWalletError::BackupCryptoFailed);
        }
        Self::restore_portable_backup(&payload, password)
    }
}
