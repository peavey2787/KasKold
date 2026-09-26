//! Wallet source import and source-type accessors.
//!
//! Mnemonic, imported account-XPrv, and raw-private-key wallets remain distinct
//! custody types. This prevents source-specific capabilities from being faked by
//! converting one credential class into another.

use offline_signer::{
    derivation::{bip32, bip39::Seed},
    transaction::model::MultisigStore,
};
use zeroize::{Zeroize, Zeroizing};

use super::{HotWallet, HotWalletError, WalletKind};

impl HotWallet {
    /// Import one raw secp256k1 private key using the same exact 64-hex
    /// contract as the hardware advanced-recovery workflow.
    pub fn import_raw_private_key_hex(value: &str) -> Result<Self, HotWalletError> {
        let bytes = value.as_bytes();
        if bytes.len() != 64 {
            return Err(HotWalletError::RawKeyInvalid);
        }
        let mut key = [0u8; 32];
        for index in 0..32 {
            let Some(high) = shared_signer::bytes::decode_hex_nibble(bytes[index * 2]) else {
                key.zeroize();
                return Err(HotWalletError::RawKeyInvalid);
            };
            let Some(low) = shared_signer::bytes::decode_hex_nibble(bytes[index * 2 + 1]) else {
                key.zeroize();
                return Err(HotWalletError::RawKeyInvalid);
            };
            key[index] = (high << 4) | low;
        }
        if bip32::pubkey_from_raw_key(&key).is_err() {
            key.zeroize();
            return Err(HotWalletError::RawKeyInvalid);
        }
        Ok(Self {
            seed: Seed { bytes: [0u8; 64] },
            account_key: None,
            account_parent_fingerprint: [0u8; 4],
            raw_key: Some(Zeroizing::new(key)),
            recovery: None,
            multisig_store: MultisigStore::new(),
        })
    }

    /// Import an account-level Kaspa XPrv exactly as the M5 SD recovery path does.
    pub fn import_account_xprv(value: &str) -> Result<Self, HotWalletError> {
        let imported =
            offline_signer::derivation::xpub::import_xprv_with_metadata(value.as_bytes())?;
        Ok(Self {
            seed: Seed { bytes: [0u8; 64] },
            account_key: Some(imported.key),
            account_parent_fingerprint: imported.parent_fingerprint,
            raw_key: None,
            recovery: None,
            multisig_store: MultisigStore::new(),
        })
    }

    #[must_use]
    pub fn wallet_kind(&self) -> WalletKind {
        if self.raw_key.is_some() {
            WalletKind::RawPrivateKey
        } else if self.account_key.is_some() {
            WalletKind::AccountXprv
        } else {
            WalletKind::Mnemonic
        }
    }

    pub(crate) fn seed_bytes(&self) -> Result<&[u8; 64], HotWalletError> {
        if self.raw_key.is_some() || self.account_key.is_some() {
            Err(HotWalletError::UnsupportedForWalletType)
        } else {
            Ok(&self.seed.bytes)
        }
    }

    pub(crate) fn raw_key_bytes(&self) -> Option<&[u8; 32]> {
        self.raw_key.as_deref()
    }

    pub(crate) fn account_key(&self) -> Option<&bip32::ExtendedPrivKey> {
        self.account_key.as_ref()
    }

    pub(crate) fn account_parent_fingerprint(&self) -> [u8; 4] {
        self.account_parent_fingerprint
    }
}
