//! Portable authenticated wallet backup for software Vaults.
//!
//! Hardware SD backups bind encryption to a non-exportable device secret.
//! Browser/mobile software cannot honestly reproduce that property, so this
//! format is explicitly portable: Argon2id derives a wrapping key from the
//! user's password and the authenticated versioned KHV3 wallet seal protects
//! the recovery material.

use offline_signer::crypto::password_kdf::{
    derive_key_32, derive_key_32_with_params, encode_metadata, parse_metadata, PasswordKdfParams,
    PasswordKdfPurpose, METADATA_SIZE, SALT_SIZE,
};
use zeroize::Zeroize;

use crate::entropy::fill_random;

use super::{HotWallet, HotWalletError, WalletKind, PLATFORM_WRAPPING_KEY_LEN, SEALED_WALLET_LEN};

const PORTABLE_MAGIC: &[u8; 4] = b"KWP1";
const HEADER_LEN: usize = 4 + METADATA_SIZE + SALT_SIZE;
pub const PORTABLE_BACKUP_LEN: usize = HEADER_LEN + SEALED_WALLET_LEN;

impl HotWallet {
    /// Create a portable encrypted backup containing only the account XPrv.
    ///
    /// This mirrors the M5 `XPrv Backup -> Encrypt to SD` source semantics: a
    /// restored backup becomes an account-XPrv wallet and does not gain a fake
    /// mnemonic/recovery phrase.
    pub fn portable_xprv_backup(&self, password: &str) -> Result<Vec<u8>, HotWalletError> {
        let xprv = self.backup_account_xprv()?;
        let wallet = Self::import_account_xprv(&xprv)?;
        wallet.portable_backup_source(password)
    }

    /// Create the software equivalent of the M5 encrypted Seed backup.
    ///
    /// The M5 route is mnemonic-seed only. Imported account XPrv and raw-key
    /// wallets must use their source-specific export paths instead of silently
    /// widening the meaning of this backup.
    pub fn portable_backup(&self, password: &str) -> Result<Vec<u8>, HotWalletError> {
        if self.wallet_kind() != WalletKind::Mnemonic {
            return Err(HotWalletError::UnsupportedForWalletType);
        }
        self.portable_backup_source(password)
    }

    fn portable_backup_source(&self, password: &str) -> Result<Vec<u8>, HotWalletError> {
        let metadata = encode_metadata(PasswordKdfParams::current())
            .map_err(|_| HotWalletError::BackupCryptoFailed)?;
        let mut salt = [0u8; SALT_SIZE];
        fill_random(&mut salt)?;
        let mut key = derive_key_32(
            PasswordKdfPurpose::PortableBackup,
            password.as_bytes(),
            &salt,
        )
        .map_err(|_| HotWalletError::BackupCryptoFailed)?;
        let sealed_result = self.seal_for_platform(&key);
        key.zeroize();
        let sealed = sealed_result?;
        let mut output = Vec::with_capacity(PORTABLE_BACKUP_LEN);
        output.extend_from_slice(PORTABLE_MAGIC);
        output.extend_from_slice(&metadata);
        output.extend_from_slice(&salt);
        output.extend_from_slice(&sealed);
        salt.zeroize();
        if output.len() != PORTABLE_BACKUP_LEN {
            output.zeroize();
            return Err(HotWalletError::BackupCryptoFailed);
        }
        Ok(output)
    }

    pub fn restore_portable_backup(data: &[u8], password: &str) -> Result<Self, HotWalletError> {
        if data.len() != HEADER_LEN + SEALED_WALLET_LEN || &data[..4] != PORTABLE_MAGIC {
            return Err(HotWalletError::InvalidSealedWallet);
        }
        let parameters = parse_metadata(&data[4..4 + METADATA_SIZE])
            .map_err(|_| HotWalletError::BackupCryptoFailed)?;
        let mut salt = [0u8; SALT_SIZE];
        salt.copy_from_slice(&data[4 + METADATA_SIZE..HEADER_LEN]);
        let mut key = derive_key_32_with_params(
            PasswordKdfPurpose::PortableBackup,
            password.as_bytes(),
            &salt,
            parameters,
        )
        .map_err(|_| HotWalletError::BackupCryptoFailed)?;
        salt.zeroize();
        let result = Self::restore_platform_sealed(&data[HEADER_LEN..], &key);
        key.zeroize();
        result
    }
}

const _: [(); PLATFORM_WRAPPING_KEY_LEN] = [(); 32];
