//! Privacy-pairing custody adapter.

use offline_signer::derivation::bip32;

use crate::{HotWallet, HotWalletError};

impl HotWallet {
    pub fn privacy_pairing_response(&self, request: &[u8]) -> Result<Vec<u8>, HotWalletError> {
        if self.raw_key_bytes().is_some() {
            return Err(HotWalletError::UnsupportedForWalletType);
        }
        if let Some(account) = self.account_key() {
            return offline_signer::privacy_pairing::respond(account, request)
                .map_err(|_| HotWalletError::CryptoOperationFailed);
        }
        let account = bip32::derive_account_key(self.seed_bytes()?)?;
        offline_signer::privacy_pairing::respond(&account, request)
            .map_err(|_| HotWalletError::CryptoOperationFailed)
    }
}
