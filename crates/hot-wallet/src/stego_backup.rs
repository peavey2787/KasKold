//! Portable steganographic backup for software Vaults.
//!
//! The JPEG carrier codec is shared with hardware. The embedded payload is the
//! authenticated Argon2id/KHV3 portable backup, so confidentiality and
//! authenticity do not depend on steganography.

use offline_signer::transaction::sighash::blake2b_hash;
use zeroize::{Zeroize, Zeroizing};

use super::{portable_backup::PORTABLE_BACKUP_LEN, HotWallet, HotWalletError};

const STEGO_KEY_DOMAIN: &[u8] = b"KasKold/WebStego/permutation/v1\0";

impl HotWallet {
    pub fn stego_backup(&self, jpeg: &[u8], password: &str) -> Result<Vec<u8>, HotWalletError> {
        let mut payload = self.portable_backup(password)?;
        let mut key_material = Vec::with_capacity(STEGO_KEY_DOMAIN.len() + password.len());
        key_material.extend_from_slice(STEGO_KEY_DOMAIN);
        key_material.extend_from_slice(password.as_bytes());
        let mut key = blake2b_hash(&key_material);
        key_material.zeroize();

        let capacity = jpeg
            .len()
            .checked_mul(2)
            .and_then(|value| value.checked_add(4_096))
            .ok_or(HotWalletError::BackupCryptoFailed)?;
        let mut encoded = vec![0u8; capacity];
        let embedded = shared_signer::stego_picture::embed(jpeg, &payload, &key, &mut encoded)
            .map_err(|_| HotWalletError::BackupCryptoFailed);
        payload.zeroize();
        key.zeroize();
        let length = embedded?;
        encoded.truncate(length);
        Ok(encoded)
    }

    pub fn restore_stego_backup(jpeg: &[u8], password: &str) -> Result<Self, HotWalletError> {
        let mut key_material = Vec::with_capacity(STEGO_KEY_DOMAIN.len() + password.len());
        key_material.extend_from_slice(STEGO_KEY_DOMAIN);
        key_material.extend_from_slice(password.as_bytes());
        let mut key = blake2b_hash(&key_material);
        key_material.zeroize();
        let mut payload = Zeroizing::new(vec![0u8; PORTABLE_BACKUP_LEN]);
        let extracted = shared_signer::stego_picture::extract(jpeg, &key, &mut payload)
            .map_err(|_| HotWalletError::BackupCryptoFailed);
        key.zeroize();
        let length = extracted?;
        if length != PORTABLE_BACKUP_LEN {
            return Err(HotWalletError::BackupCryptoFailed);
        }
        Self::restore_portable_backup(&payload[..length], password)
    }
}
