//! Stealth-scan custody adapter.
//!
//! Protocol math lives in `offline-signer::stealth`; this module only selects
//! the active account key from the typed software-wallet source.

use offline_signer::derivation::bip32;

use crate::{HotWallet, HotWalletError};

impl HotWallet {
    pub fn stealth_scan_request(&self, request: &[u8]) -> Result<Vec<u8>, HotWalletError> {
        if self.raw_key_bytes().is_some() {
            return Err(HotWalletError::UnsupportedForWalletType);
        }
        if let Some(account) = self.account_key() {
            return offline_signer::stealth::scan_request(account, request)
                .map_err(|_| HotWalletError::CryptoOperationFailed);
        }
        let account = bip32::derive_account_key(self.seed_bytes()?)?;
        offline_signer::stealth::scan_request(&account, request)
            .map_err(|_| HotWalletError::CryptoOperationFailed)
    }
}
